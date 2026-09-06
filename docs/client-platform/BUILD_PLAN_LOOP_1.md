# Loop 1 — the renderer, and a catalogue you can audit

> **What this loop builds:** the rendering engine for PowerPoint, Word and Excel, and every UI element
> presented in isolation in Storybook for design audit at both mobile and desktop sizes.
>
> **What this loop explicitly does not build:** the application itself. No shell wiring, no command
> dispatch, no document-to-chrome integration. That is loop 2, and it starts only after the catalogue
> has been audited and signed off.

The separation is deliberate and it is the right one: a component you have approved in isolation is a
fixed point, and integrating against fixed points is a different and far cheaper activity than
designing and integrating at the same time.

---

## 1 · The gap this plan has to close first

**A large and visually critical class of UI cannot be a Storybook web component, because it is drawn
by the Rust renderer inside the canvas.** By decision, anything that must track document geometry at
60 fps lives there:

selection handles · rotation affordance · resize ghosts · alignment and snap guides · marching ants ·
the text caret and its bidi split form · selection fills · cell-range handles and the fill handle ·
drag previews and drop indicators · rulers and their margin/indent markers · gridlines and guides ·
comment anchors · tracked-change marks · the page shadow and canvas backdrop · autofit and overflow
indicators · touch handles at their larger hit sizes

These are the *most* design-sensitive elements in an editor and exactly the ones the audit needs to
cover. Left as-is, "audit every UI element" would silently miss all of them.

**Resolution: Storybook is the audit surface for both, with two kinds of story.**

| Story kind | Source | Covers |
|---|---|---|
| Live web component | TypeScript custom element | ribbon, panels, menus, dialogs, all chrome |
| **Generated image plate** | Rust `tiny-skia` painter → PNG + manifest | every in-canvas UI element, in every state, theme and DPI |

The Rust side gains a `catalogue` binary that renders each in-canvas element across its state matrix
(default / hover / active / focused / disabled × light / dark × 1× / 2× / touch) into
`ui/catalogue/` with a JSON manifest; a Storybook loader turns the manifest into stories. The plates
sit beside the live components in the same navigation, so an audit pass covers the whole surface in
one place.

This costs almost nothing extra, because **the plate generator is the fidelity oracle's harness**.
The same code that lets you look at a selection handle is the code that snapshots it for regression.
Building it early serves both tracks.

The same mechanism carries the renderer itself: a **document plate gallery** — each fixture rendered
per format, beside its LibreOffice reference and a perceptual diff. That is how "does it render
properly" becomes something you can *look at* rather than something you take on trust.

---

## 2 · Two tracks, one at a time

Track A is Rust, Track B is TypeScript, and they share only the token source. They are an *ordering*,
not a concurrency: per the project's standing rule, exactly one implementation agent runs at a time
and the orchestrator reviews each child against its gate before the next starts. Interleaving the
tracks between children is fine and useful — it keeps the token pipeline honest in both directions.

### Track A — the rendering engine

| # | Unit | Gate |
|---|---|---|
| A1 | `mjx-tokens` + the codegen emitting CSS, TS and Rust from one source | all three artefacts generated and committed; Rust and CSS agree on every value |
| A2 | `mjx-text` — font DB, substitution table, shaping, bidi, itemisation, line breaking, glyph atlas | a paragraph of mixed Latin/Arabic/CJK shapes with correct advances; metric-compatible substitution proven against reference metrics |
| A3 | `mjx-layout` — `BoxModel`, `FragmentTree`, `Checkpoint`, spatial index | contract compiles with a trivial second implementation, proving it is not OOXML-shaped |
| A4 | `mjx-scene` — display list, `lyon` tessellation, `GeometryProvider` **returning the placeholder shape** | a scene round-trips through the binary encoding; tessellation is deterministic across platforms |
| A5 | `mjx-paint` — `wgpu` and `tiny-skia` painters, PDF and SVG exporters | the same display list renders identically on both painters within tolerance |
| A6 | The fidelity oracle + the plate generator | golden images run headless in CI; the in-canvas catalogue generates |
| A7 | `mjx-session` — resident document, edit journal, invalidation | a mutation dirties exactly the pages it should, and no more |
| A8 | `mjx-layout-pptx` + `mjx-view` — slide layout, text bodies, autofit, tables, groups, effects, images | a real deck renders at parity within frame and memory budget |
| A9 | `mjx-layout-xlsx` — grid, **number-format engine**, conditional formatting, panes, drawings | a real workbook renders at parity; the format engine passes its own conformance suite |
| A10 | `mjx-layout-docx` — reflow, pagination, tables, floats and wrapping, footnotes, fields, OMML | a real document paginates identically to the reference for its full length |

A10 is the longest single unit in the programme and should be decomposed further when it is reached;
the others are one unit each. **Charts and SmartArt are deliberately not in this list** — they are
shared subsystems used by all three formats and belong to their own units, scheduled after A8 proves
the pipeline.

