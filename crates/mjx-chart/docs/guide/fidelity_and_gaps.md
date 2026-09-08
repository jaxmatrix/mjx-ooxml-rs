# Fidelity and the known gaps

What survives a round trip through `mjx-chart`, `mjx-omml` and `mjx-vml`, what enforces it, and what
nothing here checks. This page covers all three crates, because the mechanism is the same in all
three and the *guarantee* is not.

## Two mechanisms, and neither of them is written per type

Every type in these three crates that models an element reaches XML through exactly one of two things:

| Mechanism | What it is | Backed by |
|---|---|---|
| `#[derive(FromXml, ToXml)]` | `mjx-derive`'s codegen: the four framework fields, a typed child list, and an implicit `Raw(RawNode)` catch-all for everything unmatched | `crates/mjx-derive/tests/derive.rs`, once for the whole workspace |
| the crate's own `fidelity_*!` macro | one body per crate — `crates/mjx-chart/src/build.rs`, `crates/mjx-omml/src/support.rs`, `crates/mjx-vml/src/build.rs` | `xtask/tests/upper_markup_ledger.rs` |

At 0.0.140 that is **62 element declarations** and **zero hand-written `FromXml`/`ToXml` impls**:
`mjx-chart` 36 declarations (30 derived, 6 on `fidelity_element_impls!`), `mjx-omml` 10 (1 derived, 9
on the same macro — one of them `fidelity_struct!`, which backs 38 types), `mjx-vml` 16 (5 derived, 11
on `fidelity_leaf!`).

**The `Raw` catch-all cannot be forgotten.** `crates/mjx-derive/src/parse.rs` hard-codes the variant
name and `crates/mjx-derive/src/expand.rs` emits its arm unconditionally, so a content enum without
one does not compile. That is why an unmodelled plot type, an unknown attribute, an `extLst`, a
`w10:wrap` or an `mc:AlternateContent` branch comes back byte for byte without anyone deciding that it
should.

## Why the ledger here asks a different question

`crates/mjx-dml/tests/serialization_ledger.rs` asks what eight hand-written pairs lose in between
reading and rebuilding — MJXOFF-216 found six of them destroying a foreign attribute, a foreign child
or an `xmlns` declaration. `crates/mjx-sml/tests/serialization_ledger.rs` found that question did not
transfer there, and followed the risk into the rebuilder behind a delegation.

It does not transfer here either, and the reason is the table above: with zero hand-written impls
there is no body to audit, and a ledger of impl bodies would be an empty list that passes forever.
So `xtask/tests/upper_markup_ledger.rs` asks the question that *is* live — **which mechanism is a type
on, and would anyone notice a type going on neither?** — and answers it five ways:

1. both mechanisms are still being found, with the counts printed;
2. no type hand-writes an impl (and any that starts to must be written onto a ledger with a reason);
3. no type that models an element is on neither mechanism;
4. each `fidelity_*!` writer mentions every field its own reader captured, and never builds a fresh
   empty collection — MJXOFF-216's exact shape, stated generically;
5. **the impl scanner is calibrated against `mjx-dml` and `mjx-sml`**, because a zero cannot carry a
   floor of its own. With the scanner deliberately mistyped, check 2 still reported *0 hand-written
   impls, 0 on the ledger* and passed; only the calibration noticed.

## The guarantee is not the same in all three crates

| | Schema-validated | Child order from the XSD | Round-trip |
|---|---|---|---|
| `mjx-chart` | **yes** — `dml-chart.xsd` is a modelled schema in `crates/mjx-schema-gate/src/categories.rs`, probed at `chartSpace` | **yes** — `dml-chart` is in the generator's `CHILD_ORDER_SCHEMAS`, so every writer places children by table | yes |
| `mjx-omml` | **yes, as a subtree** — there is no math part; an `m:oMath` is validated with the `word/document.xml` that carries it, and `wml.xsd` imports `shared-math.xsd` | **yes** — `shared-math` graduated into `CHILD_ORDER_SCHEMAS` with MJXOFF-134 | yes |
| `mjx-vml` | **no** | **no** | **yes, and it is the only check there is** |

