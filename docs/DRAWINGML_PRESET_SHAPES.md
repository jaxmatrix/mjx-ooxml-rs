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

## Batch plan (each a reviewable, always-green PR that updates this ledger)

1. **Foundation — ✅ done.** `PresetShapeType` enum (187, generated in `mjx-ooxml-types::drawingml`)
   + the `prstGeom`/`avLst`/`gd` fidelity model in `mjx-dml::geometry` (`PresetGeometry` /
   `GeometryGuideList` / `GeometryGuide`, round-trips any shape, plus a minimal typed builder) +
   this ledger scaffold. Reuses the `FromXml`/`ToXml` traits from the text-model PR. **Every shape
   below is now at least `fidelity`.**
2. **Fixed geometry** — 64 shapes with 0 adjustments → parameterless.
3. **Single adjustment** — 41 shapes (mostly clean `*/ ss adj 100000` fractions).
4. **Two adjustments** — 38 shapes.
5. **Complex** — 43 shapes (3–8 interdependent adjusts), hand-curated names in sub-batches.

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
| `bentConnector3` | Connector | 1 | 1×ahXY | fidelity | — |
| `bevel` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `bracePair` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `bracketPair` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `can` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `chevron` | Arrow / ribbon | 1 | 1×ahXY | fidelity | — |
| `cube` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `curvedConnector3` | Connector | 1 | 1×ahXY | fidelity | — |
| `decagon` | Basic / geometric | 1 | — | fidelity | — |
| `diagStripe` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `donut` | Basic / geometric | 1 | 1×ahPolar | fidelity | — |
| `foldedCorner` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `frame` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `homePlate` | Arrow / ribbon | 1 | 1×ahXY | fidelity | — |
| `horizontalScroll` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `leftBracket` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `mathMinus` | Math | 1 | 1×ahXY | fidelity | — |
| `mathMultiply` | Math | 1 | 1×ahXY | fidelity | — |
| `mathPlus` | Math | 1 | 1×ahXY | fidelity | — |
| `moon` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `noSmoking` | Basic / geometric | 1 | 1×ahPolar | fidelity | — |
| `octagon` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `parallelogram` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `plaque` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `plus` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `rightBracket` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `round1Rect` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `roundRect` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `smileyFace` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `snip1Rect` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `star12` | Star / seal | 1 | 1×ahXY | fidelity | — |
| `star16` | Star / seal | 1 | 1×ahXY | fidelity | — |
| `star24` | Star / seal | 1 | 1×ahXY | fidelity | — |
| `star32` | Star / seal | 1 | 1×ahXY | fidelity | — |
| `star4` | Star / seal | 1 | 1×ahXY | fidelity | — |
| `star8` | Star / seal | 1 | 1×ahXY | fidelity | — |
| `sun` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `teardrop` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `trapezoid` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `triangle` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `verticalScroll` | Basic / geometric | 1 | 1×ahXY | fidelity | — |

### Batch 4 — two adjustments — 38 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `arc` | Basic / geometric | 2 | 2×ahPolar | fidelity | — |
| `bentConnector4` | Connector | 2 | 2×ahXY | fidelity | — |
| `chord` | Basic / geometric | 2 | 2×ahPolar | fidelity | — |
| `cloudCallout` | Callout | 2 | 1×ahXY | fidelity | — |
| `corner` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `curvedConnector4` | Connector | 2 | 2×ahXY | fidelity | — |
| `doubleWave` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `downArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `gear6` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `gear9` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `halfFrame` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `heptagon` | Basic / geometric | 2 | — | fidelity | — |
| `hexagon` | Basic / geometric | 2 | 1×ahXY | fidelity | — |
| `leftArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `leftBrace` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `leftRightArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `mathEqual` | Math | 2 | 2×ahXY | fidelity | — |
| `nonIsoscelesTrapezoid` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `notchedRightArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `pentagon` | Basic / geometric | 2 | — | fidelity | — |
| `pie` | Basic / geometric | 2 | 2×ahPolar | fidelity | — |
| `ribbon` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `ribbon2` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `rightArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `rightBrace` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `round2DiagRect` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `round2SameRect` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `snip2DiagRect` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `snip2SameRect` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `snipRoundRect` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `star10` | Star / seal | 2 | 1×ahXY | fidelity | — |
| `star6` | Star / seal | 2 | 1×ahXY | fidelity | — |
| `stripedRightArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `swooshArrow` | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `upDownArrow` _(dup in spec)_ | Arrow / ribbon | 2 | 2×ahXY | fidelity | — |
| `wave` | Basic / geometric | 2 | 2×ahXY | fidelity | — |
| `wedgeEllipseCallout` | Callout | 2 | 1×ahXY | fidelity | — |
| `wedgeRectCallout` | Callout | 2 | 1×ahXY | fidelity | — |

