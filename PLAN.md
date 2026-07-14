# mjx-ooxml-rs — Roadmap

The living, public roadmap. For the deep architecture rationale, see the design decisions summarized
below and in `CLAUDE.md`.

## Objective

A pure-Rust, cross-platform library that can **open any OOXML file, load it fully into RAM, edit it at
runtime, and write back a valid file** — preserving everything it does not explicitly model — for
PowerPoint, Word, and Excel. Rendering and language bindings come later.

## Guiding principles

1. **Fidelity-first** — never corrupt parts/elements/attributes we do not understand.
2. **Pure-Rust only** in the shipped graph (clean wasm / Android / iOS cross-compilation).
3. **Lazy, part-oriented** — parts are raw bytes until touched; untouched parts re-emit verbatim.
4. **Namespace-agnostic core, namespace-aware edges** — Transitional is the primary target.
5. **Binding-ready facade** — a separate project will add bindings over `mjx-ooxml` later.
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

## Phases

- **Phase 0 — Skeleton + container + round-trip proof.** 🚧 *in progress.* Workspace, CI, docs,
  `mjx-ooxml-core` / `mjx-xml` / `mjx-opc`, and `xtask` codegen → `mjx-ooxml-types`. Exit: open real
  `.pptx`/`.docx`/`.xlsx`, enumerate parts + content types + rels, re-zip with per-part byte identity.
- **Phase 1 — Fidelity + MCE.** `mjx-derive`, the `RawElement` preservation tree, attribute-order /
  prefix / whitespace preservation, and `mjx-mce` (AlternateContent resolution).
- **Phase 2 — PowerPoint vertical slice.** Minimal `mjx-dml` + `mjx-pptx`: open a real `.pptx`, read
  slides + shape text, edit a run, add a shape/slide, write a file PowerPoint & LibreOffice open.
- **Phase 3 — DrawingML + PPTX depth.** Images, preset geometry, color model, theme, layout/master.
- **Phase 4 — Word slice.** `mjx-docx` body/styles/tables/sections/numbering/headers + `mjx-omml`.
- **Phase 5 — Excel slice.** `mjx-xlsx` workbook/sheets/shared-strings/styles; formulas as text (no
  calc engine).
- **Phase 6 — Charts + VML.** `mjx-chart`; `mjx-vml` (feature-gated, preserve-first).
- **Phase 7+ (deferred).** Rendering (IR → text/layout → SVG → raster → PDF); and, in a **separate
  cargo project**, language bindings (UniFFI → wasm → C-ABI).

## Explicitly out of scope for v1

Language bindings (separate project), full-fidelity rendering, a spreadsheet calculation engine,
encrypted/password-protected packages, and digital-signature processing (preserved, not processed).