`crates/mjx-ooxml-types/COVERAGE.md` says the same thing in its own two tables, per schema.
`mjx_vml::guide` is where VML's weaker guarantee is stated for a caller who has one in their hands.

## What is not modelled, per crate

### `mjx-chart`

* **The chart drawing canvas.** `dml-chartDrawing.xsd` (`cdr:relSizeAnchor` and friends — free shapes
  a user has drawn *on top of* a chart) is not modelled at all; `COVERAGE.md` records it as
  "nothing in this workspace authors one". A chart part carrying one round-trips it raw.
* **No `ST_*` enumeration is generated for `dml-chart`.** The typed scalars here
  ([`BarDirection`](crate::BarDirection), [`ChartKind`](crate::ChartKind),
  [`LegendPosition`](crate::LegendPosition), …) are hand-written against the schema's own tokens
  rather than taken from `mjx-ooxml-types`, which is why `COVERAGE.md`'s simple-type row for this
  schema reads *pending*.
* **Nothing is resolved and nothing is recomputed.** A `c:f` is text
  ([Reading a chart](reading_a_chart)); a cached value is what draws and is never recalculated from a
  worksheet; a chart style id is reported, never applied.
* **`CT_Extension`'s wildcard is declared without a `minOccurs` in `dml-chart.xsd`**, exactly as it is
  in `sml.xsd`. Markup-compatibility resolution empties an `<ext>` whose only child was ignorable, and
  the schema then rejects the hole. That is a defect in how the two compose rather than a property of
  anyone's file, it is **MJXOFF-196**, and `xtask/src/validation/ingest.rs` carries the full statement
  and the third address a fix has to visit.

### `mjx-omml`

All 72 `shared-math.xsd` complex types are modelled, so the gap is not coverage but *decomposition*:
`mjx_omml::ControlProperties` (`m:ctrlPr`) preserves its children **wholesale and
raw**, because the schema declares them as WordprocessingML (`EG_RPrMath` — a `w:rPr`, a `w:ins`, a
`w:del`) and `mjx-omml` sits below `mjx-docx`. `mjx-docx` types them from above. `mjx_omml::guide` has
the argument and why it costs no fidelity.

### `mjx-vml`

`COVERAGE.md` records all five VML schemas as *not modelled* for simple types and for child order, and
means it literally: **`mjx-vml` never authors an `ST_*` value and nothing re-sequences a VML part.**
What the crate models is the subset a modern construct has to reach —
`mjx_vml::Shape`, `mjx_vml::ShapeTemplate`,
`mjx_vml::ShapeGroup`, `mjx_vml::ShapeIdMap` and their siblings — and
everything else stays in the `Raw` bucket. Because there is no schema gate behind it, the round-trip
suite is load-bearing rather than reassuring; `mjx_vml::guide` says what that means in practice.

## What no gate in these three crates can see

Stated here rather than left as an absence, because an unstated exclusion is how a gate becomes
vacuous without anyone deciding that it should.

* **Whether a part we did not write should have existed**, or whether a part we did write should have
  been left alone. Every check here asks whether the bytes we wrote are the bytes we meant. That is
  the shape MJXOFF-198 §5 names, and it is why the embedded-workbook default was inverted for as long
  as it was.
* **Whether a typed accessor reads the right thing.** The ledger is about what survives a round trip.
  `crates/mjx-chart/tests/`, `crates/mjx-omml/tests/deep_nesting.rs` and
  `crates/mjx-vml/tests/drawing.rs` are where behaviour is asserted.
* **Rendering.** `soffice` is a change detector in this project and never the reference; parity is
  judged against real Microsoft Office by a person, through `docs/validation/06-the-office-pass.md`.
