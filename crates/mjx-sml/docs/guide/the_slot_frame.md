# The slot frame

Every whole part in this crate is the same shape, and it is the shape the fidelity contract is made
of. This page is what *held* means, why an untouched child comes back byte for byte whether this
project has ever heard of it or not, and where a new child goes when one is inserted.

## The shape

A frame is a root element — its qualified name with the prefix the file used, its attributes in
order, every `xmlns:` declaration among them, and whether it was written self-closing — plus a vector
of **slots** in document order. A slot holds two things: the element as it was *moved* out of the
parsed tree with its verbatim source range intact, and the value the frame made of it.

```text
WorksheetPart
├── prologue           the XML declaration, and any comment or PI beside it
├── name, attributes   as the file wrote them
├── content: Vec<Slot>
│   ├── Slot { verbatim: Some(<sheetPr …/>), value: Properties(SheetProperties) }
│   ├── Slot { verbatim: None,               value: SheetData(the packed store) }
│   ├── Slot { verbatim: Some(<phoneticPr/>), value: Raw(node) }
│   └── …
└── epilogue
```

Six types are frames: [`mjx_sml::WorksheetPart`](crate::WorksheetPart),
[`mjx_sml::WorkbookPart`](crate::WorkbookPart), [`mjx_sml::StylesheetPart`](crate::StylesheetPart),
[`mjx_sml::ChartSheetPart`](crate::ChartSheetPart),
[`mjx_sml::DialogSheetPart`](crate::DialogSheetPart) and
[`mjx_sml::MacroSheetPart`](crate::MacroSheetPart). The last three share one generic frame, in
`crates/mjx-sml/src/sheets/frame.rs`, because a sheet kind whose markup is entirely borrowed from the
worksheet spine should be a content enum and a set of accessors and nothing else.

## Modelled and held are different claims

A slot is **modelled** when the frame typed it — there is a variant of the content enum for it, and
an accessor. It is **held** when the frame kept the node exactly as it was read and offers no
accessor at all.

**Both round-trip byte for byte.** That is the point, and it is also the trap: a worksheet whose
`pageSetup` survives a save is proof the *frame* works, not proof `pageSetup` was modelled. Held is
not dropped, and a documented gap is never a validation failure.

Every one of these figures is **derived from the read path**, not written down. The derivation reads
a part holding one of every slot the generated table names and asks the reader which of them it
typed; the tables and the sentences on this page and in the source are then held to that answer.

| Complex type | Slots | Modelled | Held | Held slots | Derived by |
|---|---|---|---|---|---|
| `CT_Worksheet` | 39 | 35 | 4 | `phoneticPr`, `legacyDrawingHF`, `drawingHF`, `extLst` | `every_slot_of_the_generated_sequence_is_accounted_for`, `crates/mjx-sml/src/worksheet/frame.rs` |
| `CT_Workbook` | 19 | 18 | 1 | `extLst` | `every_slot_of_the_generated_sequence_is_accounted_for`, `crates/mjx-sml/src/workbook/mod.rs` |
| `CT_Stylesheet` | 11 | 10 | 1 | `extLst` | `every_slot_of_the_generated_sequence_is_accounted_for`, `crates/mjx-sml/src/styles/stylesheet.rs` |
| `CT_Chartsheet` | 14 | 10 | 4 | `legacyDrawing`, `legacyDrawingHF`, `drawingHF`, `extLst` | `every_slot_of_every_sheet_kind_is_accounted_for`, `crates/mjx-sml/src/sheets/frame.rs` |
| `CT_Dialogsheet` | 16 | 10 | 6 | the four above plus `oleObjects`, `controls` | the same test |
| `CT_Macrosheet` | 27 | 20 | 7 | the four above plus `sheetData`, `phoneticPr`, `oleObjects` | the same test |

`CT_Worksheet` is the widest content model in the schema — ten times `CT_Slide`'s and twice
`CT_Workbook`'s — and its split is the count this programme has got wrong most often: it was written
down as 25/14, then 31/8, then 34/5, then 35/4 across four children, and three of the four were wrong
when they were written. MJXOFF-88 §9 B2 named the structural cause, which is that
`crates/mjx-sml/src/styles/stylesheet.rs` asserted the sum and `crates/mjx-sml/src/worksheet/frame.rs`
asserted nothing. MJXOFF-220 closed it: the split is derived, the module's rank table is held to the
derivation row by row, and so are the sentences around it — including the heading, which is where the
last stale figure actually was.

