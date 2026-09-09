# CLAUDE.md — guidance for AI agents working in mjx-ooxml-rs

This file orients Claude Code (and any coding agent) working in this repo. Humans: see `README.md`,
`PLAN.md`, and `CONTRIBUTING.md`.

## What this project is

A pure-Rust, cross-platform library to parse/edit/generate/(later)render OOXML (`.pptx`, `.docx`,
`.xlsx`). The overriding requirement is **fidelity**: open any file, edit it, write it back without
corrupting parts we did not touch.

## How we work here (non-negotiable process)

Every unit of work follows: **Plan → Plan-Optimization → thorough atomic implementation.**

1. **Plan** the atomic piece of work.
2. **Plan-Optimization** — *before writing code*, refine the design for **low memory, fast & reliable
   execution, and correctness**. Weigh allocations, copies, cache behavior, and failure modes. **No
   monkey-patching or shortcuts** — choose the design that is right, not merely working.
3. **Thorough atomic implementation** — finish the piece *completely, correctly, with tests* before
   moving on. No half-done atoms, no "fix it later" placeholders in shipped code.
4. **Discussion-first** — begin each working session by discussing the plan for what we're about to
   implement; bounce ideas/tradeoffs and converge before touching code. Extra planning time is welcome.

## Architecture rules

