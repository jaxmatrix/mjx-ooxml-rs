# mjx-ooxml-rs

A **pure-Rust** library to **parse, edit, generate, and (later) render** Office Open XML (OOXML)
documents — PowerPoint (`.pptx`), Word (`.docx`), and Excel (`.xlsx`).

The goal: open *any* OOXML file, load it fully into RAM, operate on it at runtime, and write back a
valid file **without corrupting the parts you did not touch** — with a codebase that cross-compiles
cleanly to desktop, Android, iOS, and WebAssembly for use inside Tauri and beyond.

> **Status:** pre-release `v0.0.x`. The packaging, byte-fidelity, and Markup-Compatibility core and
> the schema-type generator are implemented and tested; the format models are being built
> **PowerPoint first** — milestones `v0.1` = PowerPoint, `v0.2` = Word, `v0.3` = Excel. The public API
> is not stable until `v0.1`. See [`PLAN.md`](PLAN.md) and [`CHANGELOG.md`](CHANGELOG.md).

## Why another OOXML library?

- **Fidelity-first.** Unknown parts, unknown elements/attributes, namespace prefixes, attribute order,
  and Markup-Compatibility (`mc:`) constructs are all preserved, so round-tripping a real-world file
  keeps untouched content byte-for-byte intact.
- **Pure Rust, cross-platform.** No C/system libraries in the shipped dependency graph, so
  `wasm32-unknown-unknown`, `aarch64-linux-android`, and `aarch64-apple-ios` build cleanly.
- **Unified model.** One packaging + compatibility + DrawingML core shared across all three formats,
  rather than three unrelated libraries.
- **Binding-ready.** The public API is designed so a *separate* project can later add language
  bindings (Kotlin/Swift/JS/C) over a stable surface.

## Format support

| Format | Crate | Status |
|---|---|---|
| PowerPoint `.pptx` | `mjx-pptx` | 🚧 first target |
| Word `.docx` | `mjx-docx` | ⏳ planned |
| Excel `.xlsx` | `mjx-xlsx` | ⏳ planned |

Rendering (document viewer) and language bindings are **deferred** — see [`PLAN.md`](PLAN.md).

## Workspace layout

Layered Cargo workspace; dependencies only ever point *downward*.

```
Foundations     mjx-ooxml-core  ·  mjx-xml  ·  mjx-derive
Packaging/compat mjx-opc  ·  mjx-mce  ·  mjx-ooxml-types (generated)
Shared markup   mjx-dml  ·  mjx-omml  ·  mjx-chart  ·  mjx-vml
Formats         mjx-pptx  ·  mjx-docx  ·  mjx-xlsx
Facade          mjx-ooxml   (open()/save(), the binding-ready public API)
Tooling         xtask       (schema codegen)
```

See [`PLAN.md`](PLAN.md) for what each crate does and the phase it lands in.

## Building

```sh
cargo build --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The ECMA-376 reference schemas live under `References/` (git-ignored, local-only). They are read by
`xtask` to regenerate `mjx-ooxml-types`; the generated source is committed, so normal builds and CI do
**not** need `References/` present.

## Testing

```sh
cargo test --workspace                       # everything
cargo test -p mjx-opc --test roundtrip       # OPC container: open → save → reopen, per-part byte identity
cargo test -p mjx-opc --test tree_roundtrip  # fidelity tree: every XML part re-serializes byte-identical
```

The sample files under [`tests/fixtures/`](tests/fixtures) — a real LibreOffice `.docx` and `.xlsx`
plus a structurally-complete `.pptx` — are the current confirmation that parsing works. As of the
Phase 1 core, **all three parse without failure**: `tree_roundtrip` runs every `.xml`/`.rels` part of
all three files (20+ parts) through the fidelity reader/writer and asserts **byte-for-byte** identity,
and `roundtrip` re-zips each package with per-part byte identity. A broader multi-producer corpus and
fuzzing come in a later iteration.

## Documentation

```sh
cargo doc --workspace --no-deps --open   # start at the `mjx-ooxml` crate — the docs hub
```

Every public item is documented; the `missing_docs` lint and a strict rustdoc CI job keep it that way.

## Contributing

Development is **test-driven** and **incremental** — see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the
red→green→refactor loop, the fidelity-test tiers, and the git/commit conventions.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
