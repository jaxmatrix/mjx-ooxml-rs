# DrawingML preset-shape geometry — porting ledger & methodology

> Status of porting the DrawingML preset shapes into `mjx-dml` typed geometry, and the
> **methodology** for turning each shape’s raw `adj` control points into self-explanatory
> **named control parameters**. This document is the source of truth for that workstream; the code
> lands after the DrawingML text model, foundation-first then names-in-batches.

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

1. **Foundation** — `PresetShapeType` enum (187) + `prstGeom` fidelity model (round-trips any
   shape) + this ledger scaffold. Reuses the `FromXml`/`ToXml` traits from the text-model PR. After
   it, every shape is at least `fidelity`.
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
| `actionButtonBackPrevious` | Action button | 0 | — | pending | — |
| `actionButtonBeginning` | Action button | 0 | — | pending | — |
| `actionButtonBlank` | Action button | 0 | — | pending | — |
| `actionButtonDocument` | Action button | 0 | — | pending | — |
| `actionButtonEnd` | Action button | 0 | — | pending | — |
| `actionButtonForwardNext` | Action button | 0 | — | pending | — |
| `actionButtonHelp` | Action button | 0 | — | pending | — |
| `actionButtonHome` | Action button | 0 | — | pending | — |
| `actionButtonInformation` | Action button | 0 | — | pending | — |
| `actionButtonMovie` | Action button | 0 | — | pending | — |
| `actionButtonReturn` | Action button | 0 | — | pending | — |
| `actionButtonSound` | Action button | 0 | — | pending | — |
| `bentConnector2` | Connector | 0 | — | pending | — |
| `chartPlus` | Basic / geometric | 0 | — | pending | — |
| `chartStar` | Basic / geometric | 0 | — | pending | — |
| `chartX` | Basic / geometric | 0 | — | pending | — |
| `cloud` | Basic / geometric | 0 | — | pending | — |
| `cornerTabs` | Basic / geometric | 0 | — | pending | — |
| `curvedConnector2` | Connector | 0 | — | pending | — |
| `diamond` | Basic / geometric | 0 | — | pending | — |
| `dodecagon` | Basic / geometric | 0 | — | pending | — |
| `ellipse` | Basic / geometric | 0 | — | pending | — |
| `flowChartAlternateProcess` | Flowchart | 0 | — | pending | — |
| `flowChartCollate` | Flowchart | 0 | — | pending | — |
| `flowChartConnector` | Flowchart | 0 | — | pending | — |
| `flowChartDecision` | Flowchart | 0 | — | pending | — |
| `flowChartDelay` | Flowchart | 0 | — | pending | — |
| `flowChartDisplay` | Flowchart | 0 | — | pending | — |
| `flowChartDocument` | Flowchart | 0 | — | pending | — |
| `flowChartExtract` | Flowchart | 0 | — | pending | — |
| `flowChartInputOutput` | Flowchart | 0 | — | pending | — |
| `flowChartInternalStorage` | Flowchart | 0 | — | pending | — |
| `flowChartMagneticDisk` | Flowchart | 0 | — | pending | — |
| `flowChartMagneticDrum` | Flowchart | 0 | — | pending | — |
| `flowChartMagneticTape` | Flowchart | 0 | — | pending | — |
| `flowChartManualInput` | Flowchart | 0 | — | pending | — |
| `flowChartManualOperation` | Flowchart | 0 | — | pending | — |
| `flowChartMerge` | Flowchart | 0 | — | pending | — |
| `flowChartMultidocument` | Flowchart | 0 | — | pending | — |
| `flowChartOfflineStorage` | Flowchart | 0 | — | pending | — |
| `flowChartOffpageConnector` | Flowchart | 0 | — | pending | — |
| `flowChartOnlineStorage` | Flowchart | 0 | — | pending | — |
| `flowChartOr` | Flowchart | 0 | — | pending | — |
| `flowChartPredefinedProcess` | Flowchart | 0 | — | pending | — |
| `flowChartPreparation` | Flowchart | 0 | — | pending | — |
| `flowChartProcess` | Flowchart | 0 | — | pending | — |
| `flowChartPunchedCard` | Flowchart | 0 | — | pending | — |
| `flowChartPunchedTape` | Flowchart | 0 | — | pending | — |
| `flowChartSort` | Flowchart | 0 | — | pending | — |
| `flowChartSummingJunction` | Flowchart | 0 | — | pending | — |
| `flowChartTerminator` | Flowchart | 0 | — | pending | — |
| `funnel` | Basic / geometric | 0 | — | pending | — |
| `heart` | Basic / geometric | 0 | — | pending | — |
| `irregularSeal1` | Star / seal | 0 | — | pending | — |
| `irregularSeal2` | Star / seal | 0 | — | pending | — |
| `lightningBolt` | Basic / geometric | 0 | — | pending | — |
| `line` | Basic / geometric | 0 | — | pending | — |
| `lineInv` | Basic / geometric | 0 | — | pending | — |
| `pieWedge` | Basic / geometric | 0 | — | pending | — |
| `plaqueTabs` | Basic / geometric | 0 | — | pending | — |
| `rect` | Basic / geometric | 0 | — | pending | — |
| `rtTriangle` | Basic / geometric | 0 | — | pending | — |
| `squareTabs` | Basic / geometric | 0 | — | pending | — |
| `straightConnector1` | Connector | 0 | — | pending | — |

