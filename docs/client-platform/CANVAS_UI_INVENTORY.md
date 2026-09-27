# The in-canvas UI — the exhaustive inventory, and the harness that exercises it

Everything the Rust renderer draws that is **not document content**. It is the chrome that lives
inside the canvas because it has to track document geometry at 60 fps, and it is the most
design-sensitive surface in the product.

It gets its **own manual runtime test harness**, as a separate task from the Storybook catalogue.
This document is the inventory that makes "exhaustive" checkable, and the specification of the
harness that covers it.

---

## 1 · Why a runtime harness and not image plates

The earlier proposal — render each element to a PNG and publish the plates into Storybook — proves
**appearance** and nothing else. Almost everything on this surface is only correct in **motion and
under the pointer**:

a handle you cannot drag tells you nothing about its grab tolerance · a snap guide has to *engage* to
be judged · marching ants and a caret are animations · touch targets are only real on a touch
device · hit-testing priority between overlapping affordances only shows up when you try to grab the
wrong one · a dimension readout has to update while you resize.

So the roles split, and both are kept:

| Instrument | Role | Driven by |
|---|---|---|
| **Runtime harness** | the audit surface — manual, interactive, exhaustive | a person |
| Golden plates | regression only — appearance locked once approved | CI, headless `tiny-skia` |

Both enumerate from **this document**. An element that has no harness scene and no plate is not
covered, and the harness fails its own completeness check — the inventory is the test.

---

## 2 · The inventory

Sixty-one elements in six families. Each is a harness scene and a plate; each has a state matrix
(default / hover / active / focused / disabled, × light / dark, × 1× / 2× / 3×, × pointer / touch).

### 2.1 · Object selection (11)

1. Selection outline — single object
2. Selection outline — multiple objects, with the union bounds
3. Resize handles — four corner, four edge; hover and active states
4. Rotation handle, its tether, and the live angle readout
5. Rotation snap indicator (15° increments, and the shift-constraint state)
6. Shape adjustment handles (the `avLst` control points)
7. Connection sites — shown while a connector is being drawn
8. Connector endpoints, hover targets, and the reroute preview
9. Group selection outline versus a selected child inside a group
10. Locked-object indicator
11. Off-canvas / partially-off-slide indicator

### 2.2 · Text selection and editing (10)

12. Caret — blink cadence, hairline thickness across DPI
13. Caret — bidi split form, and the direction indicator at a script boundary
14. Selection fill — single line, multi-line, ragged trailing edge
15. Selection fill — across columns, across pages, across a table's cells
16. IME composition underline and the candidate-window anchor rectangle
17. Spelling, grammar and style squiggles (three weights and colours)
18. Hyperlink hover affordance
19. Text-overflow and autofit indicators inside a shape
20. Placeholder prompt text (empty placeholder on a slide)
21. Drag-to-select autoscroll edge indicator

### 2.3 · Grid selection — Excel (9)

22. Active-cell border, distinct from the range border
23. Range selection fill and border
24. Multi-range (discontiguous) selection
25. Fill handle, and the fill preview with its series tooltip
26. Row and column header highlight for the current selection
27. Frozen and split pane divider lines
28. Merged-cell selection behaviour
29. AutoFilter dropdown affordance in a header cell
30. Comment / note indicator triangle

### 2.4 · Manipulation (13)

31. Drag ghost / translucent preview
32. Smart alignment guides — edge, centre, and equal-spacing
33. Snap indicators — to grid, to guide, to object
34. Live dimension and position readout during a drag
35. Resize ghost with its dimension readout and aspect-lock state
36. Crop handles and the out-of-crop darkening overlay
37. Table row and column resize affordance with its preview line
38. Table insert affordance (the between-rows/columns control)
39. Custom-geometry vertex editing — vertices, control points, tangent lines
40. Motion-path editing handles and the path preview (PowerPoint animation)
41. Marching ants (cut/copy indication, and its animation cadence)
42. Drop indicator — between paragraphs, between slides, into a cell
43. Multi-touch transform feedback — pinch scale and two-finger rotate

