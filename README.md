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

## Quickstart

```rust
use mjx_pptx::{Presentation, ShapeBounds};

let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;

let slide = deck.add_slide_from_layout(1)?;          // placeholders, ready to fill
deck.set_shape_text_content(slide, 0, "Quarterly results")?;
deck.add_picture(slide, &std::fs::read("logo.png")?, ShapeBounds::from_inches(7.5, 0.3, 1.5, 1.5))?;

std::fs::write("out.pptx", deck.save()?)?;
```

`open` takes bytes and `save` returns bytes — the library never touches a filesystem, a network or a
clock, which is why the same code cross-compiles to WebAssembly and runs in a browser. Every part you
did not touch comes back byte-for-byte as it arrived.

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

# Schema validity: every fixture part and every deck the library authors, against the ECMA-376 XSDs.
# Needs `xmllint` and the reference schemas; skips cleanly without them, and MJX_REQUIRE_SCHEMA=1
# turns their absence into a failure.
MJX_REQUIRE_SCHEMA=1 cargo test -p mjx-pptx --test schema_validity
```

The sample files under [`tests/fixtures/`](tests/fixtures) — a real LibreOffice `.docx` and `.xlsx`
plus a structurally-complete `.pptx` — are the current confirmation that parsing works. As of the
Phase 1 core, **all three parse without failure**: `tree_roundtrip` runs every `.xml`/`.rels` part of
all three files (20+ parts) through the fidelity reader/writer and asserts **byte-for-byte** identity,
and `roundtrip` re-zips each package with per-part byte identity. A broader multi-producer corpus and
fuzzing come in a later iteration.

Round-tripping proves we do not *corrupt* a file; it does not prove the markup we *write* is legal, and
neither does the LibreOffice canary — LibreOffice opens invalid markup happily. `schema_validity`
closes that: it validates every fixture's PresentationML / DrawingML / chart parts and both OPC control
streams, plus every deck the library authors, against the ECMA-376 Part 4 Transitional and Part 2 OPC
schemas. The schemas are not committed (`References/` is git-ignored), so it is a **local gate** for
now.

## Documentation

```sh
cargo doc --workspace --no-deps --open   # start at the `mjx-ooxml` crate — the docs hub
```

Every public item is documented; the `missing_docs` lint and a strict rustdoc CI job keep it that way.

### Guides

Longer-form prose lives beside the code, and renders as its own pages under `cargo doc`. Every code
snippet in them is compiled as a doctest, so none of it can rot.

| Guide | What it covers |
|---|---|
| [Building a deck](crates/mjx-pptx/docs/guide/building_a_deck.md) | The whole story once: open, add slides, fill them, style them, save |
| [Shapes and text](crates/mjx-pptx/docs/guide/shapes_and_text.md) | The one shape index space, group descent, surfaces, the four text scopes, the edit cursor |
| [Tables, charts and pictures](crates/mjx-pptx/docs/guide/tables_charts_pictures.md) | Structured content, cell selections, merging, chart authoring, linked media |
| [Inheritance, layouts and masters](crates/mjx-pptx/docs/guide/inheritance_and_masters.md) | Where a property comes from when the slide does not state it |
| [Effective properties](crates/mjx-pptx/docs/effective_properties.md) | The deep reference: every inheritance ladder, why colours bake to `RRGGBB`, where each reader stops |
| [Fidelity and the known gaps](crates/mjx-pptx/docs/guide/fidelity_and_gaps.md) | The round-trip guarantee, and an honest list of what is not modelled |

### Examples

Six runnable programs. Each one reopens what it wrote and asserts something about it, and CI runs all
six on every push.

```sh
cargo run -p mjx-pptx --example build_a_deck -- out.pptx   # the guide, end to end
cargo run -p mjx-pptx --example read_deck -- deck.pptx     # inspect, changing nothing
cargo run -p mjx-pptx --example edit_text                  # and report which parts changed
cargo run -p mjx-pptx --example style_shapes
cargo run -p mjx-pptx --example build_table
cargo run -p mjx-pptx --example charts_and_media
```

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