### Track B — the component catalogue

Derived from the control-type census, so its scope is evidence rather than invention. All 15,346
published commands reduce to **22 control archetypes**:

| Archetype | Instances | Archetype | Instances |
|---|---:|---|---:|
| `button` | 8,687 | `checkBox` | 228 |
| `gallery` | 1,773 | `tab` + `tabSet` | 218 |
| `toggleButton` | 1,261 | `button (dialogBoxLauncher)` | 130 |
| `group` | 1,193 | `task` + `taskFormGroup` | 97 |
| `menu` | 649 | `comboBox` + `dropDown` | 61 |
| `splitButton` | 440 | `category` | 28 |
| `contextMenu` | 302 | `labelControl` | 20 |
| `control` (host slot) | 230 | | |

Those become **B1–B16**. The rest of the catalogue is the surfaces the ribbon census cannot see:

- **B17 Surfaces** — dialog, modal, sheet, popover, flyout, task pane (docked, resizable, tabbed)
- **B18 Selection & formatting** — mini toolbar, enhanced screentip (title + description + shortcut)
- **B19 Pickers** — colour (theme grid, standard, recent, more), font picker with live preview **and
  the substitution warning**, measure input with units (pt/in/cm/%), slider, segmented control
- **B20 Document furniture** — status bar and segments, zoom control, scrollbars, pane splitter
- **B21 Navigators** — tree/outline, virtualised list, slide thumbnail rail, sheet tab bar
- **B22 Excel-specific** — formula bar, name box
- **B23 Annotation** — comment card and thread, tracked-change card
- **B24 Feedback** — toast, inline notification, progress, empty state
- **B25 Iconography** — the Fluent set, sized 16/20/24/32/48, tinted from tokens
- **B26 Mobile forms** — bottom sheet, contextual action bar, compact command bar with overflow,
  touch-sized hit targets
- **B27 In-canvas plates** — the generated gallery from Track A's A6

---

## 3 · Settled choices for Track B

**Stack.** Storybook with `@storybook/web-components-vite`. The components are framework-free custom
elements with Shadow DOM, per the architecture, so the catalogue and the eventual application consume
exactly the same artefacts — there is no "Storybook version" of a component to drift.

**Addons that earn their place.** Viewport (the mobile/desktop matrix), themes (light/dark from the
generated tokens), a11y (this is where the contrast rule from `DESIGN_TOKENS.md` §2.2 is *enforced*,
not just documented), interactions (galleries and menus are worth exercising), and visual regression
snapshots so an approved component stays approved.

**Icons: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons).** MIT licensed,
no attribution required, commercial use allowed, ~19,757 icons, authored by Microsoft. It is Office's
own icon language, so command recognition is as high as it can be, and the licence removes the
question entirely. Subset at build time to the icons actually referenced.

**Responsive is proved by container, not viewport.** The chrome is container-query driven, so every
component story renders in a resizable frame with desktop / tablet / phone presets. A component that
only looks right at a viewport width has not been validated.

**Every component ships three things** with it: its states matrix, its token dependencies (so a token
change's blast radius is visible), and its keyboard and screen-reader behaviour. A component without
those is not ready for audit.

---

## 4 · What is deferred to loop 2, explicitly

- Command dispatch and the `ShellBridge` protocol
- Document-to-chrome binding: selection driving the ribbon, ribbon driving the document
- Region composition and the Tauri window/surface host
- The mobile native surface plugin
- Backstage / File surface (~250 controls per app) — it is file management rather than document UI,
  and it is the piece most entangled with the application shell
- Editing interaction: hit-testing wired to selection, IME, undo/redo, clipboard

Track A's A7 (`mjx-session`) is the one arguable inclusion here. It is in loop 1 because rendering a
document that is *being edited* requires invalidation, and retrofitting invalidation into a renderer
built without it is the expensive order.

---

## 5 · Honest sizing

"Render all three properly" is the largest phase set in the plan. A8, A9 and A10 are each substantial
on their own, and A10 — Word's reflow engine with floating-object wrapping and page-splitting tables —
is where rendering projects historically die. The plan does not shrink that; it sequences it so the
foundations are proved on the easiest layout discipline (slides, absolute positioning) before the
hardest one (reflow) is attempted, and so the fidelity oracle exists before there is anything to
regress.

Track B is small by comparison — ~40 components — and its cost is design iteration rather than
engineering. That asymmetry is why it goes first in wall-clock terms even though it is second in
importance: your audit passes can run while Track A is still in its foundations.

---

## 6 · One scoping note

`TabSetPivotChartTools` is **434 controls in Excel**, larger than any core tab, and
`TabSetPivotTableTools` adds 136. Pivot tables and pivot charts are effectively a fourth application
inside Excel. They are inventoried in `OFFICE_FEATURE_INVENTORY.md` but have no phase, and whether
they belong to the Excel programme or become their own is a call worth making before A9 rather than
during it.
