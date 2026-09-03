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

Before and after touching a parser, also run the fuzz campaign — see *The fuzz campaign* below. It is
deliberately **not** on the CI push path.

CI runs these in the `fmt · clippy · test` job, plus:

| Job | What it blocks on |
|---|---|
| `rustdoc` | a strict `cargo doc --workspace --no-deps` — missing docs, broken intra-doc links |
| `schema-validity (ECMA-376 XSDs)` | every fixture part and every deck the library authors validating against the ECMA-376 XSDs (see *The schema gate* below) |
| `office-open (LibreOffice)` | a deck we construct opening in a real Office implementation |
| `examples` | every example running and re-opening its own output |
| `build <target>` | a cross-compile matrix (wasm32, Android, iOS, macOS, Windows) |

Red, a clippy warning, or a doc warning blocks merge.

The sample round-trip tests are the fastest confirmation that real files still parse:
`cargo test -p mjx-opc --test roundtrip` and `cargo test -p mjx-opc --test tree_roundtrip` open the
`.pptx`/`.docx`/`.xlsx` fixtures and assert byte-identical round-trips. See the README's *Testing*
section.

### The schema gate

Round-tripping proves we do not corrupt a file, and the LibreOffice canary proves a deck *opens* —
neither proves the markup we *write* is legal. Before touching an authoring path, run

```sh
MJX_REQUIRE_SCHEMA=1 cargo test -p mjx-pptx --test schema_validity
```

which validates every fixture part and every deck the library authors against the ECMA-376 Part 4
Transitional and Part 2 OPC schemas via `xmllint`. It needs the reference schemas in the git-ignored
`References/` tree (or `MJX_SCHEMA_DIR` / `MJX_OPC_SCHEMA_DIR`) and skips cleanly without them. To
populate the tree — the same script the CI job runs, downloading the two published ECMA archives and
verifying them against `.github/ecma-376-archives.sha256` before extracting:

```sh
.github/scripts/fetch-ecma-schemas.sh
```

The `schema-validity (ECMA-376 XSDs)` CI job sets `MJX_REQUIRE_SCHEMA=1`, so in CI a missing schema or
a missing `xmllint` is a hard failure and this coverage can never silently skip. **A new authoring
path gets a case in that file**, or nothing checks the markup it emits.

### The fuzz campaign

`cargo test` proves the parsers do the right thing with the files we have. It says nothing about the
files an attacker has. **Every input this library reads is untrusted**, and the rule in `CLAUDE.md` —
no `unwrap`/`panic`/`expect` on untrusted input — needs something that actually tries to break it:

```sh
cargo run -p xtask -- fuzz                      # every target, ~200k executions each
cargo run -p xtask -- fuzz --list               # what the targets are
cargo run -p xtask -- fuzz --target opc-container --seconds 300
cargo run -p xtask -- fuzz --seed 42            # a campaign is reproducible from its seed
```

**It is not on the CI push path, and should not be.** A campaign has no natural end, and a run short
enough for a pull request is a run that finds nothing. Run it before touching a parser, after
changing one, and when a release is being prepared.

It is a stable-Rust harness with no new dependency, deliberately: `cargo-fuzz` needs a nightly
toolchain for its sanitizer flags, and a gate only some machines can run is not a gate. Everything it
knows lives in `xtask/src/fuzz/` — `xtask` is host-only and nothing depends on it, so the harness
cannot leak into the shipped graph.

What it checks is stronger than "did not crash":

| | |
|---|---|
| **Panics** | caught per execution, so one bad input does not end the run |
| **The round-trip oracle** | every input the reader *accepts* must re-serialize byte-for-byte |
| **The verbatim-span oracle** | the same corpus with the root dirtied, where a bad byte range shows |
| **Part byte identity** | a package written back and reopened holds the same part bytes |
| **Unbounded allocation** | a counting allocator measures each execution's peak against a ceiling |
| **Hangs** | a watchdog thread aborts rather than let a hang read as a slow campaign |

Read the report's **corpus** and **behaviours** columns, not just **execs**. A campaign that explored
nothing prints a clean run exactly like one that explored everything; growth in those two columns is
what tells them apart.

A finding writes its input to `target/fuzz/findings/`, and the input being executed is always in
`target/fuzz/in-flight.bin` — which is what names the culprit when a hang or the hard memory ceiling
takes the process down without unwinding.

**Every finding becomes a committed regression test in the crate that owns the path**, asserting the
property rather than the error that happened to come back. See `crates/mjx-xml/tests/untrusted_input.rs`,
`crates/mjx-opc/tests/untrusted_input.rs` and `crates/mjx-mce/tests/untrusted_input.rs`.

**A finding is never fixed by loosening what the parser accepts.** That trades a crash for a
corruption, which is the one thing this project exists to prevent. Every fix so far has been a
tightening.

Adding a target is a `Target` literal and one function in `xtask/src/fuzz/targets.rs`; the mutation
loop, the corpus, the ceilings and the crash log are shared.

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

## Naming convention (comprehensive, self-explanatory identifiers)

OOXML symbols are cryptic; our public API must not be. Applies to generated *and* hand-written types.

- Type names drop `ST_`/`CT_`, expand abbreviations, and are module-namespaced per schema
  (`wml::Justification`, never `Jc`). Variant/field names expand cryptic tokens (`t` → `Top`,
  `dist` → `Distributed`).
- When a token's meaning is not clear from the symbol, **source the name from the ECMA-376 Part 1
  prose** — never guess.
- The exact XSD wire token is preserved for (de)serialization and shown in the item's docs alongside
  its original `ST_*` symbol. Two-valued types are `bool`/`Option<bool>` with all wire spellings
  normalized on read (see `mjx-ooxml-types::support`).
- The generator (`xtask/src/codegen/`) applies this via curated tables in `spec.rs`; extending it to a
  new schema means growing those tables. See the full convention in `CLAUDE.md`.

## Code style

- Pure-Rust dependencies only in shipped crates. `unsafe` is denied workspace-wide; if genuinely
  required, `#[allow(unsafe_code)]` locally with a written safety justification.
- No `unwrap`/`expect`/`panic` on untrusted input in library paths — return typed `thiserror` errors.
- Respect the layering: dependencies point downward only (see `CLAUDE.md`).