### 2.5 · Document furniture (13)

44. Canvas backdrop, page fill, page border and page shadow
45. Page gap and the page-break indicator (Word)
46. Header and footer dimmed regions with their boundaries
47. Text-wrap boundary preview around a floating object
48. Column boundaries and the balanced-column indicator
49. Section-break markers
50. Footnote separator and continuation notice
51. Non-printing marks — paragraph, space, tab, break, optional hyphen
52. Field shading and the field-selection state
53. Bookmark brackets
54. Comment anchor and its connector line to the margin card
55. Tracked-change bars and inline insert/delete rendering
56. Slide guides, drawing-canvas boundary, and the safe area

### 2.6 · State and feedback (5)

57. Canvas focus ring
58. Page placeholder for content not yet laid out, and its progressive-refinement tier
59. Missing-resource placeholders — absent image, unsupported media
60. Font-substitution badge (the user is entitled to know what they see is not what was sent)
61. Print-area boundary (Excel) and zoom-dependent hairline rendering

---

## 3 · The harness

**A standalone runnable application, not a test suite.** `cargo run -p mjx-canvas-harness`.

**It does not need a document.** Every scene is synthetic — a shape, a text block, a grid, a page —
constructed directly against `FragmentTree`. That is deliberate: it means the harness works from the
moment `mjx-scene` and `mjx-paint` exist, long before any of the three layout engines do, so
in-canvas UI can be audited and settled while the format renderers are still being built.

**Shape of it:**

- A scene list — 61 entries, one per inventory item, searchable, with the inventory number shown.
- A state panel per scene — toggles for every axis of the state matrix, so a state is reached by
  clicking rather than by contriving an interaction.
- A live token editor — every design token adjustable at runtime, because the point of the audit is
  to *tweak*. Changes write back to the token source, so an approved value is captured rather than
  transcribed.
- An overlay ruler and a pixel-inspector readout, for judging hairlines and hit tolerances at 1×,
  2× and 3×.
- A hit-test visualiser — the grab regions drawn as translucent overlays, which is the only honest
  way to judge whether a touch target is big enough.
- A completeness check that fails if any inventory entry has no scene.

**It must run on the mobile target, not only on desktop.** Touch handle sizing, gesture conflicts
between pan/pinch and object drag, palm rejection, and thumb-reach are not judgeable on a desktop
pointer, and eleven inventory entries are specifically about touch behaviour. The harness therefore
ships as both a native desktop binary and a build for the mobile surface. This is the earliest thing
in the whole programme that forces the mobile render surface to exist, which is a benefit rather than
a cost — it de-risks the platform piece long before the application needs it.

**Track placement:** its own unit, **A6b**, immediately after A6 (the oracle and the plate
generator), depending only on A4 and A5. It is not a sub-task of the Storybook catalogue and does not
wait for A8.

---

## 4 · What was built (MJXOFF-166), and the one thing that was not

`crates/mjx-canvas-harness`. Every requirement of §3 is met except the last paragraph's, and this
section is the honest account of both halves.

### 4.1 · The shape it took, and why

**A local server and a browser, not a native window.** `cargo run -p mjx-canvas-harness -- serve`
binds `127.0.0.1:7761` and prints a URL. The scene list, the state panel, the token editor and the
overlay switches are HTML driven by the generated design tokens; **every pixel of the element itself
is drawn in Rust** — `mjx-layout` → `mjx-scene` → `mjx-paint`'s software painter — and reaches the
page as an `<img>`. There is no `<canvas>`, no SVG generation, no plotting library and no network
reference anywhere in the page, and
`crates/mjx-canvas-harness/tests/the_page_is_the_chrome_and_nothing_more.rs` is what holds that.

Three reasons this shape was chosen over a native window, in the order they matter:

1. **It reaches a phone.** `serve --host 0.0.0.0` and the machine's address on the same network puts
   the harness in a real mobile browser, at a real density, under a real thumb. Eleven of the
   sixty-one elements are about touch and none of them is judgeable any other way.
2. **It is the same instrument at both sizes**, which is what the user's audit rule asks for: the
   layout collapses below 900 pixels and the element stays the widest thing on the screen.
3. **The workspace has no windowing crate**, and adding `winit` to reach a desktop-only surface would
   have bought the one case a browser already serves.

The pixel inspector is worth one line of its own. A browser can read a pixel out of an `<img>` with
two lines of `getImageData`; it does not. The probe is an endpoint answered from the same `Pixels`
the PNG was encoded from, because a second reading of the image would be a second answer to *"what
colour is this"*, and the reason the canvas is in Rust is that there should be one.

### 4.2 · ⚠ The mobile *surface* does not exist, and this child did not build it

§3 says the harness "ships as both a native desktop binary and a build for the mobile surface", and
MJXOFF-166 adds that this is "the earliest thing in the whole programme that forces the mobile
render surface to exist — a `SurfaceHost` implementation over the platform's native surface,
delivered as a small Tauri plugin (`tauri-plugin-mjx-surface`)". **That plugin was not written**, and
the reason is not scope: a plugin that compiles and has never created a surface would satisfy the
sentence and prove nothing, which is the shape of hand-off this programme keeps having to undo.

What is missing, precisely:

| Piece | State |
|---|---|
| A `SurfaceHost` over a platform window | **Exists** — `mjx_paint::DesktopWindow` takes `WindowHandles` and is already platform-agnostic |
| An Android/iOS *shell* that owns a window and hands over its handle | Missing |
| `tauri-plugin-mjx-surface`, a Tauri 2 mobile plugin wrapping that hand-over | Missing |
| An Android SDK/NDK or Xcode toolchain to build and run either | Not present in this environment |

**The premise that this child "first forces the mobile render surface to exist" is not quite right,
and the correction is useful.** R08 generalised the surface already: `SurfaceHost` is a trait over
`SurfaceTarget::Window(WindowHandles)`, and a mobile surface is a caller that supplies an
`ANativeWindow` or a `CAMetalLayer` rather than a new implementation of anything. What is genuinely
missing is a **shell** — an application that owns a window on a phone — and that is a Tauri and
toolchain problem rather than a rendering one.

Until it exists, the eleven touch entries are exercised in a **mobile browser** over the LAN. That is
a real touch device, a real density and a real thumb, and it is not the native surface; a reader who
needs the native path should treat this row of the inventory as covered by the harness and *not* by
the platform.

### 4.3 · The gates

| Gate | What it refuses |
|---|---|
| `every_entry_draws.rs` | Sixty-one titled empty canvases. Five counters per entry — placeholders, draw calls, covered pixels, **distinct colours**, declared command kinds — plus a command count strictly above the bare stage's |
| `the_axes_are_not_identities.rs` | A state toggle that does nothing. Each entry declares which axes move its pixels and the suite measures the declaration in both directions, prints the per-axis counts, and requires all sixty-one renders to be distinct pictures. Density gets its own gate: every stroke width in proportion at 1×, 2× and 3× |
| `the_visualiser_shows_the_index.rs` | A hit-test overlay computed twice. A grab region records a *fragment*, and the rectangle is `SpatialIndex::bounds_of` inflated by the input device's padding |
| `the_token_editor_writes_back.rs` | An editor that reformats the token source, or one whose changes do not reach the canvas |
| `the_plates_go_through_the_oracle.rs` | An unapproved plate passing, a second manifest schema, and a human approval nobody gave |
| `the_page_is_the_chrome_and_nothing_more.rs` | A page that draws, or that reaches the network |

The audit checklist is `docs/client-platform/CANVAS_UI_AUDIT.md`, regenerated by
`cargo run -p mjx-canvas-harness -- checklist`. **Nothing in it is ticked, and no agent may tick
it.**
