# mjx-ooxml-rs — Roadmap

The living, public roadmap. For the deep architecture rationale, see the design decisions summarized
below and in `CLAUDE.md`.

## Objective

A pure-Rust, cross-platform library that can **open any OOXML file, load it fully into RAM, edit it at
runtime, and write back a valid file** — preserving everything it does not explicitly model — for
PowerPoint, Word, and Excel, reachable from Rust, Python and TypeScript. Rendering comes later.

## Guiding principles

1. **Fidelity-first** — never corrupt parts/elements/attributes we do not understand.
2. **Pure-Rust only** in the shipped graph (clean wasm / Android / iOS cross-compilation).
3. **Lazy, part-oriented** — parts are raw bytes until touched; untouched parts re-emit verbatim.
4. **Namespace-agnostic core, namespace-aware edges** — Transitional is the primary target.
5. **Binding-ready facade** — `mjx-ooxml` is that surface: concrete types, `u32` indices,
   `&str` part names and eleven stable error codes. The in-workspace `bindings/` members —
   `bindings/mjx-python` (PyO3) and `bindings/mjx-wasm` (wasm-bindgen) — project it and nothing
   below it.
6. **Generate the mechanical, hand-write the meaningful.**
7. **Test-driven, incremental** — always-green increments.

## Settled design decisions

- **In-memory model → Hybrid:** arena/columnar for bulk data (e.g. spreadsheet cells, shared strings),
  owned trees (`Box`/`Vec`) for small structures (paragraphs, runs, shape trees).
- **Raw-bytes retention → Copy-on-write:** keep a part's decompressed bytes until its first mutation
  (re-emit verbatim if untouched); on first edit, serialize from the model and drop the raw bytes.
- **Strings → Interning + `Cow`:** intern hot repeated strings (namespaces, element/attr names, shared
  strings); borrow text from the buffer via `Cow`, own only on edit/unescape.
- **XML:** `quick-xml` at the event level (not serde). **ZIP:** `zip` crate, deflate-only (pure Rust).
  **Errors:** `thiserror` in libraries, `anyhow` only in tooling/tests.
- **De/serialization:** hand-written via the `mjx-derive` macro, every complex type carrying an
  `extra: Vec<RawNode>` unknown-content bucket.

## Round-trip contract

Container ZIP bytes are **not** reproduced identically (deflate parameters vary by encoder). The
guarantee is **per-part decompressed-payload byte identity** + structural container identity (same part
set, content types, relationships).

## Versioning & milestones

Pre-release `v0.0.x`: the patch increments each development iteration until the first milestone; the
public API is not stable until `v0.1`. Milestones advance the minor version — **`v0.1`** PowerPoint,
**`v0.2`** Word, **`v0.3`** Excel — with later milestones (rendering) defined as scheduled.
See [`CHANGELOG.md`](CHANGELOG.md).

## Phases

- **Phase 0 — Skeleton + container + round-trip proof.** ✅ *done.* Workspace, CI, docs,
  `mjx-opc` + minimal `mjx-xml` reader, and `xtask` codegen → `mjx-ooxml-types` (namespaces + shared
  simple types). Opens real `.pptx`/`.docx`/`.xlsx`, enumerates parts/content-types/rels, re-zips with
  per-part byte identity.
- **Phase 1 — Fidelity + MCE.** ✅ *done.* The `mjx-ooxml-core` string interner + `RawElement`
  preservation tree, `mjx-xml`'s byte-preserving fidelity reader/writer (parse→serialize is
  byte-identical on every fixture part), and `mjx-mce` (AlternateContent/Ignorable/ProcessContent/
  MustUnderstand resolve + preserve). *(`mjx-derive` moved to Phase 2, where the first typed model
  gives it concrete consumers.)*
- **Phase 2 — PowerPoint vertical slice.** ✅ *done.* `mjx-derive` + `mjx-dml` + `mjx-pptx`: open a real
  `.pptx`, read slides + shape text, edit a run, add a shape/slide, write a file PowerPoint &
  LibreOffice open (the office-open canary is a CI gate).
- **Phase 3 — DrawingML + PPTX depth.** ✅ *done.* ✅ preset geometry (all 117 adjustable shapes
  named), ✅ color model + theme (`clrScheme`/`fmtScheme`, color resolution to concrete RGB), and the
  ✅ `spPr` visual trilogy — fill, outline (`a:ln`), and effects (`a:effectLst`) — each modeled both
  explicitly and *effectively* (style refs + placeholder inheritance), and ✅ **images** (`add_image`
  media parts, `add_picture` `p:pic` shapes, read/replace — on one shape index space covering every
  shape kind). ✅ **layout/master** — PresentationML simple types, the layout/master inventory,
  `Surface` addressing (every shape API works on a slide, layout, or master, so editing a layout
  reaches every slide that inherits it), and `add_slide_from_layout`, which hands back a slide
  carrying the layout's placeholders ready to fill. ✅ **removal** completes the story — `remove_shape`
  on any surface and `remove_slide`, which unwires the deck and takes with it every part only that
  slide referenced (`Package::remove_part_cascading`).
