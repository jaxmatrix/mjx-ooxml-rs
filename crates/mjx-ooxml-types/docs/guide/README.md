# Guide

**`mjx-ooxml-types` is the vocabulary every other crate here is written in, and almost none of it was
written by a person.** 84,128 of its 85,400 lines are emitted by `xtask/src/codegen/` from the
ECMA-376 XSD schemas; 1,189 are hand-written. It declares no model, opens no package and parses no
XML. It answers three questions and nothing else:

* **What may this attribute say?** — the `ST_*` simple types, given names a reader can understand.
* **Where does this child belong among its siblings?** — the `xsd:sequence` position of every child
  of every complex type of the schemas this workspace authors markup in.
* **What is this schema's namespace, in each of the two OOXML worlds?**

It sits at **layering rank 1.0**, beside `mjx-opc` and `mjx-mce`, and depends on `mjx-ooxml-core`
alone. Nine ranked crates reach it — `mjx-dml`, `mjx-sml`, `mjx-chart`, `mjx-omml`, `mjx-vml`,
`mjx-pptx`, `mjx-docx`, `mjx-xlsx`, `mjx-ooxml` — plus `mjx-schema-gate` and `xtask` from outside the
ranked graph, and `mjx-derive` as a dev-dependency only.

## The one thing to know before anything else

**Nothing re-derived the committed output until MJXOFF-224, and even now the check that does cannot
run on CI.** `CLAUDE.md` decides that generated source is *committed, never a `build.rs`*. That is
right — a `build.rs` would put a 5,000-page specification and a `rustfmt` invocation on every
consumer's critical path — but it has a cost nobody had written down:

> A generator defect is frozen into the repository rather than failing on the next build, and the
> committed file is the only artefact anyone reads, so a defect looks exactly like a deliberate
> choice.

[What to distrust](what_to_distrust) is the honest page about that: which gate catches what, which
one skips silently, and which claims in this crate nothing checks at all. **Read it before trusting
a table here that you have not verified against the schema yourself.**

## Generated, hand-written, and the line between them

| | Lines | Written by | Checked by |
|---|---:|---|---|
| `crates/mjx-ooxml-types/src/generated/*.rs` — nine simple-type modules, the child-order tables, the namespace table, the module root | 84,128 | `xtask/src/codegen/` | `xtask/tests/codegen_drift.rs` (needs `References/`) |
| `COVERAGE.md` — which schema is covered in which table | — | the same generator, from its own tables | `xtask/tests/codegen_drift.rs`, `mjx_schema_gate::categories` |
| `crates/mjx-ooxml-types/src/child_order.rs` — the placement primitives the tables are expressed in | 730 | by hand | its own `#[cfg(test)]` suite |
| `crates/mjx-ooxml-types/src/support.rs` — the wire-parse error, the three boolean normalizers, four attribute codecs | 319 | by hand | `crates/mjx-ooxml-types/tests/wire.rs` |
| `crates/mjx-ooxml-types/src/drawingml.rs`, `crates/mjx-ooxml-types/src/presentationml.rs` — the curated re-export of a `pub(crate)` generated module, plus the preset-shape metadata types | 93 | by hand | `crates/mjx-ooxml-types/tests/adjustments.rs`, `xtask/tests/codegen_drift.rs` |
| `crates/mjx-ooxml-types/src/lib.rs` | 47 | by hand | — |

The **curation** is the interesting half of that table. `drawingml` and `presentationml` are emitted
`pub(crate)` and re-exported item by item, so the crate's public surface is a decision rather than
whatever the generator happened to emit; every other module is emitted `pub` and re-exported whole.
Both re-export lists are hand-written, and
`the_curated_re_exports_cover_every_generated_item` in `xtask/tests/codegen_drift.rs` is what stops
a generated item from quietly failing to reach them.

## The pages

| Page | Read it when |
|---|---|
| [What is generated](what_is_generated) | You want to know what is actually in here — the nine modules, the 360 simple types, and the shape of every emitted item |
| [Child order](child_order) | You are writing a serializer, or you want to know why 59,529 of these lines exist |
| [The naming convention](the_naming_convention) | You are looking at a name and wondering where it came from, or you are about to add one |
| [Regenerating](regenerating) | You are changing the generator, or `codegen` has just failed on you |
| [What to distrust](what_to_distrust) | **Before relying on anything above** |

## The naming convention in one example

OOXML's own symbols are unreadable. This is `dml-spreadsheetDrawing.xsd`'s `ST_EditAs`, which says
what an anchored object does when the rows and columns under it move, and this is the whole of what
this crate is for:

```rust
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;

// The Rust name says what it means; the wire token is preserved exactly, never guessed.
assert_eq!(
    ResizingBehavior::from_wire("twoCell"),
    Some(ResizingBehavior::MoveAndResizeWithAnchorCells)
);
assert_eq!(ResizingBehavior::MoveAndResizeWithAnchorCells.to_wire(), "twoCell");
```

Every generated item records its original `ST_*` symbol and its exact wire token in its own doc
comment, so the mapping is never something a reader has to reconstruct.
[The naming convention](the_naming_convention) is how the left-hand side is decided, and it is the
one part of this crate that is curated by hand rather than derived.

## What this crate deliberately is not

**It is not a model.** There is no `Shape`, no `Cell`, no `Paragraph`. A `ST_*` type is a value an
attribute may take; the elements those attributes hang on live in `mjx-dml`, `mjx-sml` and the format
crates. The one apparent exception — the preset-shape adjustment tables in `drawingml` — is still
mechanical: it carries the spec's own `avLst`/`ahLst`/`gdLst` facts as data, and the formula language
that turns them into geometry belongs to `mjx-dml`, one rank up.

**It does not validate.** `ChildOrder` is a *write-side* placement aid: it says where a new child
belongs, and it never reorders a document that was read from disk. `mjx-schema-gate` is what holds
output to the XSD.

**It does not choose a conformance world.** `namespaces` pairs each schema's Strict (ISO-29500) and
Transitional URIs and `SchemaNamespace::for_strict` picks between them; which one a package is
written in is `mjx-opc`'s decision, not this crate's.