- **Layering:** dependencies point **downward only**, and each crate has a *rank*. An edge is legal
  **iff** it points to a **strictly lower** rank — so sideways is as illegal as upward, and the graph
  is acyclic by construction.

  | Rank | Crates |
  |---|---|
  | 0.0 — foundations, core | `mjx-ooxml-core`, `mjx-derive` |
  | 0.1 — foundations, XML | `mjx-xml` |
  | 0.2 — foundations, design tokens | `mjx-tokens` |
  | 1.0 — packaging / compatibility | `mjx-ooxml-types`, `mjx-opc`, `mjx-mce` |
  | 1.5 — typography | `mjx-text` |
  | 1.6 — box model contract | `mjx-layout` |
  | 1.7 — display list | `mjx-scene` |
  | 2.0 — shared markup, base | `mjx-dml` |
  | 2.1 — shared markup, spreadsheet | `mjx-sml` |
  | 2.2 — shared markup, upper | `mjx-chart`, `mjx-omml`, `mjx-vml` |
  | 2.5 — preset geometry | `mjx-geometry` |
  | 3.0 — formats | `mjx-pptx`, `mjx-docx`, `mjx-xlsx` |
  | 3.5 — the resident document | `mjx-session` |
  | 3.6 — the box models | `mjx-layout-pptx`, `mjx-layout-xlsx`, `mjx-layout-docx` |
  | 3.7 — the scene companions | `mjx-scene-pptx`, `mjx-scene-xlsx` |
  | 3.8 — the viewport | `mjx-view` |
  | 4.0 — facade | `mjx-ooxml` |
  | 5.0 — bindings | `bindings/mjx-python`, `bindings/mjx-wasm` |
  | 5.5 — platform boundary | `mjx-paint` |
  | — outside the graph | `mjx-fixtures`, `mjx-schema-gate`, `mjx-allocation-counter`, `mjx-render-oracle`, `mjx-reference-pack`, `mjx-canvas-harness`, `xtask` |

  **Shared markup is not flat**, and neither are the foundations. `mjx-xml` is built on
  `mjx-ooxml-core`. `mjx-tokens` (MJXOFF-156) sits above it at 0.2 and is *data*: the generated
  design-token table and its runtime resolver, with **no workspace dependency at all** and
  `mjx-ooxml-core` as its ceiling. Its rank says who may reach **it** — the client platform's
  renderer crates, all of which sit above the whole document graph — and it is below
  `mjx-ooxml-types` so every one of them can, without an upward edge. `mjx-text` (MJXOFF-157) is at
  1.5, above the packaging tier and below shared markup, and depends on `mjx-ooxml-core` and
  `mjx-tokens` alone: it is the font engine — faces, metrics, the three resolution tiers, the
  metric-compatible substitution table and the per-document substitution manifest — and **it has
  never heard of OOXML**. A document's font *reference* is `mjx-dml`'s model of `<a:latin>`; a font
  *engine* is this; the two meet above both, which is why an edge from `mjx-text` to a format crate
  or to `mjx-dml` is a layering violation rather than a convenience. `mjx-layout` (MJXOFF-160) is one
  step above it at 1.6 and depends on `mjx-ooxml-core` and `mjx-text` alone: it is the **box model
  contract** — the `BoxModel` trait, the `FragmentTree` every box model produces, the `Checkpoint`
  that makes flow layout resumable, and the spatial index that makes a hit test a query. Its rank is
  the seam the whole client platform is organised around: above a `FragmentTree`, scene building,
  painting, hit-testing, selection and export have never heard of OOXML, so the crate that defines it
  must sit where it cannot reach a format crate. That is what makes the box model swappable, and
  `crates/mjx-layout/tests/the_seam_holds.rs` names the identifiers as well as the layering test
  refusing the edge. `mjx-scene` (MJXOFF-161) is one step above it again at **1.7** and depends on
  `mjx-ooxml-core`, `mjx-tokens`, `mjx-text` and `mjx-layout` alone: it is the **display list** —
  nine commands, the paint and effect vocabularies, the resource tables and the flat binary encoding
  a `FragmentTree` becomes. Its rank looks wrong at first reading, because its consumers are painters
  and they sit far above; `docs/UI_PLATFORM_PLAN.md` §7 first wrote it at 2.6 for that reason and
  **that number was a defect**. The layering gate only refuses an edge that points up or sideways, so
  a `mjx-scene` above shared markup would make `mjx-scene → mjx-dml` a legal *downward* edge and the
  guarantee the crate exists to hold — below a display list, nothing has heard of a font, a layout
  algorithm or a document — would be enforced by nothing. At 1.7 the edge is refused by name. Do not
  raise it. `mjx-geometry` (MJXOFF-202) is the mirror image of those three, at **2.5**: every other
  rank is justified by what its crates may not *reach*, and this one by what may not reach **it**. It
  holds the `presetShapeDefinitions` path tables and the `GeometryProvider` that ends `mjx-scene`'s
  placeholder, so it is DrawingML and must live above `mjx-dml` (2.0) — which means `mjx-scene` (1.7)
  and `mjx-layout` (1.6) structurally cannot depend on it, and a display list therefore still cannot
  learn what a `.pptx` is. It is deliberately **below** the format tier, so `mjx-pptx` may reach it: a
  format crate is allowed to know what its own shapes look like. What the rank cannot do is stop
  `mjx-paint` (5.5) declaring the edge, so `crates/mjx-paint/tests/the_seam_holds.rs` forbids the
  painter naming it, exactly as it forbids `mjx-dml`. `mjx-session` (MJXOFF-167) is at **3.5**, above the format tier and below the facade, and it is the
  first rank in this table whose own number is *not* what holds its architectural property. It is the
  **resident document**: a session owns a parsed, mutable document and keeps it, records every
  mutation as an operation the instant it happens, and serialises dirty parts on a **schedule**
  rather than on every operation. The rank buys the ordinary thing — nothing in the format tier, and
  nothing below it, can depend on a session — and it cannot buy the interesting thing, because 3.5 is
  above 3.0 and `mjx-session → mjx-pptx` is therefore a legal downward edge for ever. What stops the
  editing half of the platform becoming OOXML-shaped is the crate's own construction: everything
  under `crates/mjx-session/src/` outside `src/ooxml/` is generic over `ResidentDocument`, is written
  in `mjx-layout`'s address vocabulary (`PartId`, `SourcePath`, `SourceRef` — no OOXML type appears
  in one), and compiles with the format crates absent, because they are `optional` behind a default
  `ooxml` feature. `crates/mjx-session/tests/the_seam_holds.rs` is what holds all three, by name and
  by file count, and it drives the whole journal, scheduler and undo machinery from a fourth
  residency that has never heard of a package — a trait with one implementation being a trait nothing
  has ever been swapped for. `mjx-view` (MJXOFF-168) is one step above it at **3.8**, and it is the
  same shape of rank for the same reason. It is the **viewport**: the pages on screen plus a prefetch
  ring biased in the direction of travel, a declared byte ceiling on every stage that holds something
  expensive, a scrollbar whose estimates become measurements without jumping under the reader's
  thumb, and a frame budget that defers rather than drops. Its rank buys that nothing at or below
  3.5 can reach a viewport — a `.pptx` reader with a scroll position inside it would be a batch
  library with a window manager in it — and buys **nothing** in the other direction, because 3.8 is
  above 3.0 and `mjx-view → mjx-pptx` is a legal downward edge for ever. *A viewport has never heard
  of OOXML* is held instead by construction and by `crates/mjx-view/tests/the_seam_holds.rs`: the
  crate is generic over `mjx-layout`'s `BoxModel` and over its own `SceneSource` and names no
  implementation of either, and it declares `mjx-session` with `default-features = false`, so a plain
  `cargo test -p mjx-view` is a build in which the three format crates are **not present** and a line
  that reached one would not compile. **`mjx-layout-pptx` (MJXOFF-169) sits between them at 3.6**,
  and it is the first implementation of `mjx_layout::BoxModel` and the first code in the workspace
  that turns a real `.pptx` into a `FragmentTree`: slide geometry, the shape tree in z-order,
  text-body layout, the nine bullet levels and autofit. It is above the format tier because it
  **consumes** `mjx-pptx`'s effective-property ladder rather than re-deriving it, and there is no way
  to do that from below. What its rank buys is the one thing that matters most: **`mjx-layout` at 1.6
  cannot depend on it**, so the box-model contract stays a contract instead of quietly becoming
  PowerPoint's own shape — and neither can `mjx-pptx`, which therefore cannot grow a layout engine.
  What it does not buy is the other direction: at 3.6, `mjx-layout-pptx → mjx-geometry` (2.5) is a
  legal downward edge for ever, so *a box model says which shape at what size and never resolves an
  outline* — the property that keeps `docs/UI_PLATFORM_PLAN.md` §4 L4's `GeometryProvider` swappable
  — is held by `crates/mjx-layout-pptx/tests/the_seam_holds.rs`, which checks **both** dependency
  sections, and by nothing else.

  **`mjx-layout-xlsx` (MJXOFF-171) shares that rank rather than sitting above it**, and the sharing
  is the decision. Excel's box model is a *grid* — two sparse indices over stated rows and `col`
  runs, every cell a binary search in `mjx-sml`'s packed store, and not one iteration of the
  seventeen billion addressable coordinates anywhere in it — and it consumes `mjx-xlsx`'s
  `SheetFormatting` and the `xf` ladder beneath it for exactly the reason PowerPoint's consumes
  `mjx-pptx`'s. Two box models at **equal** rank means an edge between them is *sideways*, which the
  layering gate refuses by name: a spreadsheet's box model must not know what a slide is, and a
  slide's must not know what a worksheet is. What
  the rank does not buy is, again, the other direction — at 3.6 an edge to `mjx-pptx` (3.0) is a
  perfectly legal *downward* edge — so *this box model reads one format, resolves no geometry and
  never paints* is held by `crates/mjx-layout-xlsx/tests/the_seam_holds.rs`, which refuses
  `mjx-geometry`, `mjx-scene`, `mjx-paint`, `mjx-pptx` and `mjx-docx` in both sections.

  **`mjx-layout-docx` (MJXOFF-174) is the third at 3.6, and it is the only one of the three that
  *reflows*.** PowerPoint's pagination is a slide index and Excel's is arithmetic over a row
  geometry: page *N* is reachable in both without ever looking at page *N−1*. Word's is **emergent**
  — where page 200 begins depends on everything on the 199 pages before it, and one font
  substitution moves every boundary in the document — so this crate is the first real consumer of
  `mjx-layout`'s `Checkpoint`, which exists for exactly that. It consumes `mjx-docx`'s
  `DocumentFormatting`, which **MJXOFF-174 added** and which is the reason laying a long document out
  is affordable at all: `Document::effective_paragraph_properties` re-parses `word/document.xml`,
  `word/styles.xml` and the theme on *every call*, which is right for a caller asking one question
  and quadratic for a layout engine asking per paragraph. The read-once surface is `mjx-xlsx`'s
  `SheetFormatting` answer applied to a second format, and `crates/mjx-docx/tests/residency.rs`
  asserts the two orchestrations of the one ladder agree paragraph by paragraph and run by run.

  What that rank buys is what it bought twice already, and nothing more: `mjx-layout` at 1.6 cannot
  depend on it, so the contract stays a contract; `mjx-docx` cannot grow a layout engine. What it
  does not buy is again the other direction, so `crates/mjx-layout-docx/tests/the_seam_holds.rs`
  refuses `mjx-geometry`, `mjx-scene`, `mjx-paint`, `mjx-pptx`, `mjx-xlsx` and both sibling box
  models in **both** sections — and refuses `mjx-dml` besides, which is a decision rather than an
  omission: MJXOFF-174's ticket lists DrawingML as a dependency and the tree does not need it,
  because `mjx-docx` resolves every `themeColor` and `asciiTheme` reference *before* a value reaches
  the box model. **Word has no scene companion yet.** PowerPoint's is `mjx-scene-pptx` and Excel's
  `mjx-scene-xlsx`, both at 3.7; `mjx-scene-docx` is the ticket that has to follow MJXOFF-174, and
  until it exists a Word `FragmentTree` cannot reach pixels.

  **`mjx-scene-pptx` at 3.7 (MJXOFF-170) exists because that same gate forbids the edge that would
  have made it a module.** `mjx_scene::ResourceResolver` is documented as implemented by *the box
  model's companion — the layer that issued the handles*, and for PowerPoint that layer is
  `mjx-layout-pptx`; but its seam gate refuses `mjx-scene` **by name**, on the ground that a box
  model which built a display list would have merged two stages the architecture separates on
  purpose. The gate is right and the answer to it is a crate. **3.7 is the only rank it can have**:
  it must name the box model (3.6) for the handles, the display list (1.7) for what they resolve
  into and the geometry tables (2.5) for the outlines — one step above the highest of the three —
  and it must stay below `mjx-view` (3.8), or a viewport would reach PowerPoint through it and stop
  being format-agnostic. What the rank does not buy is, again, the other direction: every format
  crate is a legal downward edge from 3.7, so *a resolver reads no document* is held by
  `crates/mjx-scene-pptx/tests/the_seam_holds.rs`, which refuses `mjx-pptx` in `[dependencies]` and
  permits it in `[dev-dependencies]` — a suite that proves a real deck's fills resolve has to open
  one.

  **`mjx-scene-xlsx` (MJXOFF-244) shares 3.7 for the same reason the two box models share 3.6.** It
  is Excel's companion — the resolver that turns `mjx-layout-xlsx`'s handles into a cell's pattern or
  gradient, a border band's colour and a run's own font colour — and it exists because
  `crates/mjx-layout-xlsx/tests/the_seam_holds.rs` refuses `mjx-scene` there by name, which is the
  same argument run again rather than an analogy to PowerPoint's. The shared rank makes an edge
  between the two companions **sideways**, and `xtask/tests/layering.rs` refuses that: the two
  formats meet at `mjx-scene`, which is the whole point of there being a display list, and a
  spreadsheet's resolver that could read a slide's would have made the display list a place where two
  formats negotiate rather than a vocabulary both answer in.

  What the rank buys is that `mjx-layout-xlsx` cannot grow a display-list builder and `mjx-scene`
  cannot learn what a `.xlsx` is. What it does not buy is either of two things a reader might assume.
  *A resolver reads no document* is held by `crates/mjx-scene-xlsx/tests/the_seam_holds.rs`, which
  refuses `mjx-xlsx` in `[dependencies]` and permits it in `[dev-dependencies]`. And the **absence of
  `mjx-geometry`** is not the rank's doing either — 2.5 is below 3.7, so that edge is legal and
  always will be. The crate does not declare it because a worksheet's fragment tree carries no
  `ShapeFragment` at all: a cell is a rectangle and so is a border band, so its `SheetGeometry`
  refuses every handle rather than standing in, and `DrawReport::placeholders` on an Excel render is
  a genuine but *narrow* assertion — zero against zero — rather than the count of unanswerable
  presets it is for a deck.

  **Two changes to R16's box model came out of building it, and both were "check before you
  consume" findings rather than scope.** A cell's four borders cannot be carried on a
  `mjx_scene::Decoration`, which has one stroke — so `mjx-layout-xlsx` now emits each edge as its own
  filled `BoxFragment`, exactly as `mjx-layout-pptx` does for a table cell, and `crates/mjx-layout-xlsx/src/border.rs`
  owns the (GUESS-laden) weight table. And a gradient fill carried a bare `is_gradient: bool`, which
  names no stops, so a gradient-filled cell had no path to a pixel at all; the stops are carried now.
  Both are stated where a reader will look, and the dash a filled band cannot draw is asserted as a
  loss in `crates/mjx-scene-xlsx/tests/the_dash_is_lost_at_the_band.rs` rather than described.

  `mjx-sml` sits between
  `mjx-dml` and `mjx-chart` because SpreadsheetML *is*
  shared markup — an embedded workbook is SpreadsheetML inside a `.pptx` or a `.docx` — which is what
  makes `mjx-chart → mjx-sml → mjx-dml` legal and lets `mjx-chart`'s duplicate workbook writer be
  deleted. Excel is therefore **two** crates: `mjx-sml` (the markup) and `mjx-xlsx` (the package and
  `Workbook` surface, format tier). **The bindings depend on `mjx-ooxml` alone** — never on a crate
  below it — and nothing depends on them.

  This is checked, not trusted: `xtask/tests/layering.rs` reads the real graph out of
  `cargo metadata --no-deps` and fails on any edge that does not point strictly down, naming both
  crates and both ranks. The table there and the table here are the same table; a new crate must be
  added to both. Dev-dependencies are deliberately exempt from the rank check (`mjx-derive` tests
  against `mjx-ooxml-types`; every format crate dev-depends on the gate) but may still never reach a
  binding or `xtask`.

  **`mjx-paint` at 5.5 is a rank that says who may reach *it*, and nothing else** (MJXOFF-163). It
  sits above the facade so that *nothing in the document graph can depend on it* — no format crate,
  no `mjx-ooxml`, and above all no binding, because `bindings/mjx-python` must never grow a GPU
  dependency. **Do not lower it**: below the format tier the formats and the facade would sit
  *above* it and could legally depend on it, which is the outcome the position exists to prevent.
  But the layering gate refuses only an edge that points up or sideways, so at 5.5 **every crate in
  the workspace is a legal dependency of `mjx-paint`** — and the architecture's second seam (*below
  a display list, nothing has heard of a font, a layout algorithm or a document*) is therefore held
  for that crate by an explicit manifest gate, `crates/mjx-paint/tests/the_seam_holds.rs`, and by
  nothing else. `mjx-scene` got 1.7 so the layering gate could refuse its illegal edge by name; **no
  rank can do the same job for a painter, in either direction.** A reader who assumes the rank is
  protecting the painter's own edges has it backwards.