### Batch 5 — complex (3–8 interdependent adjustments) — 43 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `accentBorderCallout1` | Callout | 4 | 2×ahXY | fidelity | — |
| `accentBorderCallout2` | Callout | 6 | 3×ahXY | fidelity | — |
| `accentBorderCallout3` | Callout | 8 | 4×ahXY | fidelity | — |
| `accentCallout1` | Callout | 4 | 2×ahXY | fidelity | — |
| `accentCallout2` | Callout | 6 | 3×ahXY | fidelity | — |
| `accentCallout3` | Callout | 8 | 4×ahXY | fidelity | — |
| `bentArrow` | Arrow / ribbon | 4 | 4×ahXY | fidelity | — |
| `bentConnector5` | Connector | 3 | 3×ahXY | fidelity | — |
| `bentUpArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `blockArc` | Basic / geometric | 3 | 2×ahPolar | fidelity | — |
| `borderCallout1` | Callout | 4 | 2×ahXY | fidelity | — |
| `borderCallout2` | Callout | 6 | 3×ahXY | fidelity | — |
| `borderCallout3` | Callout | 8 | 4×ahXY | fidelity | — |
| `callout1` | Basic / geometric | 4 | 2×ahXY | fidelity | — |
| `callout2` | Basic / geometric | 6 | 3×ahXY | fidelity | — |
| `callout3` | Basic / geometric | 8 | 4×ahXY | fidelity | — |
| `circularArrow` | Arrow / ribbon | 5 | 4×ahPolar | fidelity | — |
| `curvedConnector5` | Connector | 3 | 3×ahXY | fidelity | — |
| `curvedDownArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `curvedLeftArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `curvedRightArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `curvedUpArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `downArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `ellipseRibbon` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `ellipseRibbon2` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `leftArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `leftCircularArrow` | Arrow / ribbon | 5 | 4×ahPolar | fidelity | — |
| `leftRightArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `leftRightCircularArrow` | Arrow / ribbon | 5 | 4×ahPolar | fidelity | — |
| `leftRightRibbon` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `leftRightUpArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `leftUpArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `mathDivide` | Math | 3 | 3×ahXY | fidelity | — |
| `mathNotEqual` | Math | 3 | 2×ahXY, 1×ahPolar | fidelity | — |
| `quadArrow` | Arrow / ribbon | 3 | 3×ahXY | fidelity | — |
| `quadArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `rightArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `star5` | Star / seal | 3 | 1×ahXY | fidelity | — |
| `star7` | Star / seal | 3 | 1×ahXY | fidelity | — |
| `upArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `upDownArrowCallout` | Callout | 4 | 4×ahXY | fidelity | — |
| `uturnArrow` | Arrow / ribbon | 5 | 5×ahXY | fidelity | — |
| `wedgeRoundRectCallout` | Callout | 3 | 1×ahXY | fidelity | — |

---

_Missing from the spec file but standardised in `ST_ShapeType`: `upArrow` (author on port)._