### Batch 3 — single adjustment — 41 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `bentConnector3` | Connector | 1 | 1×ahXY | pending | — |
| `bevel` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `bracePair` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `bracketPair` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `can` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `chevron` | Arrow / ribbon | 1 | 1×ahXY | pending | — |
| `cube` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `curvedConnector3` | Connector | 1 | 1×ahXY | pending | — |
| `decagon` | Basic / geometric | 1 | — | pending | — |
| `diagStripe` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `donut` | Basic / geometric | 1 | 1×ahPolar | pending | — |
| `foldedCorner` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `frame` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `homePlate` | Arrow / ribbon | 1 | 1×ahXY | pending | — |
| `horizontalScroll` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `leftBracket` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `mathMinus` | Math | 1 | 1×ahXY | pending | — |
| `mathMultiply` | Math | 1 | 1×ahXY | pending | — |
| `mathPlus` | Math | 1 | 1×ahXY | pending | — |
| `moon` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `noSmoking` | Basic / geometric | 1 | 1×ahPolar | pending | — |
| `octagon` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `parallelogram` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `plaque` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `plus` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `rightBracket` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `round1Rect` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `roundRect` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `smileyFace` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `snip1Rect` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `star12` | Star / seal | 1 | 1×ahXY | pending | — |
| `star16` | Star / seal | 1 | 1×ahXY | pending | — |
| `star24` | Star / seal | 1 | 1×ahXY | pending | — |
| `star32` | Star / seal | 1 | 1×ahXY | pending | — |
| `star4` | Star / seal | 1 | 1×ahXY | pending | — |
| `star8` | Star / seal | 1 | 1×ahXY | pending | — |
| `sun` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `teardrop` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `trapezoid` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `triangle` | Basic / geometric | 1 | 1×ahXY | pending | — |
| `verticalScroll` | Basic / geometric | 1 | 1×ahXY | pending | — |

