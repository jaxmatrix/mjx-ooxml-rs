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

- **Layering:** dependencies point **downward only**. Foundations (`mjx-ooxml-core`, `mjx-xml`,
  `mjx-derive`) → packaging/compat (`mjx-opc`, `mjx-mce`, `mjx-ooxml-types`) → shared markup
  (`mjx-dml`, `mjx-omml`, `mjx-chart`, `mjx-vml`) → formats (`mjx-pptx`, `mjx-docx`, `mjx-xlsx`) →
  facade (`mjx-ooxml`). Never introduce an upward or sideways dependency.
- **Pure-Rust only** in shipped crates — no C/system libs. C tools (`xmllint`, LibreOffice) are for
  CI/tests only. `quick-xml` lives *only* behind `mjx-xml`; the ZIP backend *only* behind `mjx-opc`.
- **`unsafe_code = "deny"`** workspace-wide; a crate that truly needs it (arena/interner) must
  `#[allow(unsafe_code)]` locally **with a written safety justification**.
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

## Commands

```sh
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- codegen        # regenerate mjx-ooxml-types from References/ (local only)
```

## Git / commits

- **Project-setup commits go on `main`;** once features start, **branch per feature + open a PR**.
- **Atomic commits** (one self-contained change, easy rollback/cherry-pick); commit only when
  `cargo build` + `cargo test --workspace` are green.
- **Do NOT add `Co-Authored-By` or any AI-attribution trailer** to commits.
- `References/` is git-ignored — never stage it; put test inputs under `tests/fixtures/`.
