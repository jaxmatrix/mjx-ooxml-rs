# What to distrust

This is the honest page for a crate that, for most of its life, nothing re-derived. Every other page
here says what is true; this one says how each of those truths is held up, and where the prop is
missing. It is written the way `crates/mjx-sml/docs/guide/fidelity_and_gaps.md` and
`crates/mjx-chart/docs/guide/fidelity_and_gaps.md` are written, and for the same reason: an
undocumented gap is worse than a documented one.

## The shape of the problem

Three facts compose, and each is individually reasonable:

1. The generated source is **committed, not built** — deliberately, for the reasons in
   [Regenerating](regenerating).
2. `References/` is **git-ignored**, so most machines cannot regenerate at all.
3. **CI never downloads ECMA-376 Part 1**, which is two of the generator's three inputs.

Together they mean the generator and its output were free to disagree for as long as this crate has
existed, and the only artefact anyone reads is the output. `xtask/tests/codegen_drift.rs` closes most
of that, but not by CI — read the table below for exactly how much.

**One reassurance, and it is a real one.** At `0.0.141`, on a machine with the full `References/`
tree, `cargo run -p xtask -- codegen` produces **no diff at all** against the committed output — all
thirteen artefacts, 2,626,921 bytes, byte for byte. That is the first time in this repository's
history the question has been asked, and the answer was clean.

## What each gate catches, and what it cannot

| Gate | Catches | Cannot catch | Runs |
|---|---|---|---|
| `the_committed_output_is_what_the_generator_produces_today` | any difference between the committed bytes and today's generator — including a hand-edit that still compiles | whether the **generator** is right; it compares the output to its producer, not to ECMA-376 | only where `References/` is complete |
| the five schema-free tests beside it | a module, file, banner, `COVERAGE.md` count, child-order row or curated re-export that has drifted from the generator's tables | anything that needs a schema to know | every push |
| `crates/mjx-ooxml-types/tests/wire.rs` | a wire token that does not round-trip, and two variants colliding on one token | only the types it names — it is a hand-written roster, not a sweep of all 360 | every push |
| `crates/mjx-ooxml-types/tests/adjustments.rs` | the preset-shape tables against hand-checked facts | the other 180-odd shapes | every push |
| `crates/mjx-ooxml-types/src/child_order.rs`'s own suite | rank order, the unordered-type safety property, and the content-model census, over **all nine** tables | whether a rank matches the XSD — that is the generator's job, checked by the gate above | every push |
| `mjx_schema_gate::categories`' `the_declared_owners_agree_with_the_generated_coverage_document` | the gate's own ownership claims disagreeing with `COVERAGE.md` | the prose in a `COVERAGE.md` row | every push |
| `spec::unused_overrides` | a naming-override row naming a type or wire value the schema does not declare | whether the **name** the row chose is right | only when the generator runs |

## Four things nothing here checks

**1 · Whether a curated name is the name ECMA-376 gives.** The audit trail is the `§` citation
required on every variant-override row, and it is a trail for a person: nothing in this workspace
reads the specification's prose, which ships as a PDF outside the tree. An audit against that PDF for
MJXOFF-224 found two places where the trail leads somewhere a reader should know about, and both are
recorded here rather than changed, because renaming a generated type is an API break that belongs to
its own ticket:

* **`ST_PresetShadowVal`'s twenty variants are `Shadow1` … `Shadow20`**, on a comment saying the
  tokens have *"no semantic name"*. §20.1.10.52 names all twenty — `shdw1` is *Top Left Drop Shadow*,
  `shdw3` is *Back Left Perspective Shadow*, `shdw11` is *Back Left Long Perspective Shadow*. The
  names are faithful and unambiguous, but they are not self-explanatory, which is what this crate is
  for, and the justification beside them is factually wrong.
* **`ST_SchemeColorVal`'s `phClr` is `PlaceholderColor`.** §20.1.10.54 titles it *Style Color* and
  describes it as *"a color used in theme definitions which means to use the color of the style"*.
  `PlaceholderColor` reads the `ph` as *placeholder*, which is a guess, and the guess says something
  the specification does not.

The same audit checked all 743 variant overrides against the Part 1 prose and found no third case:
every other divergence from the published friendly name is either a deliberate, documented departure
(`MYD`, `axisPage` — see [The naming convention](the_naming_convention)) or a plain expansion of it.

**2 · Whether an `UNCOVERED_SCHEMAS` note is still true.** `check_uncovered_schemas_are_live` makes a
row fail when the tables have made it **unreachable** — a schema that gained coverage in both tables
and kept its row, a note written for a column that is computed elsewhere. It cannot tell you that
*"bibliography sources are preserved verbatim, never authored"* has stopped being true of
`mjx-docx`. That sentence is generated into `COVERAGE.md`, a shipped document, and it is prose. Treat
every `not modelled — …` note as a claim made by a person on a date, not as a checked fact.

**3 · Whether the curated slices are the right slices.** `dml-main` emits 32 of the schema's simple
types and `pml` emits 5. Nothing anywhere says a thirty-third is needed; a workstream discovers it by
reaching for a type that is not there. The lists are a record of what has been ported, not a
judgement about what should be.

**4 · Anything about the Strict world beyond the namespace URI.** `namespaces` pairs both
conformance worlds, and every other generated table is read from the **Transitional** schemas only.
Where Strict and Transitional differ in more than a namespace, this crate describes Transitional.
That is the right default — it is what Microsoft Office emits — but it is a choice, and it is not
recorded anywhere else.

## The failure mode this page is written against

`MJXOFF-88 §7`, in this crate's form:

> A gate phrased *"the committed output is checked"* is green precisely when the check is skipped.

`the_committed_output_is_what_the_generator_produces_today` **skips silently and passes** on any
machine without the full `References/` tree, which includes every CI runner. `MJX_REQUIRE_CODEGEN=1`
is what turns that skip into a failure, and **no workflow sets it**, because no workflow could: the
job that extracts schemas extracts Part 4 and Part 2, and this generator needs Part 1. That is stated
here, in the ticket (MJXOFF-197, which owns the archive change) and in the test's own module
documentation, so that the skip is a known cost rather than an accidental one.

If you are reading this on a machine that has the spec, the one-line way to find out whether any of
it has rotted since is:

```sh
cargo run -p xtask -- codegen --check
```
