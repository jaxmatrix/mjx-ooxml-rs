# Fidelity and the known gaps

What is guaranteed here, what backs each guarantee, and every gap that is still open. Read this
before relying on something in production, and read the gap list as a list of **claims about
behaviour** rather than as a list of inconveniences: *cannot author a theme* was filed on
`mjx-xlsx`'s limitations page as an authoring nicety and meant every Word and Excel chart rendered
with a title, axes, labels, a legend and no bars.

## The contract

Per-part **decompressed-payload byte identity** for every part this crate did not touch, and
structural container identity — not identical ZIP bytes.
`crates/mjx-opc/docs/guide/the_round_trip_contract.md` states it in full, with what enforces each
clause and what it deliberately leaves out. Everything below is how this crate holds up its end.

## The four mechanisms, and which one backs a given type

| Mechanism | Backed by | Where |
|---|---|---|
| `#[derive(FromXml, ToXml)]` | `mjx-derive`'s codegen emits the unknown-child fallthrough unconditionally, with no way to invoke the arm without it | `crates/mjx-derive/tests/derive.rs`, once, for every derived type at once |
| The shape macros — `attribute_bag!`, `character_data_shape!`, `relationship_reference!`, `entry_list!` | one macro body each, storing `name`, `attributes`, children and the self-closing flag verbatim | `crates/mjx-sml/src/leaf.rs`, `crates/mjx-sml/src/features/embedded.rs` |
| A **rebuilder** — an inherent `as_raw_element(&self)` | this crate's own ledger, below | `crates/mjx-sml/tests/serialization_ledger.rs` |
| The **slot frames** — whole parts | verbatim source ranges and one door per slot | [The slot frame](the_slot_frame) |

**Why a rebuilder rather than the derive's `ToXml`.** `ToXml::to_xml` takes `&mut Interner`, because
a model that authors an element name has to intern it. Nothing in the worksheet family ever authors
one, and a worksheet's writer takes `&self` — which is what lets the `sheetData` slot be
[a packed store](the_cell_store) rather than a subtree. A `&self` writer has no mutable interner to
lend, so the rebuild is an inherent method and `to_xml` is that method with the parameter ignored.

### The ledger, and why its shape differs from `mjx-dml`'s

MJXOFF-216 found `mjx_dml::Picture` reading three children by name, discarding every other one, and
rebuilding with a synthesised element name and an empty attribute vector — destroying a foreign
attribute, a foreign child and every `xmlns` declaration on the element. It passed three gates at
once. `crates/mjx-dml/tests/serialization_ledger.rs` is what stops the next one there: every
hand-written impl is on a ledger declaring an idiom, and the idiom is checked against the body.

Copying that here would have been **vacuous**, and the reason is the failure mode this whole phase is
written against. Almost every hand-written impl in `mjx-sml` is `ToXml`-only — the reader is the
derive — and almost every one of the writers is the *same three lines*, handing the work to the
type's own rebuilder. Each would be a "dispatcher" under `mjx-dml`'s vocabulary, each would pass a
dispatcher's check trivially and forever, and a ledger of one reason repeated fifty-odd times is a
list that passes once it is written.

So `crates/mjx-sml/tests/serialization_ledger.rs` follows the delegation instead. It requires every
hand-written writer to **be** that delegation or to carry a row saying what it keeps, holds every
rebuilder in the crate to `self.name`, `self.attributes` and `self.empty` — MJXOFF-216's exact shape —
requires every content-enum dispatcher to construct no element at all, and puts every hand-written
reader on a ledger of its own with the reason the derive does not fit. It prints its census on every
run (impls, types, rebuilders, dispatchers, derive sites, macro invocations) rather than pinning
them, so the numbers cannot go stale in prose.

**Nothing in this crate loses content today**: every rebuilder already preserves. What is new is that
the next one cannot arrive unnoticed — and it would have. Replacing `&self.attributes` with a fresh
vector in one rebuilder leaves every unit test and every suite in the crate green while destroying
`@count` and every foreign attribute on the element, because *a rebuilder is only reached once a slot
has given up its verbatim bytes*, so every round-trip assertion here is green whether the rebuild
preserves anything or not. `crates/mjx-sml/tests/sheet_grid.rs`'s
`editing_the_merges_keeps_the_attributes_the_file_wrote_on_the_element` is the behavioural half that
runs one of them.

