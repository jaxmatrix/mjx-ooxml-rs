# Contributing to mjx-ooxml-rs

This project is built **deliberately, test-first, and incrementally**. The app grows in small,
always-green steps — never a large untested drop, never a shortcut we intend to "fix later".

## The development loop (TDD)

Every change follows **red → green → refactor**:

1. **Red** — write a failing test first. Prefer a *fidelity* test: a round-trip assertion, a parse
   expectation against a fixture, or an edit-isolation check.
2. **Green** — write the minimum code to make it pass.
3. **Refactor** — clean it up with the tests still green.

Before writing code for a non-trivial piece, do the **Plan → Plan-Optimization** step: decide the
design and *optimize it for memory, speed, and reliability first* (allocations, copies, cache, failure
modes). We prefer the correct design over the merely-working one. See `CLAUDE.md`.

## Fidelity-test tiers

1. **Pass-through parts** — a part we do not model must re-serialize to **byte-identical** decompressed
   bytes.
2. **Modeled parts** — parse → serialize → parse must be equal under a canonicalized-XML comparison
   (insignificant whitespace / prefix noise normalized).
3. **Edit isolation** — change exactly one thing; assert every *other* part is byte-identical.

Round-trip contract: **per-part decompressed-payload byte identity** + structural container identity
(NOT identical ZIP bytes — deflate parameters vary by encoder).

## Adding a new modeled element

1. Add a real fixture under `tests/fixtures/` (never read from the git-ignored `References/`).
2. Write a **failing** round-trip / parse test against it.
3. Model the type via `#[derive(FromXml, ToXml)]`, including an `extra: Vec<RawNode>` unknown-content
   bucket so unmodeled siblings still round-trip.
4. Make it green; verify the edit-isolation tier still holds.

## Required checks (must be green before every commit)

```sh
cargo fmt --all
cargo build  --workspace
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs these plus a cross-compile *build* matrix (wasm32, Android, iOS, desktop). Red or a clippy
warning blocks merge.

## Git & commit conventions

- **Atomic commits** — one self-contained change per commit, so history is easy to roll back and
  cherry-pick. Split unrelated changes.
- **Commit only when green** — a test is committed with or before the code it covers.
- **No `Co-Authored-By` or AI-attribution trailers.** Keep messages plain (imperative subject, optional
  body explaining *why*). Conventional-commit-style prefixes are encouraged: `feat(opc): …`,
  `fix(pptx): …`, `chore: …`, `docs: …`, `test: …`, `refactor: …`.
- **Branching:** project-setup commits go directly on `main`. Once feature development begins, create a
  **feature branch** and consolidate via a **pull request**; `main` stays the integration branch.
- **Never stage `References/`** (it is git-ignored) — test inputs belong in `tests/fixtures/`.

## Code style

- Pure-Rust dependencies only in shipped crates. `unsafe` is denied workspace-wide; if genuinely
  required, `#[allow(unsafe_code)]` locally with a written safety justification.
- No `unwrap`/`expect`/`panic` on untrusted input in library paths — return typed `thiserror` errors.
- Respect the layering: dependencies point downward only (see `CLAUDE.md`).
