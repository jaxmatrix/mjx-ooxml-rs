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
  | 3.0 — formats | `mjx-pptx`, `mjx-docx`, `mjx-xlsx` |
  | 4.0 — facade | `mjx-ooxml` |
  | 5.0 — bindings | `bindings/mjx-python`, `bindings/mjx-wasm` |
  | 5.5 — platform boundary | `mjx-paint` |
  | — outside the graph | `mjx-fixtures`, `mjx-schema-gate`, `mjx-allocation-counter`, `xtask` |

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
  raise it. `mjx-sml` sits between
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
- **Three test-only crates sit outside that graph:** `mjx-schema-gate` (the shared ECMA-376 schema
  and child-order gate, a `dev-dependency` of the three format crates), `mjx-fixtures` (the committed
  corpus at `tests/fixtures/`, with **no dependencies at all** so `mjx-opc`'s suites can reach it
  without an upward edge) and `mjx-allocation-counter` (the counting global allocator, also with **no
  dependencies at all**, because its two consumers sit in different tiers — `xtask`'s fuzz campaign
  and `mjx-sml`'s allocation gate — and nothing may depend on `xtask`). All three are
  `publish = false` and no shipped crate depends on any of them. A test suite reads its fixture
  corpus from `mjx-fixtures` — never from a `const FIXTURES` list — and installs its allocator from
  `mjx-allocation-counter` rather than writing a second one.
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
  re-emit verbatim; on first edit, serialize from the model and drop raw bytes.
- **Unknown bucket:** every modeled complex type carries `extra: Vec<RawNode>` for unknown children,
  and preserves unknown attributes, attribute order, and namespace prefixes.
- **MCE** (`mc:AlternateContent`/`Ignorable`/`ProcessContent`) is handled in `mjx-mce`, preserved on
  write and resolved (non-mutating) on read/render.
- **Round-trip contract:** per-part decompressed-payload byte identity + structural container identity
  (not identical ZIP bytes).

## Settled implementation choices

- Hybrid model (arena for bulk data, owned trees for small structures).
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
  not documented:** every colour token declares its usage, and the generator refuses a token tagged
  for text that does not reach 4.5 : 1 against its declared background.

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

The acceptance test for both is the same: `crates/mjx-ooxml/examples/build_a_deck.rs` exists a second
time as `bindings/mjx-python/tests/test_build_a_deck.py` and a third as
`bindings/mjx-wasm/tests/node/build_a_deck.mjs`, and each compares its deck against the Rust one
**part by part, byte for byte**. A method wired to the wrong `Deck` method changes one payload and
fails there.

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