## Half of `sml.xsd` is preserved rather than modelled

`mjx_ooxml_types::child_order::SML_TYPES` carries the child order of every SpreadsheetML complex
type, and its length is the schema's count: **367**. Nine clusters of them describe features whose
data this library neither computes nor refreshes, and together they come to **184** — measured by
MJXOFF-133 against `sml.xsd` and tabulated cluster by cluster, with its line ranges and root
elements, in `crates/mjx-sml/src/preserved/mod.rs`.

| Cluster | Why not modelled |
|---|---|
| pivot caches and pivot tables (97 types — a quarter of the schema) | **derived data.** A pivot cache is a snapshot of a range and a pivot table an aggregation over it, so modelling it faithfully means being able to say what a consumer would show, which means a calculation model this library explicitly does not have. If it is ever modelled it is a phase of its own. |
| shared-workbook revisions, cell metadata, single-cell XML tables, custom XML mappings | modellable in principle, and not worth their weight against the features a caller asks for |
| external links, data connections, query tables | refreshing one is **I/O** — opening another workbook, running a query — and this library performs none |
| volatile dependencies | evaluating the graph is calculation |

**Preserved is not ignored.** Every one of those parts is carried through a save byte for byte by
`mjx-opc`'s part-level copy-on-write, and that is asserted per part kind rather than assumed:
`crates/mjx-xlsx/tests/preserved_parts.rs` opens a workbook carrying one of each, edits a cell on the
very sheet the pivot table sits on, and compares every one against the bytes it went in with. *A part
nothing asserts on is a part that silently disappears.*

What this crate offers instead of a model is a set of **read-only identity views** —
[`mjx_sml::PivotTableIdentity`](crate::PivotTableIdentity),
[`mjx_sml::PivotCacheIdentity`](crate::PivotCacheIdentity),
[`mjx_sml::ExternalLinkIdentity`](crate::ExternalLinkIdentity),
[`mjx_sml::ConnectionIdentity`](crate::ConnectionIdentity),
[`mjx_sml::QueryTableIdentity`](crate::QueryTableIdentity),
[`mjx_sml::XmlMapIdentity`](crate::XmlMapIdentity),
[`mjx_sml::RevisionHeadersIdentity`](crate::RevisionHeadersIdentity) and their siblings. Each reads
the few attributes that answer *what is this, and what does it point at*, hands back an owned value,
and implements no `ToXml` at all, so nothing is ever written back through one.

## What is never done

**Nothing is evaluated.** There is no calculation engine and there will not be one. A formula is
text; a cached `<v>` is what a producer last computed and is never invalidated by an edit; an
autofilter, a sort state and a conditional-formatting rule are each a *record* of an operation rather
than the operation. Blanking a stale `<v>` would destroy data in cells the caller never named, and
setting `fullCalcOnLoad` would write into a part the caller did not ask to edit — Excel recalculates
on open when it needs to, and [`mjx_sml::CalculationProperties`](crate::CalculationProperties) is
there for a caller who decides otherwise deliberately.

**Nothing is repaired.** A `dimension` that disagrees with the cells, a duplicate `row@r`, a merge
overlapping another, a `spans` that does not match the row: each is reported — as a
[`mjx_sml::GridAnomaly`](crate::GridAnomaly), a
[`mjx_sml::SheetDataAnomaly`](crate::SheetDataAnomaly), or as `None` from a method that cannot answer
— and none is corrected.

**Nothing is deduplicated or reordered.** See [The stylesheet](the_stylesheet): indices are identity,
and so is a shared string's position.

**No I/O, and no clock.** Which is also why this crate compiles to WebAssembly unchanged.

## The gaps

Absent, not the consequence of a decision above, and named here so nobody plans around a surface that
is not present.