- **Six test-only crates sit outside that graph**, and the last three are outside it in the
  *opposite* direction from the first three. `mjx-schema-gate` (the shared ECMA-376 schema and child-order gate,
  a `dev-dependency` of the three format crates and a dependency of `xtask`, whose
  `validation-artefacts --ingest` *reports* the verdicts a suite asserts), `mjx-fixtures` (the
  committed corpus at `tests/fixtures/`, with **no dependencies at all** so `mjx-opc`'s suites can
  reach it without an upward edge) and `mjx-allocation-counter` (the counting global allocator, also
  with **no dependencies at all**, because its two consumers sit in different tiers — `xtask`'s fuzz
  campaign and `mjx-sml`'s allocation gate — and nothing may depend on `xtask`) all sit *below* their
  consumers, which is why they carry no rank: they must be reachable from everywhere.

  **`mjx-reference-pack` (MJXOFF-207) has no rank because it sits at the top.** It authors the
  artefacts one Windows sitting needs and ingests what comes back, so it names the format tier
  (`mjx-pptx`, `mjx-docx`), `mjx-geometry` and `mjx-paint` **together** — a set of edges no shipped
  crate could legally declare, since 5.5 is above 3.0 and nothing in the document graph may sit above
  the platform boundary. A rank of 6.0 would have been the wrong shape of answer: a rank promises who
  may reach *it*, and the answer here is *nobody, in either dependency section*, which
  `xtask/tests/layering.rs` enforces directly and more strictly than it does for the first three.

  **`mjx-canvas-harness` (MJXOFF-166) sits beside the pack, at the same height and under the same
  rule.** It is the manual audit surface for the sixty-one in-canvas UI elements of
  `docs/client-platform/CANVAS_UI_INVENTORY.md` — a synthetic `FragmentTree` per element, the state
  matrix reachable by clicking, a hit-test visualiser drawn from `mjx-layout`'s own spatial index,
  and a live token editor that writes back to `docs/client-platform/data/tokens.json`. It names
  **no format crate, no facade, no packaging tier and no shared markup at all** — the geometry table
  included — which is what makes *"the harness needs no document and no fixture"* true, and that in
  turn is what lets in-canvas design be settled while the format renderers are still being built.
  What it names together is `mjx-paint` (5.5) and `mjx-render-oracle`, and nothing may depend on it
  in either section.

  **`mjx-render-oracle` (MJXOFF-165) sits one step below both, and they are the only two crates that
  may reach it.** It is the fidelity oracle: the three assertion tiers (`FragmentTree` snapshot,
  `DisplayList` snapshot, pixel diff) with the localisation that says *which* of them moved, the
  perceptual metric, the committed baselines and their approval events, and the plate gallery it
  renders — which U01 loads and which R11 calls `gallery::render` to produce its own copy of, rather
  than loading. It names **no format crate, no facade and no packaging beyond `PresetShapeType`**,
  which is what lets it have consumers where the pack has none. It is **not** free of DrawingML: it
  declares `mjx-geometry`, `mjx-dml` and `mjx-ooxml-types` for the `preset-star` specimen, so that
  `placeholders == 0` is asserted about the real `PresetGeometryProvider` rather than about a page
  that happens to contain no shapes. So the oracle knows what a *shape* is and does not know what a
  *file* is; the harness beside it knows neither. **The harness's property is the stronger of the
  two, not the same one.** **No crate with a rank may reach the oracle, in either section** — a
  `[dev-dependencies]` entry included, because this crate depends on `mjx-paint` and a test build
  that links Vulkan is still a build that links Vulkan — and a crate that is *itself* above the
  whole graph may. `mjx-reference-pack` was the first such consumer and `mjx-canvas-harness` is the
  second; the layering gate now asserts **both** edges by name, because the two use different halves
  of the crate — the pack takes the authority vocabulary and never renders a plate, and the harness
  takes the plate generator, the PNG encoder and the baseline store, which until MJXOFF-166 were
  permitted-but-unconsumed.

  ⚠ **Neither crate's property was enforced by anything until audit pass 10, and the paragraphs
  above described both as though it were.** A rank is what `xtask/tests/layering.rs` compares, and
  these two have none — so its downward rule holds exactly one thing about them (*nothing may depend
  on them*) and **nothing at all** about what they depend on. It would have accepted
  `mjx-canvas-harness → mjx-pptx` without a word. This is the same shape as `mjx-paint` at 5.5, and
  it has the same answer: `crates/mjx-canvas-harness/tests/the_seam_holds.rs` and
  `crates/mjx-render-oracle/tests/the_seam_holds.rs` each assert their crate's dependency set
  **exactly** and scan its sources for a forbidden name, both reading the one scanner at
  `crates/mjx-paint/tests/support/manifest.rs` — shared by `#[path]` include rather than copied,
  because what is worth sharing is the three parser defects MJXOFF-164 fixed in it, and a copy would
  not carry them. That scanner's own two instrument tests compile into all three gates, so a scan
  that had silently stopped seeing a category is caught in each. **Read this paragraph before adding
  a dependency to either crate.**

  That one edge exists because the *authority vocabulary* — `ReferenceProvider`, the three-state
  `Verdict`, the provider-attached exclusions — must have exactly one home, for the same reason
  `ReferenceAuthority` lives in `mjx-text` and not in two crates. MJXOFF-207 wrote it first;
  MJXOFF-165 needed it; nothing may depend on the pack; so it moved **down** into the oracle and the
  pack re-exports it as `mjx_reference_pack::authority`.

  All six are `publish = false` and no shipped crate depends on any of them. A test suite reads its
  fixture corpus from `mjx-fixtures` — never from a `const FIXTURES` list — and installs its
  allocator from `mjx-allocation-counter` rather than writing a second one.
- **Pure-Rust only *in the document graph*** — ranks 0 through the facade — no C/system libs. C
  tools (`xmllint`, LibreOffice) are for CI/tests only. `quick-xml` lives *only* behind `mjx-xml`;
  the ZIP backend *only* behind `mjx-opc`. PyO3 and wasm-bindgen live *only* behind `bindings/`.

  **The rule used to say "in shipped crates", and MJXOFF-163 amended it, because a pixel cannot
  reach a screen without the operating system's graphics stack.** `wgpu` links `ash` (Vulkan),
  `metal`/`objc2` and `windows-rs`, and no amount of Rust removes that. So the boundary is
  *declared* rather than crossed quietly: **`mjx-paint` at rank 5.5 is the platform boundary and the
  only crate that may link the platform's graphics API.** Everything below it — `mjx-scene`,
  `mjx-layout`, `mjx-text`, the whole document graph — stays pure Rust, so the headless, `wasm32`,
  export and test paths never require a GPU. That is not a hope: `tiny-skia` is a **required**
  second painter (R09) precisely so that a fully pure-Rust path to pixels always exists, and the
  cross-build matrix builds the library graph for five targets without one.

  **That painter now exists** (MJXOFF-164). `mjx-paint` holds four painters against one contract —
  `wgpu`, `tiny-skia`, PDF and SVG — and three of them link nothing at all. The software one is not
  a fallback: every golden image from R10 onward is taken through it, headlessly. It is also the only
  honest way to test the first, because **two independent implementations agreeing is evidence and
  one implementation agreeing with itself is not** — which is why `compare_painters` *refuses* two
  painters that report the same name rather than reporting how well one agrees with itself.
- **One lowering, four painters, and no painter re-walks a display list.** `plan_frame` turns a list
  into layers and draw operations with no graphics API in it, and every painter consumes that. A
  painter that walked the commands itself would be a *second interpretation*, and a cross-painter
  comparison would then be comparing interpretations rather than rasterisers. For the same reason,
  anything four painters must agree about is stated **once** and read from there: the fifty-four
  preset hatch masks (`mjx_paint::PATTERN_MASKS`), the gradient ramp resolver, `resolve_outline` and
  `dash_lengths` in `mjx-scene`, and `draws_behind`/`replaces_subtree` for where an effect's result
  goes relative to its subtree. A shared answer that is *documented* as the single source of truth
  and not actually *read* is worse than none: `draws_behind` was exactly that between MJXOFF-163 and
  MJXOFF-164, so flipping it failed a test and changed no pixel while swapping two lines painted
  every shadow on top of its shape with the suite green.
- **PDF and SVG are exporters, not stages on the way to raster.** Both consume the display list
  directly, through the same lowering. This **supersedes** `PLAN.md`'s Phase 7 line describing an
  IR → SVG → raster → PDF chain, and the supersession is written there too. A PDF made by printing
  an SVG carries no selectable text, which is the requirement the whole exporter was written around;
  it is checked with `pdftotext`, a reader this project did not write, because checking our own
  export with our own reader would prove nothing about the format.
- **`unsafe_code = "deny"`** workspace-wide; a crate that truly needs it must `#[allow(unsafe_code)]`
  locally **with a written safety justification**. **No crate in the document graph does.** Four
  places allow it — three outside the shipped graph, and the platform boundary:
  - `crates/mjx-paint` (MJXOFF-163), with **one** hand-written `unsafe` block: the surface created
    from a window handle the shell supplied. No safe API can promise that a raw window handle
    outlives the surface made from it; that promise is the window system's invariant and the
    shell's to keep, it is documented at the call site and stated as the caller's obligation on
    `DesktopWindow::new`, and every other `unsafe` in the crate's tree belongs to `wgpu`. **CI greps
    `crates/mjx-paint/src`** for the `unsafe` keyword outside a comment and fails on any line that
    does not carry the marker `MJX-PAINT-SURFACE-UNSAFE`, in the same job that guards
    `bindings/*/src`; `crates/mjx-paint/tests/the_seam_holds.rs` asserts the same thing one round
    trip earlier and additionally that there is exactly **one** such line.
  - `bindings/mjx-python` and `bindings/mjx-wasm`, with the same justification: *no hand-written
    `unsafe`; every unsafe block is generated by `#[pyclass]` / `#[wasm_bindgen]`.* CI greps
    `bindings/*/src` for `unsafe` outside that comment and fails if it finds any, so the
    justification cannot quietly become false.
  - `crates/mjx-allocation-counter`, one `unsafe impl GlobalAlloc` whose every method forwards its
    arguments unchanged to `std::alloc::System`, so it adds no obligation the system allocator does
    not already carry. It is what gives the fuzz campaign a *measured* memory ceiling instead of an
    OOM kill (`xtask`, a host-only developer binary nothing depends on) and what lets `mjx-sml`'s
    cell-store gate assert a byte bound instead of asserting by inspection. It was `xtask`'s own
    module until MJXOFF-95 needed the same instrument from a crate that may not depend on `xtask`;
    the answer to a second consumer is one implementation both can reach, not a second
    `unsafe impl`.
- **No `unwrap`/`panic`/`expect` on untrusted input** in library code paths — inputs are untrusted
  files. Return typed errors (`thiserror`). `anyhow` only in `xtask`/tests/examples.

## Fidelity rules (the reason the project exists)

- **Part-level laziness + copy-on-write:** parts stay raw bytes until first mutation; untouched parts
  re-emit verbatim. **On first edit, drop the raw bytes and mark the part dirty — the model is now
  authoritative; serialize at commit, once, however many edits have accumulated** (MJXOFF-167).

  That rule used to read *"on first edit, serialize from the model and drop raw bytes"*, which
  described one moment where the implementation has always had two: `Package::part_tree_mut` drops
  the bytes and marks the part dirty, and `Package::save` is where XML is actually written. Batched
  commit is what made the difference matter — twenty keystrokes into one run must produce twenty
  model mutations and **one** serialization, on a schedule — so the timing is now stated rather than
  glossed, and `Package::settle_edited_parts` is the call that performs it and clears the dirty set.
  **The round-trip guarantee is untouched**: an untouched part is never marked dirty and still
  re-emits byte for byte. Only the timing moved.
- **Unknown bucket:** every modeled complex type carries `extra: Vec<RawNode>` for unknown children,
  and preserves unknown attributes, attribute order, and namespace prefixes.
- **MCE** (`mc:AlternateContent`/`Ignorable`/`ProcessContent`) is handled in `mjx-mce`, preserved on
  write and resolved (non-mutating) on read/render.
- **Round-trip contract:** per-part decompressed-payload byte identity + structural container identity
  (not identical ZIP bytes).

## Settled implementation choices

- Hybrid model (arena for bulk data, owned trees for small structures).
- **One byte-budgeted cache, at the floor of the workspace** (MJXOFF-168).
  `mjx_ooxml_core::ByteBudgetCache` is the workspace's only least-recently-used cache with a declared
  byte ceiling, and it is at rank 0.0 because its consumers are at 1.7 (`mjx-scene`'s tessellation
  cache), 3.5 (`mjx-session`'s worksheet residency) and 3.8 (`mjx-view`'s per-stage caches) — a cache
  written in the highest of those is unreachable from the other two, so the choice was one
  implementation at the floor or three that drift. It is the same argument, and the same answer, as
  `Emu` moving down out of `mjx-dml` in MJXOFF-160. Two things it is built around, both of which are
  asserted rather than documented: **a cache that evicts everything satisfies every byte bound
  perfectly**, so every budget gate in this workspace is two-sided and reads `evictions` before it
  trusts a ceiling; and **an eviction count cannot say which entry went**, so
  `least_recently_used()` exists for a test to name it. A *pinned* entry — a worksheet with unwritten
  edits, the page the next frame paints — is charged to the budget and never evicted, which makes the
  pinned set a floor the budget cannot go below; that is stated in the type's own documentation
  rather than discovered.
- Interning + `Cow` for strings.
- Hand-written de/serialization via `mjx-derive` (not serde).
- Generated `mjx-ooxml-types` (simple types + constant tables) via `xtask`; **output is committed**,
  never a `build.rs`. Regenerate with `cargo run -p xtask -- codegen` (needs local `References/`).
- **One design-token source, three generated consumers** (MJXOFF-156). The chrome is HTML and the
  document canvas is Rust, and *a canvas cannot inherit a CSS custom property*, so
  `docs/client-platform/data/tokens.json` is generated into `ui/tokens/tokens.css`,
  `ui/tokens/tokens.ts` and `crates/mjx-tokens/src/generated.rs` by `cargo run -p xtask -- tokens` —
  committed output, same doctrine. Two divergence gates hold it together: `xtask/tests/tokens.rs`
  proves the artefacts are *derived* from the source, and `mjx-tokens`'s `artefacts_agree` suite
  proves they are *equal to each other*. **The contrast rule of `DESIGN_TOKENS.md` §2.2 is enforced,
  not documented:** every colour token declares its usage, and a token tagged for text that does not
  reach 4.5 : 1 against its declared background is refused.

  **That rule lives in `mjx-tokens`, and the generator *calls* it** (audit pass 10). It used to be a
  private `check_usage` inside `xtask/src/codegen/tokens/model.rs`, which made the generator the only
  thing in the workspace able to apply it — and the generator is **not the only writer of
  `tokens.json`**. `mjx-canvas-harness`'s live token editor writes back to that file and may not
  depend on `xtask`, so a text colour tweaked in the editor was accepted at the keystroke and failed
  the next `cargo run -p xtask -- tokens`: exactly the failure that editor's type validation exists
  to prevent, for the rule a design tweak is most likely to trip. `mjx_tokens::check_usage`,
  `contrast_ratio` and `ColorUsage` are now the one copy, `TokenIdentity` carries `usage` and
  `background` as **data** rather than as prose in a doc comment, and the editor sweeps *every*
  text-tagged token before a byte reaches the disk — because the rule binds a pair, and darkening
  `color.paper` breaks every `on-light-text` colour measured against it. `xtask` therefore declares
  `mjx-tokens`: a downward edge from a host-only binary, and the thing that keeps there being one
  copy of the rule.

## Bindings

Two workspace members project the facade, and neither adds behaviour: every method calls exactly one
`mjx_ooxml::Deck` method, and every class wraps exactly one value.

- **`bindings/mjx-python`** — PyO3, abi3-py39 wheels, module `mjx_ooxml`. The mapping is the
  **identity**: `mjx-ooxml`'s names are already `snake_case` and already self-explanatory, so nothing
  is renamed. The single exception is forced — the `None` *member* of fourteen enumerations is
  spelled `NONE`, because `None` is a Python keyword. (Nine until MJXOFF-137; Excel's own
  vocabulary added five more.) Committed `.pyi` + `py.typed`, checked by
  `mypy --strict` and by `tests/test_stub_parity.py`, which compares the stub to the compiled module
  in both directions.
- **`bindings/mjx-wasm`** — wasm-bindgen, one npm package with conditional exports. Method names are
  **camelCase**, from an explicit `js_name` on every one, because a `snake_case` API is an immediate
  smell to a TypeScript consumer. Two further shapes differ, both forced: a range argument becomes
  two numbers, and a `Format`'s accessors are free functions (a wasm enumeration is a number in
  JavaScript and cannot carry a getter). **No serde** — `serde-wasm-bindgen` would need derives on
  `FillSpec`/`ColorSpec` in the *shipped* `mjx-dml`, contradicting the hand-written-de/serialization
  decision above.

