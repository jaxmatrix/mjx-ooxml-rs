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

## Contributing

Development is **test-driven** and **incremental** — see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the
red→green→refactor loop, the fidelity-test tiers, and the git/commit conventions.

## License

**mjx-ooxml-rs is free to use, modify, and distribute** — including in commercial and closed-source
projects — under the permissive **[MIT License](LICENSE-MIT)**. It is offered as `MIT OR Apache-2.0`,
so you may use it under either the [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) license, at your
option. The only condition is the usual one: keep the copyright and license notice with copies.

Unless you explicitly state otherwise, any contribution you submit for inclusion is licensed the same
way, with no additional terms.