- **Phase 3b — finishing PowerPoint (→ `v0.1`).** ✅ **text formatting** — `a:rPr`/`a:pPr`, bullets and
  indent levels, editable at four selection scopes, and *effective* resolution up a seven-tier ladder
  ending in the master's `p:txStyles` (`docs/TEXT_FORMATTING_HANDOFF.md`). ✅ **transform** (`a:xfrm`)
  — position, size, rotation and mirror flags, read and written on every shape kind, plus
  `effective_shape_bounds`, so a placeholder's real position resolves through the layout and master
  (`docs/TRANSFORM_HANDOFF.md`). ✅ **tables** (`a:tbl` inside a `p:graphicFrame`) — the model,
  creation, cell text and formatting, selections, merging, rows and columns, the `tableStyles.xml`
  part, inline styles, and effective cell formatting (`docs/TABLES_HANDOFF.md`). ✅ **speaker notes**
  (the notes slide and notes master parts). ✅ the follow-ups each workstream recorded — group
  descent, hyperlinks, run coalescing, `a:br`/`a:fld` addressability, package hygiene,
  external-source neutralisation, custom geometry, 3-D, charts and VML.
- **Phase 3c — the road to `v0.1`.** ✅ **usage documentation** — the five-page guide and six runnable
  examples, so the library documents *tasks* and not only *items*. ✅ **the `mjx-ooxml` facade** —
  `detect_format` over the OPC layer, `Deck` restating the PresentationML surface with concrete
  FFI-expressible types, an `Error` collapsing every `PptxError` into eleven stable codes, and the
  whole authoring vocabulary re-exported so nothing downstream names a lower crate. ✅ **the
  bindings** over that facade — `bindings/mjx-python` (PyO3, `pip install mjx-ooxml`) and
  `bindings/mjx-wasm` (wasm-bindgen, `npm install @mjx/ooxml`), both projecting the whole `Deck`
  surface, both proved by writing the guide's walkthrough a second and third time and checking that
  all three produce byte-identical parts. 🔨 Next, **validation**: every shipped feature checked by
  hand against real PowerPoint, which nothing has yet been.
- **Phase 4 — Word slice.** `mjx-docx` body/styles/tables/sections/numbering/headers + `mjx-omml`.
- **Phase 5 — Excel slice.** **Two** crates, not one: `mjx-sml` holds the SpreadsheetML *markup* —
  cells, rows, shared strings, styles, number formats, formulas as text (no calc engine) — in the
  shared-markup tier at rank 2.1, and `mjx-xlsx` holds the `Workbook` surface and the package graph
  in the format tier. The split exists because an authored chart embeds a whole workbook inside a
  `.pptx` or a `.docx`, so `mjx-chart` needs SpreadsheetML and may only point *downward*: with one
  Excel crate, retiring `mjx-chart`'s duplicate workbook writer would need `mjx-chart → mjx-xlsx`,
  which points up. See `CLAUDE.md`'s rank table and `xtask/tests/layering.rs`.
- **Phase 6 — Charts + VML.** `mjx-chart`; `mjx-vml` (a typed drawing model with shape-level
  references, re-exposed from `mjx-pptx` behind the `vml` feature).
- **Phase 7+ (deferred).** Rendering (IR → text/layout → SVG → raster → PDF).

### Recorded divergence: where the bindings live, and what they are built with

Earlier revisions of this plan (and of `README.md`) said language bindings would live in a
**separate cargo project** on a **UniFFI → wasm → C-ABI** stack targeting Kotlin, Swift, JavaScript
and C, deferred to Phase 7. **They do not.** They are workspace members under `bindings/`, built
directly on PyO3 and wasm-bindgen, with no UniFFI, no napi-rs and no C ABI, and they shipped in
Phase 3c.

Three reasons the decision changed:

* **The library is already binding-shaped.** It is bytes in and bytes out, with no file I/O, no
  clock, no threads, no `getrandom` and no C dependencies, and `wasm32-unknown-unknown` has built
  green in CI since Phase 0. Both binding technologies are a thin veneer over `mjx-ooxml`, not a
  porting layer that deserves its own repository.
* **In-workspace means one truth.** One `cargo test`, one lint policy, one version number, and no
  skew between the facade and its bindings while the API is still moving. A separate project would
  have to track a moving `mjx-ooxml` by git revision.
* **UniFFI was a poor fit for the primary requirement**, which is the browser. It has no wasm
  backend; going through a C ABI to reach JavaScript would have meant hand-writing the glue that
  wasm-bindgen generates, and would have produced structurally-typed objects rather than the
  `.d.ts` classes a TypeScript consumer expects.

Kotlin, Swift and C are not served by this decision. They are not served by the old plan either —
nothing was ever built — and the door is open: a UniFFI member could sit beside the two that exist,
over the same facade, if a caller ever needs one.

## Explicitly out of scope for v1

Bindings for Kotlin, Swift and C (see the recorded divergence above), full-fidelity rendering, a
spreadsheet calculation engine, encrypted/password-protected packages, and digital-signature
processing (preserved, not processed).
