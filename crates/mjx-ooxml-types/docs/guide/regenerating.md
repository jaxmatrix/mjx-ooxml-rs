# Regenerating

```sh
cargo run -p xtask -- codegen           # rewrite every artefact
cargo run -p xtask -- codegen --check   # write nothing; say whether the committed output is current
```

## What you need

Three trees under a git-ignored `References/` at the repository root, and `rustfmt`:

| What | Where the generator looks | Which ECMA-376 part |
|---|---|---|
| Transitional XSDs | `References/ECMA-376-4_…/OfficeOpenXML-XMLSchema-Transitional/` | **Part 4** |
| Strict XSDs (for the namespace pairing) | `References/ECMA-376-1_…/OfficeOpenXML-XMLSchema-Strict/` | **Part 1** |
| `presetShapeDefinitions.xml` | `References/ECMA-376-1_…/OfficeOpenXML-DrawingMLGeometries/` | **Part 1** |

`codegen::references_are_present` is the predicate, and the three constants at the head of
`xtask/src/codegen/mod.rs` are the paths. `rustfmt` is shelled out to for every `.rs` artefact, so
the committed bytes are formatted source rather than something a later `cargo fmt` would rewrite.

**`References/` is git-ignored by a standing rule of this repository** — whether the ECMA-376 files
may be redistributed inside the tree is the repository owner's decision, not an agent's — so this is
a local operation. `.github/scripts/fetch-ecma-schemas.sh` downloads what CI needs, pinned by
SHA-256.

## Why the output is committed rather than built

`CLAUDE.md` decides it: *generated `mjx-ooxml-types` … output is committed, never a `build.rs`*. The
reasons are good ones.

* A `build.rs` would make every consumer's build depend on a **5,000-page specification** that is not
  redistributable, is not in the tree, and would have to be downloaded.
* It would put an XSD parse and a `rustfmt` invocation of 84,107 lines on the critical path of every
  clean build, in every downstream crate, forever.
* The output is the thing people read. `crates/mjx-ooxml-types/src/generated/wordprocessingml.rs` is
  browsable, greppable and diffable in review; a `build.rs` artefact under `target/` is none of
  those.
* Diffs are the review surface. A naming change shows up as a diff of names, which is exactly the
  thing a reviewer should be looking at.

## What that costs, and what MJXOFF-224 did about it

The cost is one sentence, and until MJXOFF-224 it was written down nowhere:

> **Nothing re-derived the committed output, so a generator defect was frozen into the repository
> rather than failing on the next build — and the committed file is the only artefact anyone reads,
> which makes a defect indistinguishable from a deliberate choice.**

The compiler catches the structural half of this and no more. Rename a generated enum by hand and
`mjx-ooxml-types` stops compiling. Change a wire token, a rank in a child-order table, a doc comment
recording an `ST_*` symbol, or a row of `COVERAGE.md`, and nothing anywhere notices — those are
precisely the parts a reader trusts and no build touches.

`xtask/tests/codegen_drift.rs` is the answer, in two tiers because only one of them can run
everywhere:

| | What it does | Where it runs |
|---|---|---|
| `the_committed_output_is_what_the_generator_produces_today` | regenerates all thirteen artefacts in memory and compares them byte for byte with what is committed | **only where `References/` is complete**; skips otherwise, and `MJX_REQUIRE_CODEGEN=1` makes the absence a failure |
| the five tests beside it | re-derive the module set, the file set, the `@generated` banners, `COVERAGE.md`'s counts and child-order rows, and the curated re-exports — **from the generator's own tables and the committed files, with no schema at all** | every push |

`cargo run -p xtask -- codegen --check` is the same first check as a command, and it prints the
artefact list and the verdict.

**The obstacle, stated plainly.** CI cannot run tier 1 today, and not because of a missing switch:
`.github/scripts/fetch-ecma-schemas.sh`'s `ARCHIVES` holds ECMA-376 **Part 4** (Transitional schemas)
and **Part 2** (OPC schemas) — and this generator needs **Part 1** for two of its three inputs. The
schema-validity job therefore has a `References/` tree that codegen cannot run against. Growing that
list is MJXOFF-197's, which owns the same download for `crates/mjx-dml/tests/guide_formula.rs`'s
preset-geometry sweep; one archive unlocks both. Until then, tier 1 runs where a developer has the
spec, and tier 2 is what stands between two runs of it.

## Landing a generator change

A change to `xtask/src/codegen/` and the regenerated files **must land in the same commit**. There is
no build step that would reconcile them later, and the reviewer of a generator diff with no output
diff has no way to see what the change did. In order:

1. Edit the generator.
2. `cargo run -p xtask -- codegen`.
3. `git diff --stat crates/mjx-ooxml-types` — and read it. An unexpected file in that list is the
   finding, not a nuisance.
4. `cargo test --workspace`, which includes `crates/mjx-ooxml-types/tests/wire.rs`,
   `crates/mjx-ooxml-types/tests/adjustments.rs` and `xtask/tests/codegen_drift.rs`.
5. Commit the generator and the output together.

## The generator refuses rather than guesses

Every one of these is a hard error, and each is here because the alternative is a silent loss:

* two emitted types reaching the same Rust name, or two values of one enumeration reaching the same
  variant (`emit::emit_types`);
* a naming-override row that matched nothing (`spec::unused_overrides`);
* an unresolved `xsd:group ref`, or a slot whose element lands in a namespace no parsed schema
  declares (`complex::SchemaSet::group`, `child_order::render_table`);
* a schema in the Transitional set with neither coverage nor an `UNCOVERED_SCHEMAS` row;
* an `UNCOVERED_SCHEMAS` row, or one of its two notes, that the tables have made **unreachable** —
  `check_uncovered_schemas_are_live`, added by MJXOFF-224. `UNCOVERED_SCHEMAS` writes prose straight
  into a shipped document, and before that check a row could keep asserting *not modelled* about a
  schema the generator had started generating, with nothing to say so. What the check enforces is
  the rule the table's own doc comment already stated and nothing tested: **a schema that gains
  coverage in both tables has its row removed, and a note is written only for the column that
  actually needs one.** Three rows (`pml`, `wml`, `dml-main`) and one note (`dml-chart`'s
  child-order note, which said `generated — every complex type` about a column computed elsewhere)
  were already dead when the check was written.