The acceptance test for both is the same, and there are now four of it. Each of the three
walkthroughs — `crates/mjx-ooxml/examples/build_a_deck.rs`, `build_a_document.rs`,
`build_a_workbook.rs` — exists a second time under `bindings/mjx-python/tests/` and a third under
`bindings/mjx-wasm/tests/node/`, and so does the whole validation catalogue
(`xtask/src/validation/` ↔ `bindings/mjx-python/tests/test_validation_artefacts.py` ↔
`bindings/mjx-wasm/tests/node/validation_artefacts.mjs`). Every one compares its output against the
Rust one **part by part, byte for byte**. A method wired to the wrong `Deck` method changes one
payload and fails there.

When the facade grows a method, both bindings grow it: a binding that projects part of the surface is
a surface two languages cannot use.

## Naming convention (comprehensive, self-explanatory identifiers)

OOXML symbols are cryptic (`ST_OnOff`, `ST_Jc`; values like `t`/`ctr`/`dist`). **Every public
identifier in this project must be self-explanatory** — a reader should not need the spec to
understand a name. This applies to generated *and* hand-written types.

- **Type names:** drop `ST_`/`CT_`; expand abbreviations to full words; module-namespace per schema
  (e.g. `wml::Justification`, never a bare `Jc`).
- **Variant/field names:** expand cryptic tokens to full words (`t` → `Top`, `dist` → `Distributed`).
  Where the meaning is not inferable from the token, source the name from the **ECMA-376 Part 1
  prose** — never guess.
