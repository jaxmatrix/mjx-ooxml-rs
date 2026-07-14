# AGENTS.md

Vendor-neutral entry point for coding agents working in **mjx-ooxml-rs**.

The full, authoritative guidance lives in:

- [`CLAUDE.md`](CLAUDE.md) — architecture rules, fidelity rules, the Plan → Plan-Optimization →
  thorough-atomic process, settled implementation choices, and commands.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — the test-driven workflow, fidelity-test tiers, and git/commit
  conventions.
- [`PLAN.md`](PLAN.md) — the phased roadmap and current status.

## The short version

- **Fidelity-first, pure-Rust, cross-platform.** Never corrupt parts we don't model.
- **Layering points downward only.** `quick-xml` only behind `mjx-xml`; ZIP only behind `mjx-opc`.
- **Test-driven & incremental** — write the failing test first; keep every increment green.
- **Atomic commits, no `Co-Authored-By`/AI-attribution trailers.** Setup on `main`, then feature
  branches + PRs.
- **Do the work thoroughly and correctly — no monkey-patching.** Optimize the design (memory, speed,
  reliability) *before* coding.
- **Comprehensive, self-explanatory names.** No cryptic OOXML symbols in public identifiers — expand
  them (`ST_Jc` → `Justification`, `t` → `Top`), source unclear ones from the ECMA-376 prose, and
  keep the exact wire token for (de)serialization. See the naming convention in `CLAUDE.md`.
- `References/` is git-ignored (local-only spec material); test inputs go under `tests/fixtures/`.
