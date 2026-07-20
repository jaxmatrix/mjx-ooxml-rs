# Changelog

All notable changes to **mjx-ooxml-rs** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning

The project is pre-release and uses `v0.0.x`: the patch number is incremented each development
iteration until the first milestone. Milestones then advance the minor version:

- **`v0.1`** — PowerPoint (`.pptx`) complete
- **`v0.2`** — Word (`.docx`) complete
- **`v0.3`** — Excel (`.xlsx`) complete

Further milestones (rendering, bindings, …) are defined as that work is scheduled. The public API is
**not** stable until `v0.1`.

## [0.0.2] - 2026-07-20

The PowerPoint slice — Phases 2 and 3. A real `.pptx` can now be opened, read, edited, built up from
its own layouts and pruned back down, and written out so PowerPoint and LibreOffice open it with every
untouched part byte-identical. Phase 3 closes here; Word (Phase 4) is next.

### Added

- **De/serialization (Phase 2)** — `FromXml`/`ToXml` in `mjx-ooxml-core::convert` and the
  `#[derive(FromXml, ToXml)]` proc-macro in `mjx-derive`. Every modeled type keeps an unknown-content
  bucket, so what we do not model survives a round trip.
- **DrawingML text (Phase 2)** — `mjx-dml`'s `TextBody`/`Paragraph`/`TextRun`/`Text`, with a mutation
  surface.
- **PresentationML (Phase 2)** — `mjx-pptx::Presentation`: `open`/`save`, slide inventory, shape
  enumeration, `shape_text`/`set_shape_text`, and construction — `add_text_box`, `add_shape`,
  `add_slide`. The **office-open canary** (LibreOffice headless must render the produced deck to a
  valid PDF) became a CI gate.
- **Preset geometry (Phase 3)** — all 187 `ST_ShapeType` values generated, and the 117 adjustable
  shapes given **named, spec-sourced control parameters** (a rounded rectangle exposes
  `corner_radius`, never `adj1`), with the meaning derived from `presetShapeDefinitions.xml`.
- **Color, theme and the `spPr` visual trilogy (Phase 3)** — theme (`clrScheme`/`fmtScheme`) with
  color resolution to concrete RGB, and **fill**, **outline** (`a:ln`) and **effects**
  (`a:effectLst`), each modeled both *explicitly* and *effectively* — resolved through style
  references and placeholder inheritance to what actually renders.
- **Images (Phase 3)** — `add_image` media parts (de-duplicated by content, format identified by
  magic bytes), `add_picture` `p:pic` shapes, and picture read/replace — on one shape index space
  covering every shape kind.
- **Layouts and masters (Phase 3)** — the layout/master inventory, generated PresentationML simple
  types, **`Surface` addressing** (every shape call works on a slide, a layout or a master, so editing
  a layout reaches every slide inheriting it), and `add_slide_from_layout`, which returns a slide
  carrying the layout's placeholders ready to fill.
- **Removal (Phase 3)** — `remove_shape` on any surface, and `remove_slide`, which unwires
  `p:sldIdLst` → relationship → part and takes with it every part only that slide referenced (its
  notes slide, unshared media) while sparing anything the rest of the deck still uses.
- **Packaging** — `Package::{insert_part, remove_part, remove_part_cascading,
  set_content_type_default/override, add_relationship, remove_relationship}` over a copy-on-write part
  body, plus `PartName::{resolve, resolve_from_root, relative_target}` — the part-name algebra Word
  and Excel will share.

### Fixed

- `add_shape` / `add_text_box` built a paragraph with no run, so the shape they returned could not be
  filled by `set_shape_text`. Every paragraph they create now holds exactly one run, blank lines
  included.
- `add_slide_from_layout` cloned the date, footer and slide-number placeholders. Those render *from
  the layout* for slides that do not declare them, so the clones suppressed the layout's rendering and
  showed as empty boxes; they are now skipped, as PowerPoint does.

### Notes

- The round-trip contract is unchanged and continuously asserted: per-part decompressed-payload byte
  identity plus structural container identity. Reading dirties nothing; an edit re-serializes only its
  own part.
- Public API remains unstable until `v0.1`.

## [0.0.1] - 2026-07-15

First versioned snapshot. Establishes the workspace, the packaging + fidelity + compatibility core,
the schema-type generator, and full documentation. No format models yet.

### Added

- **Packaging (Phase 0)** — `mjx-opc`: load an OOXML package fully into RAM as an ordered part graph,
  parse `[Content_Types].xml` and `_rels/*.rels`, and re-zip with per-part decompressed-byte identity.
  Minimal namespace-resolving reader in `mjx-xml`.
- **Schema codegen (Phase 0)** — `xtask` generates `mjx-ooxml-types` (namespace table +
  `shared-commonSimpleTypes`) with comprehensive, self-explanatory names and exact wire tokens;
  output is deterministic and committed.
- **Fidelity layer (Phase 1)** — `mjx-ooxml-core` string interner + the `RawDocument` preservation
  tree, and `mjx-xml::fidelity`, a byte-preserving reader + hand-written writer. Parsing then
  re-serializing any part reproduces the source **byte-for-byte** (verified on real `.pptx`/`.docx`/
  `.xlsx` fixtures).
- **Markup Compatibility (Phase 1)** — `mjx-mce`: preserve mode (the untouched tree) and a
  non-mutating resolve mode (`AlternateContent` Choice/Fallback, `Ignorable`, `ProcessContent`,
  `MustUnderstand`).
- **Documentation** — comprehensive rustdoc across all crates (crate guides + runnable examples), a
  facade docs hub (`mjx-ooxml`), enforced via `missing_docs` and a strict-rustdoc CI job.
- **Project** — CI (fmt/clippy/test + wasm/Android/iOS/macOS/Windows cross-compile build matrix),
  dual `MIT OR Apache-2.0` license, and the contributor/agent guides.

### Notes

- Cross-platform: pure-Rust dependency graph; the library crates cross-compile to
  `wasm32-unknown-unknown`, `aarch64-linux-android`, and Apple/Windows targets.
- A broader multi-producer sample corpus and fuzzing are planned for later iterations.

[0.0.2]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.2
[0.0.1]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.1