| Absent | What exists instead |
|---|---|
| **Threaded comments** — `xl/threadedComments/*.xml` and `xl/persons/person.xml` | Nothing. They appear nowhere in this repository, and the reason is below. |
| **Writing a formula into a cell** | [`mjx_sml::CellFormula`](crate::CellFormula) is a read-only view over a cell's `<f>` bytes. A formula round-trips because nothing rewrites a cell it was not asked to; authoring one means writing the `<c>` markup yourself. |
| **Modelling any of the 184** | The identity views above, and byte-for-byte preservation. |
| **A macrosheet's cells** | Held verbatim; nothing here interprets XLM. |
| **Resolving an `r:id`** | The string the file wrote, plus the prefix it bound the relationship namespace to. Resolution is `mjx-xlsx`'s, deliberately. |

### Threaded comments, and why the gap is bigger than it looks

`sml.xsd` models `xl/comments1.xml` — `CT_Comments`, the note attached to a cell — and this crate
models it: [`mjx_sml::Comments`](crate::Comments),
[`mjx_sml::CommentText`](crate::CommentText), [`mjx_sml::parse_comments`](crate::parse_comments), and
the VML box that draws one is `mjx-vml`'s.

**A modern Excel does not write only that.** A comment made in Excel 2016 or later is a *threaded*
comment: the text lives in `xl/threadedComments/threadedComment1.xml`, the authors live in
`xl/persons/person.xml`, and the legacy `comments1.xml` carries a **shadow copy** so that older
consumers show something. Neither of the two new parts is in ECMA-376 at all — they are
`http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments`, an extension — which is
why nothing generated from the schema names them and why they appear nowhere in this repository, in
code or in prose, before this page.

What that costs, stated plainly:

* **They round-trip.** They are ordinary parts, `mjx-opc` carries them byte for byte, and neither
  this crate nor `mjx-xlsx` touches them. A workbook opened and saved keeps its threaded comments.
* **They are invisible.** `mjx_xlsx::Workbook::sheet_comments` reports the legacy shadow copy, which is the
  text but not the thread: not the replies, not the resolved flag, not the `personId` that says who
  wrote it. A caller counting comments gets a plausible answer that is not the whole one.
* **And the two halves can be made to disagree.** Nothing here edits either part, so they cannot
  drift *through* this library today — but any future write path that edited `comments1.xml` without
  editing the threaded part beside it would produce a file whose two copies of the same comment say
  different things, and no gate in this workspace would see it.

That reasoning existed only in a tracker comment (MJXOFF-88 §9 B12) until MJXOFF-220 put it here.
Closing the gap means modelling a Microsoft extension schema the generator does not cover, which is a
unit of its own; `crates/mjx-ooxml-types/COVERAGE.md` is where the generator's schema coverage is
recorded.

## What no gate here can see

Stated so the holes are deliberate rather than silent.

* **The schema gate validates the markup a test authored.** It cannot see a constructor nothing calls
  that way, which is exactly how `Color::from_opaque_rgb` produced a ten-character `@rgb` for two
  phases without a single failure. The answer is a gate over the *authoring vocabulary* rather than
  over one more fixture — `every_authored_colour_is_a_valid_unsigned_int_hex` — and the same shape of
  hole exists wherever a public constructor has no caller in a test.
* **Round-tripping proves the frame, not the model.** Every part in this crate re-emits from its own
  bytes until something edits it, so a preservation assertion is green whether the model below it
  keeps anything or not. That is why the serialization ledger reads *source shapes* and why
  `sheet_grid.rs` runs one rebuilder for real.
* **No gate asks whether a part we did not write should have existed.** Threaded comments are the
  live instance: nothing fails, and the answer is incomplete.
* **Every fixture under `tests/fixtures/` was written by this project or by LibreOffice**, so every
  gate in the workspace proves that our reader agrees with our writer. `tests/office-authored/` is
  the road to changing that, `docs/validation/06-the-office-pass.md` is how a person walks it, and
  **the corpus is empty**: a file's value there is entirely its provenance, and no agent may fill it.
