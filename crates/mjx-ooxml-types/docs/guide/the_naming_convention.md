# The naming convention

This crate exists because ECMA-376's own identifiers are unreadable. `ST_Jc`, `ST_Shd`, `ST_OnOff`;
values spelled `t`, `ctr`, `dist`, `shdw13`, `3TrafficLights1`. A model built directly on those is a
model nobody can read without the specification open beside them, and `CLAUDE.md` therefore requires
that **every public identifier in this project be self-explanatory** — generated ones included.

Two rules do the work, and they pull in opposite directions:

> **Wire tokens are preserved exactly, never guessed.** Each type and variant maps to its exact XSD
> string, and the original `ST_*` symbol and wire token appear in the item's docs.
>
> **Where the meaning is not inferable from the token, the name is sourced from the ECMA-376 Part 1
> prose — never guessed.**

The first is mechanical and checked. The second is a human judgement, made 907 times, and it is the
part of this crate a reader should treat as curated rather than derived.

## How a name is decided

`xtask/src/codegen/naming.rs` is the engine and it is 168 lines. In order:

1. **A curated override wins.** `NameEngine::type_name` and `NameEngine::variant_name` look the
   symbol up in `type_overrides` / `variant_overrides` first and return the curated name if there
   is one.
2. Otherwise the token is **split into words** on separators and camelCase / letter↔digit humps
   (`split_words`), so `flowChartProcess` becomes `flow`, `Chart`, `Process` and `star4` becomes
   `star`, `4`.
3. Each word is **expanded** if the engine's abbreviation table knows it (`alg` → `Algorithm`), and
   PascalCased otherwise.
4. The result is **sanitized** into a valid, non-reserved Rust identifier: a digit-leading name gets
   an `N` prefix, and `Self`/`Super`/`Crate` — reserved even capitalized, and not expressible as raw
   identifiers — get a `Value` suffix.

Only the Rust-facing name is ever touched. The wire token is carried through untouched into
`from_wire`, `to_wire` and the item's doc comment.

## The tables, and why there are five engines

`xtask/src/codegen/spec.rs` holds the data — 3,239 lines of it, and it is the largest hand-written
file in the codegen. There are five `NameEngine`s rather than one because **`ST_*` symbols are
schema-scoped and collide**: `ST_Jc` is WordprocessingML's justification *and* Office Math's;
`ST_Direction` is PresentationML's placeholder orientation, WordprocessingML's bidirectional
direction *and* SmartArt's traversal direction. One flat table could not name all three.

| Engine | Schemas | Type overrides | Variant overrides |
|---|---|---:|---:|
| `ENGINE` | `shared-commonSimpleTypes`, `dml-main`, `pml`, and the two drawing schemas | 50 | 291 |
| `WORDPROCESSINGML_ENGINE` | `wml` | 78 | 169 |
| `SPREADSHEETML_ENGINE` | `sml` | 14 | 149 |
| `DIAGRAM_ENGINE` | `dml-diagram` | 9 | 120 |
| `OFFICEMATH_ENGINE` | `shared-math` | 13 | 14 |

164 type overrides and 743 variant overrides. Everything else — the great majority of the 2,364 enumeration variants the nine generated modules
declare between them — is the mechanical path above.

## What "sourced from the prose" looks like

Four worked examples, each of which the mechanical path would have got wrong, and each of which is
what the ECMA-376 Part 1 table actually says:

| Wire token | Mechanical name | Curated name | Why |
|---|---|---|---|
| `3TrafficLights1` | `N3TrafficLights1` | `ThreeTrafficLights` | §18.18.42 titles it *3 Traffic Lights*; the trailing `1` distinguishes it from `3TrafficLights2`, *3 Traffic Lights Black* |
| `3Symbols` | `N3Symbols` | `ThreeSymbolsCircled` | §18.18.42 titles `3Symbols` *3 Symbols Circled* and `3Symbols2` *3 Symbols* — the digit and the adjective run in opposite directions, so a mechanical name would invert them |
| `count` | `Count` | `CountNonEmpty` | §18.18.17: the Count consolidation function *"works the same as the COUNTA worksheet function"*, and §18.18.83 titles it *Non Empty Cell Count* |
| `stdDev` | `StdDev` | `EstimatedStandardDeviation` (`ST_TotalsRowFunction`), `SampleStandardDeviation` (`ST_DataConsolidateFunction`), `StandardDeviation` (`ST_ItemType`) | the same token means three subtly different things in three tables, and each row follows its own section |

The last one is the reason the tables are keyed by `(ST_*, wire)` rather than by wire alone.

**Where the published table is wrong, the row says so.** ECMA-376's friendly-name column has real
errors, and two of them are recorded in `spec.rs` beside the rows that route around them: `MYD` in
§18.18.27 is printed *"Month Day Year"*, identical to `MDY`'s, and its Description says *month, year,
day* — following the friendly names would have collapsed two wire tokens onto one variant, which the
generator refuses outright. `axisPage` in §18.18.1 is printed *"Include Count Filter"*, which belongs
to a different table entirely; its Description says *Page axis*.

## What is checked, and what is not

**Checked.** `spec::unused_overrides` — called from `check_overrides_are_live` on every run — fails
the generator when an override row names a type the schema does not declare or a wire value it does
not list. That is what makes *wire tokens preserved exactly, never guessed* a property rather than a
convention: an invented token cannot survive a regeneration.

**Checked.** `every_curated_enumeration_cites_the_spec_section_its_names_came_from` in
`xtask/tests/codegen_drift.rs` requires that, for each of the 110 enumerations these five tables
curate, at least one of its rows sits directly under a comment naming the ECMA-376 section the names
were read out of. A name taken from the prose without a citation is indistinguishable from a guess.
Six of the 110 had none when the gate was written — `ST_ShapeType`, `ST_SchemeColorVal`,
`ST_PresetPatternVal`, `ST_ColorSchemeIndex`, `ST_PathFillMode` and `ST_Border`, all among the oldest
rows here, written before the discipline existed. Type overrides are deliberately outside this gate:
`CLAUDE.md`'s rule for a *type* name is to expand abbreviations to full words, which is mechanical
and needs no prose.

**Not checked, and cannot be.** Whether the name a cited row chose is the name the cited section
actually gives. Nothing in this workspace reads ECMA-376's prose — it ships as a PDF and
`References/` is git-ignored — so the citation is an audit trail for a person, not an assertion. Two
places where that trail leads somewhere a reader should know about are recorded in
[What to distrust](what_to_distrust).

## Adding a name

Adding a schema means growing the tables, not the engine. In order:

1. Add the type to `DRAWINGML_TYPES` / `PRESENTATIONML_TYPES` if it is one of the two curated
   modules, or the schema to `SIMPLE_TYPE_MODULES` if it is a new one.
2. Run `cargo run -p xtask -- codegen` and read the emitted names. Most will be right.
3. For each that is not, open the ECMA-376 Part 1 section for that `ST_*` type, take the
   enumeration-value title, and add a `variant_overrides` row **under a comment naming the section**.
   The gate above requires the citation; the generator requires the wire token to exist.
4. Regenerate, and commit the generated files with the change. The two must land together — see
   [Regenerating](regenerating).