### Batch 4 — two adjustments — 38 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `arc` | Basic / geometric | 2 | 2×ahPolar | pending | — |
| `bentConnector4` | Connector | 2 | 2×ahXY | pending | — |
| `chord` | Basic / geometric | 2 | 2×ahPolar | pending | — |
| `cloudCallout` | Callout | 2 | 1×ahXY | pending | — |
| `corner` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `curvedConnector4` | Connector | 2 | 2×ahXY | pending | — |
| `doubleWave` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `downArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `gear6` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `gear9` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `halfFrame` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `heptagon` | Basic / geometric | 2 | — | pending | — |
| `hexagon` | Basic / geometric | 2 | 1×ahXY | pending | — |
| `leftArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `leftBrace` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `leftRightArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `mathEqual` | Math | 2 | 2×ahXY | pending | — |
| `nonIsoscelesTrapezoid` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `notchedRightArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `pentagon` | Basic / geometric | 2 | — | pending | — |
| `pie` | Basic / geometric | 2 | 2×ahPolar | pending | — |
| `ribbon` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `ribbon2` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `rightArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `rightBrace` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `round2DiagRect` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `round2SameRect` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `snip2DiagRect` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `snip2SameRect` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `snipRoundRect` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `star10` | Star / seal | 2 | 1×ahXY | pending | — |
| `star6` | Star / seal | 2 | 1×ahXY | pending | — |
| `stripedRightArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `swooshArrow` | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `upDownArrow` _(dup in spec)_ | Arrow / ribbon | 2 | 2×ahXY | pending | — |
| `wave` | Basic / geometric | 2 | 2×ahXY | pending | — |
| `wedgeEllipseCallout` | Callout | 2 | 1×ahXY | pending | — |
| `wedgeRectCallout` | Callout | 2 | 1×ahXY | pending | — |

### Batch 5 — complex (3–8 interdependent adjustments) — 43 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `accentBorderCallout1` | Callout | 4 | 2×ahXY | pending | — |
| `accentBorderCallout2` | Callout | 6 | 3×ahXY | pending | — |
| `accentBorderCallout3` | Callout | 8 | 4×ahXY | pending | — |
| `accentCallout1` | Callout | 4 | 2×ahXY | pending | — |
| `accentCallout2` | Callout | 6 | 3×ahXY | pending | — |
| `accentCallout3` | Callout | 8 | 4×ahXY | pending | — |
| `bentArrow` | Arrow / ribbon | 4 | 4×ahXY | pending | — |
| `bentConnector5` | Connector | 3 | 3×ahXY | pending | — |
| `bentUpArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `blockArc` | Basic / geometric | 3 | 2×ahPolar | pending | — |
| `borderCallout1` | Callout | 4 | 2×ahXY | pending | — |
| `borderCallout2` | Callout | 6 | 3×ahXY | pending | — |
| `borderCallout3` | Callout | 8 | 4×ahXY | pending | — |
| `callout1` | Basic / geometric | 4 | 2×ahXY | pending | — |
| `callout2` | Basic / geometric | 6 | 3×ahXY | pending | — |
| `callout3` | Basic / geometric | 8 | 4×ahXY | pending | — |
| `circularArrow` | Arrow / ribbon | 5 | 4×ahPolar | pending | — |
| `curvedConnector5` | Connector | 3 | 3×ahXY | pending | — |
| `curvedDownArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `curvedLeftArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `curvedRightArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `curvedUpArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `downArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `ellipseRibbon` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `ellipseRibbon2` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `leftArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `leftCircularArrow` | Arrow / ribbon | 5 | 4×ahPolar | pending | — |
| `leftRightArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `leftRightCircularArrow` | Arrow / ribbon | 5 | 4×ahPolar | pending | — |
| `leftRightRibbon` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `leftRightUpArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `leftUpArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `mathDivide` | Math | 3 | 3×ahXY | pending | — |
| `mathNotEqual` | Math | 3 | 2×ahXY, 1×ahPolar | pending | — |
| `quadArrow` | Arrow / ribbon | 3 | 3×ahXY | pending | — |
| `quadArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `rightArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `star5` | Star / seal | 3 | 1×ahXY | pending | — |
| `star7` | Star / seal | 3 | 1×ahXY | pending | — |
| `upArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `upDownArrowCallout` | Callout | 4 | 4×ahXY | pending | — |
| `uturnArrow` | Arrow / ribbon | 5 | 5×ahXY | pending | — |
| `wedgeRoundRectCallout` | Callout | 3 | 1×ahXY | pending | — |

---

_Missing from the spec file but standardised in `ST_ShapeType`: `upArrow` (author on port)._