**A macrosheet's `sheetData` is held on purpose.** Its cells hold the XLM macro language rather than
the formula language [`mjx_sml::CellFormula`](crate::CellFormula) models, and nothing in this
workspace interprets XLM, so a modelled macrosheet cell would be a
[`mjx_sml::CellValue`](crate::CellValue) nobody could act on.

## Placement is generated, never written by hand

A new child goes in at its rank in the type's own `xsd:sequence`, read from
[`mjx_ooxml_types::child_order`](mjx_ooxml_types::child_order) — generated from `sml.xsd` by
`cargo run -p xtask -- codegen`, and committed. MJXOFF-89 deleted fourteen hand-rolled ordering tables
across the workspace and no crate is going to add a fifteenth.

**A held child is ranked too, and that is load-bearing.** The obvious implementation answers "no
rank" for anything unmodelled, on the grounds that placement steps over what it cannot rank. That is
safe only while the modelled slots are a *prefix* of the sequence, which they were when
`CT_Worksheet`'s ranks 0–6 were modelled and nothing else was. They interleave now: a worksheet
holding `sheetData` (5), a held `phoneticPr` (15) and a modelled `conditionalFormatting` (16) would
put a new `mergeCells` (14) on the wrong side of the `phoneticPr` if the `phoneticPr` were unranked —
a worksheet in schema-invalid order, written by this library. So a held element is ranked through the
same generated table, which still answers "no rank" for exactly what placement must step over: text,
a comment, an `mc:AlternateContent`, and any element in a namespace the complex type does not put
there. `crates/mjx-sml/tests/sheet_grid.rs` pins the interleaved case.

The same table is what makes an *insertion* stable: `crates/mjx-sml/tests/worksheet_spine.rs`'s
`the_emitted_children_are_in_generated_rank_order` writes the part out, parses it again, and checks
the ranks of what came back — so what is asserted is the sequence the writer produced rather than the
one the reader was handed.

## Copy-on-write, at a fourth granularity

`mjx-opc` gives copy-on-write per **part**; a [`RawElement`](mjx_ooxml_core::RawElement) gives it per
**subtree**; [the cell store](the_cell_store) restates it per **sheet, row and cell**. A frame
restates it once more, per **slot**:

* **The whole part.** Until anything is edited, writing the part out is one `extend_from_slice` of
  the buffer it was parsed from. Prologue, root start tag, every slot, the whitespace between them:
  all of it, unexamined.
* **One slot.** After an edit somewhere, every *other* slot still writes from its own bytes — a held
  child is a node that kept its source range, and a modelled one keeps the element it was read from
  beside the model.
* **Below `sheetData`.** The store's own three levels take over.

The rule that makes the slot level sound is **exactly one door**: a modelled slot's verbatim element
is dropped by the `_mut` accessor that hands out `&mut`, and by the setter that replaces it — the two
ways a caller can reach one — so *a slot whose bytes are still claimed is a slot nothing has been
able to change*. The element is **moved** out of the parsed tree rather than cloned, because a cloned
`RawElement` drops the verbatim source range and a move does not.

## What that buys, stated as behaviour

```
# fn main() -> Result<(), mjx_sml::SmlError> {
use mjx_sml::{CellRange, CellReference, WorksheetPart};

// A `mergeCells` carrying a foreign attribute in a namespace no schema here declares.
let markup = concat!(
    r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    r#"<sheetData/>"#,
    r#"<mergeCells count="1" xmlns:q="urn:example:q" q:note="keep me">"#,
    r#"<mergeCell ref="A1:B2"/></mergeCells>"#,
    "</worksheet>",
);
let mut sheet = WorksheetPart::read_part(markup.as_bytes())?.expect("an x:worksheet");

sheet.merge_cells(CellRange::Cells {
    start: CellReference::parse("C1")?,
    end: CellReference::parse("D2")?,
})?;

let emitted = String::from_utf8(sheet.to_markup()).expect("UTF-8");
assert!(emitted.contains(r#"q:note="keep me""#));
assert!(emitted.contains(r#"xmlns:q="urn:example:q""#));
// `@count` was declared, so it is updated rather than dropped or invented.
assert!(emitted.contains(r#"count="2""#));
# Ok(())
# }
```

That is `crates/mjx-sml/tests/sheet_grid.rs`'s
`editing_the_merges_keeps_the_attributes_the_file_wrote_on_the_element`, and it is the behavioural
half of the shape check in `crates/mjx-sml/tests/serialization_ledger.rs` —
[Fidelity and the known gaps](fidelity_and_gaps) says why both are needed.
