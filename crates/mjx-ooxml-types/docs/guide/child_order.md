# Child order

**59,529 of this crate's 84,128 generated lines are one table**, and it exists because of a fact
about OOXML that is easy to miss:

> Children in the wrong order are invalid even when every child is present and every child is itself
> correct — and an application that opens such a file offers to repair it.

Order is validity, not style. `mjx_ooxml_types::child_order` is how a writer in this workspace gets
it right without having read the XSD.

## What is in it

Nine tables, one per schema the workspace authors markup in, holding **1,335 complex types** between
them. The lengths are in the array types themselves, so they are a fact the compiler carries rather
than a comment:

| Table | Schema | Complex types |
|---|---|---:|
| `SML_TYPES` | `sml.xsd` | 367 |
| `WML_TYPES` | `wml.xsd` | 285 |
| `DML_MAIN_TYPES` | `dml-main.xsd` | 231 |
| `PML_TYPES` | `pml.xsd` | 149 |
| `DML_CHART_TYPES` | `dml-chart.xsd` | 136 |
| `SHARED_MATH_TYPES` | `shared-math.xsd` | 72 |
| `DML_DIAGRAM_TYPES` | `dml-diagram.xsd` | 58 |
| `DML_WORDPROCESSING_DRAWING_TYPES` | `dml-wordprocessingDrawing.xsd` | 20 |
| `DML_SPREADSHEET_DRAWING_TYPES` | `dml-spreadsheetDrawing.xsd` | 17 |

**There is no allowlist here, unlike the simple types.** A serializer can only be stopped from
writing out of sequence if the type it is writing is in the table, so a schema that joins
`CHILD_ORDER_SCHEMAS` is emitted whole. A schema joins when a crate starts authoring its markup —
`dml-wordprocessingDrawing` with MJXOFF-131, `shared-math` with MJXOFF-134, `dml-spreadsheetDrawing`
with MJXOFF-107, each graduating out of the *parsed only to resolve a cross-schema reference* list
beside it.

Above the nine arrays sit **205 named constants** — `SHAPE_PROPERTIES`, `LINE_PROPERTIES`,
`CATEGORY_AXIS` — one for each complex type a serializer in this workspace holds as a constant. Those
names are curated (`spec::CHILD_ORDER_EXPORTS`), for the same reason every other name here is:
`CT_CatAx` is `CATEGORY_AXIS` because ECMA-376 calls `c:catAx` a Category Axis, not because an
abbreviation table guessed it.

## Rank, and why an `xsd:choice` shares one

Every child a complex type can hold carries a **rank**: its position in the type's flattened content
model. Members of an `xsd:sequence` get successive ranks; the alternatives of an `xsd:choice` *share*
one, because the schema lets any of them stand in that place.

```rust
use mjx_ooxml_types::child_order::TEXT_LIST_STYLE;

// `CT_TextListStyle` is `defPPr`, `lvl1pPr` … `lvl9pPr`, `extLst`.
assert_eq!(TEXT_LIST_STYLE.rank_of(None, "defPPr"), Some(0));
assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl9pPr"), Some(9));
assert_eq!(TEXT_LIST_STYLE.rank_of(None, "extLst"), Some(10));
assert_eq!(TEXT_LIST_STYLE.rank_of(None, "bodyPr"), None); // not a child of this type
```

The shared rank is what makes `ChildOrder::replace_or_insert` correct: in `CT_ShapeProperties` the
six fill elements all rank alike, so setting a solid fill replaces whichever fill was there rather
than adding a second one beside it.

## The census, and the one `xsd:all`

Over all nine tables:

| Content model | Types | What it means for placement |
|---|---:|---|
| `Sequence` | 806 | order is validity; this is the only model that imposes one |
| `Empty` | 470 | no child elements at all — attribute-only or empty types |
| `Choice` | 58 | any alternative may stand in the place; no order is imposed |
| `All` | **1** | `CT_DocPartPr` in `wml.xsd`; ECMA-376 places no order on its members |

That census is asserted, not stated: `the_census_of_content_models_is_what_the_schemas_say` in
`crates/mjx-ooxml-types/src/child_order.rs` counts it out of the tables and pins the tuple, so it
moves only when the schemas or the flattener do. Beside it,
`no_unordered_type_is_given_a_false_order` pins the safety property the whole design rests on: **no
child of a `Choice` or an `All` type may outrank another.** A flattener change that started ranking a
choice's branches would make this table *worse than none* — it would fault conforming markup — and
that test is what stands in front of it.

## What is never reordered

Placement is a **write-side** operation. Nothing here reads a document and rewrites it into schema
order, and three separate rules keep it that way:

* **A child the table does not name is invisible to placement.** An unmodelled element, a foreign
  namespace, a comment, an `mc:AlternateContent` — none of them moves, and none of them moves the
  insertion point. Unmodelled markup keeps its position relative to its known neighbours, which is
  what makes the unknown bucket survive an edit.
* **Existing known children are never sorted.** A new child is inserted *after the last sibling that
  must precede it*; everything already in the element stays where the caller's file had it.
* **A `Choice` or `All` type imposes no order and this module does not invent one.**

`ChildOrder::first_out_of_order` and `audit_tree` exist to *report* on a tree, for a gate, and they
still change nothing. `mjx-schema-gate` is what holds output to the XSD; this is what makes it pass.

## Cost

A `ChildOrder` is a `&'static` slice of a handful of names — the median complex type in these schemas
has four children; the largest has around forty. Lookup is a linear scan of `&str` comparisons over
that slice: no hashing, no allocation, nothing built per call. Serializers hold the
`&'static ChildOrder` for their own type as a constant, so the per-call cost is the scan alone. The
by-symbol lookups (`find`, `root_element`) bisect a table sorted by XSD symbol and exist for
tree-walking audits, not for the serialization path.