- **Wire tokens are preserved exactly, never guessed:** each type/variant maps to its exact XSD
  string for (de)serialization; the original `ST_*` symbol and wire token appear in the item's docs.
- **Two-valued types use two values:** boolean-ish types (`ST_OnOff` family) are `bool` /
  `Option<bool>`; all wire spellings are normalized on read and one canonical form is written (see
  `mjx-ooxml-types::support`).
- **Measures are named with intent** (`Twips`, `Emu`, `Percentage`, …), units explicit.
- **Deterministic sanitization** of invalid identifiers (Rust keywords, digit-leading, punctuation).

The generator lives in `xtask/src/codegen/` — curated naming data (overrides, abbreviations) in
`spec.rs`; the engine in `naming.rs`. Adding a schema means growing the tables, not the engine.

## Commands

```sh
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- codegen        # regenerate mjx-ooxml-types from References/ (local only)
cargo run -p xtask -- tokens         # regenerate the three design-token artefacts from
                                     #   docs/client-platform/data/tokens.json; `--check` refuses
                                     #   instead of writing, which is what the tests run
cargo run -p xtask -- fuzz           # the untrusted-input campaign; on demand, never on CI push
cargo run -p xtask -- corpus         # the large-file benchmarking corpus (--mem <pptx|docx|xlsx>)

# The reference pack (MJXOFF-207): the four artefacts one Windows sitting needs, and the preliminary
# LibreOffice pass over them. `preliminary` is a REPORT and not a gate — a LibreOffice disagreement
# is a thing to look at before the sitting, never a build failure, and never parity. The exports come
# back into tests/office-exports/, which is EMPTY and which no agent may fill:
# docs/validation/07-the-reference-pack.md is the hand-off that says how a person does.
cargo run -p mjx-reference-pack -- generate     target/reference-pack
cargo run -p mjx-reference-pack -- preliminary  target/reference-pack
cargo run -p mjx-reference-pack -- items        # the seven questions the sitting answers

# The fidelity oracle (MJXOFF-165): three assertion tiers over five specimens, checked against
# committed baselines. `check` is the gate; `gallery` writes the plates, the manifest R11 loads and
# the page a person approves from.
#
# ⚠ TWO PATHS ARE DELIBERATELY NOT AUTOMATIC, because a golden image generated by the code under
# test always matches the code under test:
#   * `regenerate` refuses without MJX_ORACLE_REGENERATE=1, and REMOVES the approval it overwrites.
#   * `approve` refuses without MJX_ORACLE_APPROVED_BY, and refuses outright under CI.
# Every committed baseline today is stamped `approver = generator`, which is a real approval record
# — the digest binding is live — and is NOT a human review. `list` says which.
# The canvas UI harness (MJXOFF-166): the sixty-one in-canvas elements of
# docs/client-platform/CANVAS_UI_INVENTORY.md, exercised BY HAND. `serve` is the deliverable — a
# local page with a scene list, a state panel, a ruler, a hit-test overlay, a pixel inspector and a
# live token editor that writes back to docs/client-platform/data/tokens.json. Every pixel of the
# element is drawn in Rust through the software painter; the page is chrome.
#
# ⚠ ELEVEN OF THE SIXTY-ONE ARE ABOUT TOUCH, and a touch target judged with a mouse has not been
# judged. `--host 0.0.0.0` and the machine's address on the same network puts the harness on a
# phone. The Tauri mobile surface the ticket specifies does NOT exist; see
# docs/client-platform/CANVAS_UI_INVENTORY.md §4.2 for exactly what is missing.
cargo run -p mjx-canvas-harness -- serve                  # http://127.0.0.1:7761/
cargo run -p mjx-canvas-harness -- serve --host 0.0.0.0   # …and on a phone
cargo run -p mjx-canvas-harness -- check                  # every plate against its baseline
cargo run -p mjx-canvas-harness -- plates [directory]     # the plates, the manifest and the gallery
cargo run -p mjx-canvas-harness -- checklist              # docs/client-platform/CANVAS_UI_AUDIT.md
MJX_ORACLE_REGENERATE=1 cargo run -p mjx-canvas-harness -- regenerate [number]
MJX_ORACLE_APPROVED_BY='Your Name' cargo run -p mjx-canvas-harness -- approve <number> "why"

cargo run -p mjx-render-oracle -- check
cargo run -p mjx-render-oracle -- gallery        # target/oracle-gallery/index.html
cargo run -p mjx-render-oracle -- list
MJX_ORACLE_REGENERATE=1 cargo run -p mjx-render-oracle -- regenerate [specimen]
MJX_ORACLE_APPROVED_BY='Your Name' cargo run -p mjx-render-oracle -- approve <specimen> "why"

# The artefacts the human Microsoft Office pass reads (MJXOFF-122). It marks nothing: see
# docs/validation/00-method.md. Two runs are byte-identical, and the two bindings produce the same
# set. MJX_REQUIRE_OFFICE_CORPUS=1 makes an empty tests/office-authored/ a failure rather than a skip.
cargo run -p xtask -- validation-artefacts --list
cargo run -p xtask -- validation-artefacts [--format pptx|docx|xlsx] [--area <id or number>]

# The other direction (MJXOFF-130): hand it a file saved out of Office and it reports which entry the
# file answers, whether it round-trips at the container and through the facade, whether the package
# invariants hold, whether its child order matches ours, whether it validates, and where it would be
# committed. It copies nothing. The corpus at tests/office-authored/ is EMPTY and no agent may fill
# it: a file's value there is entirely its provenance. docs/validation/06-the-office-pass.md is the
# hand-off that says how a person does.
cargo run -p xtask -- validation-artefacts --ingest <file> --area 2
cargo test -p xtask --test office_corpus

# The ECMA-376 gate, one harness over all three formats. Skips without References/; MJX_REQUIRE_SCHEMA=1
# turns any absence into a failure, which is what CI sets.
MJX_REQUIRE_SCHEMA=1 cargo test -p mjx-pptx --features vml --test schema_validity
MJX_REQUIRE_SCHEMA=1 cargo test -p mjx-docx -p mjx-xlsx -p mjx-schema-gate

# Python binding (needs maturin, pytest, mypy — a virtualenv is enough)
cd bindings/mjx-python && maturin develop && pytest && mypy --strict

# WebAssembly binding (needs wasm-pack and the wasm32-unknown-unknown target)
bindings/mjx-wasm/build-npm.sh                       # both targets into npm/dist/
node --test "bindings/mjx-wasm/tests/node/*.mjs"     # the published shape
wasm-pack test --node bindings/mjx-wasm              # the Rust side, in a wasm runtime
```

## Git / commits

- **Project-setup commits go on `main`;** once features start, **branch per feature + open a PR**.
- **Atomic commits** (one self-contained change, easy rollback/cherry-pick); commit only when
  `cargo build` + `cargo test --workspace` are green.
- **Do NOT add `Co-Authored-By` or any AI-attribution trailer** to commits.
- `References/` is git-ignored — never stage it; put test inputs under `tests/fixtures/`.

## Versioning — three files move together

The version lives in **three** places, and a bump that misses one leaves CI red:

1. `Cargo.toml`'s `[workspace.package] version` — the patch digit only; the user raises minor and
   major.
2. `CHANGELOG.md` — a new entry.
3. **`bindings/mjx-wasm/npm/package.json`'s `"version"`** — the npm package states its own, and
   `bindings/mjx-wasm/build-npm.sh` **refuses to build** when it disagrees with the workspace. That
   refusal is deliberate: a published package that cannot be traced back to a commit is worse than
   one that will not build.

The third is the one that gets forgotten, because the rule used to be written only inside
`build-npm.sh` — where the person doing the bump never looks. MJXOFF-156 bumped the workspace, left
the package at the older number, and the `wasm-pack` job was red for two children before anyone
asked CI. Move all three, in the same commit.
