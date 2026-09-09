# Fidelity and the known gaps

The reason this project exists is on this page. Read it before you rely on any of it in production.

## The contract

**Open a file, edit one thing, save it: every part you did not touch comes back byte for byte.**
Precisely — per-part decompressed-payload byte identity, plus structural container identity. Not
identical ZIP bytes: the compression level and the entry order are the container's business, not the
document's.

Three mechanisms make it true, and none of them is this crate's:

* **Part-level laziness and copy-on-write** — `mjx_opc` holds every part as raw bytes and
  materialises one into a typed model only when something asks. A part nobody asked about was never
  parsed, so it cannot have been re-serialised differently.
* **The byte-preserving reader** — `mjx_xml::fidelity` keeps attribute order, namespace prefixes,
  self-closing form and whitespace as the file wrote them, so a part that *was* parsed and re-emitted
  is still the same bytes unless an edit changed it.
* **The unknown bucket** — every modelled complex type carries an `extra: Vec<RawNode>` for children
  this library does not model, so an edit to a shape's fill carries the producer's own extension
  elements through untouched.

What this crate adds is **no re-serialisation of its own**. Every method delegates to exactly one
method one layer down, and `crates/mjx-ooxml/tests/preservation/main.rs` is the gate that proves the
delegation kept the contract: every committed fixture under `tests/fixtures/`, crossed with every
mutating method of all three surfaces, checked part by part.

```
use mjx_ooxml::{Deck, Surface};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let original = mjx_fixtures::fixture("charts.pptx");
let mut deck = Deck::open(&original)?;

// One edit, on the chart that `charts.pptx` carries as the first shape of its second slide.
deck.set_chart_title(Surface::Slide(1), 0.into(), Some("Revised"))?;
let saved = deck.save()?;

// Compared at the container, because a `Deck` has no general part door — see the gaps below.
let before = mjx_opc::Package::open(&original)?;
let after = mjx_opc::Package::open(&saved)?;
let names: Vec<_> = before.part_names().collect();
assert_eq!(names, after.part_names().collect::<Vec<_>>(), "no part appeared or vanished");

let changed: Vec<String> = names
    .iter()
    .filter(|part| before.part_bytes(part) != after.part_bytes(part))
    .map(|part| part.as_str().to_owned())
    .collect();
assert_eq!(changed.len(), 1, "one edit touched {changed:?}");
assert!(changed[0].contains("chart"), "and it was the chart part: {}", changed[0]);
# Ok(())
# }
```

## Nothing is repaired, and nothing is evaluated

Two standing refusals, and each looks like a defect from a different angle.

**Nothing is repaired on read.** A `dimension` that disagrees with the cells, a duplicate row number,
a merge overlapping another, a shape with no bounds — each is *reported* (as a `SpreadsheetDefect`,
as a [`GridAnomalyInfo`], as a `PresentationDefect`, or as `None` from a method that cannot honestly
answer) and none is corrected. Correcting a file to match what this library expects is how a fidelity
library loses the argument it exists to win.

**Nothing is evaluated.** There is no calculation engine and there will not be one, which has three
consequences worth spelling out: a formula's cached `<v>` goes stale after an edit and is left
exactly as the producer wrote it; a conditional-formatting rule is reported and never resolved; a
filter, a sort and a validation rule are recorded and never applied. Each is a decision with a reason
and a workaround, and [*Deliberate limitations*](mjx_xlsx::guide::deliberate_limitations) is the page
that gives all three.

## What is preserved rather than modelled

A large amount of every format is recognised, reported and deliberately not modelled — pivot tables
and their caches, shared-workbook revisions, external references, cell metadata, data connections,
query tables, custom XML mappings, the legacy VML that draws an OLE object's fallback. **Preserved is
not ignored and it is not lost**: every part of every one of those comes back byte for byte through
an unrelated edit, and each has a reader here that says what it is without a model behind it.

<!-- guide-example: preserved_rather_than_modelled rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let original = mjx_fixtures::fixture("preserved_parts.xlsx");
use mjx_ooxml::Workbook;

let workbook = Workbook::open(&original)?;

// Inventoried from the relationships, with no markup parsed at all.
let summary = workbook.preserved_parts()?;
assert!(!summary.pivot_tables.is_empty());
assert!(!summary.pivot_cache_definitions.is_empty());

// And resolved far enough to say which tab each one sits on.
for table in workbook.pivot_tables()? {
    assert!(!table.sheet_name.is_empty());
}
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: preserved_rather_than_modelled python -->
```python
from mjx_ooxml import Workbook

workbook = Workbook.open(original)

# Inventoried from the relationships, with no markup parsed at all.
summary = workbook.preserved_parts()
assert summary.pivot_tables
assert summary.pivot_cache_definitions

# And resolved far enough to say which tab each one sits on.
for table in workbook.pivot_tables():
    assert table.sheet_name
```
<!-- guide-example end -->

<!-- guide-example: preserved_rather_than_modelled js -->
```js
import { Workbook } from "@mjx/ooxml";

const workbook = Workbook.open(original);

// Inventoried from the relationships, with no markup parsed at all.
const summary = workbook.preservedParts();
if (summary.pivotTables.length === 0 || summary.pivotCacheDefinitions.length === 0) {
  throw new Error("this workbook carries pivot tables and the caches they read");
}

// And resolved far enough to say which tab each one sits on.
for (const table of workbook.pivotTables()) {
  if (table.sheetName === "") {
    throw new Error("every pivot table names the tab it sits on");
  }
  table.free();
}

// a wasm handle owns memory the garbage collector cannot see
summary.free();
workbook.free();
```
<!-- guide-example end -->

`original` is the file's bytes, read by each runner before the block starts: this library is bytes in
and bytes out and never touches a filesystem. The example saves nothing, because reading is the whole
of what it demonstrates — so the two binding harnesses have no package to compare, and the three
halves agreeing about producing nothing is itself checked.

**A documented gap is not a validation failure.** Nothing on that list is a defect to be filed, and
the per-format pages say which clusters each format holds:
[Fidelity and gaps](mjx_pptx::guide::fidelity_and_gaps) for PowerPoint,
[Fidelity and gaps](mjx_docx::guide::fidelity_and_gaps) for Word,
[Fidelity and the part graph](mjx_xlsx::guide::fidelity_and_the_part_graph) for Excel.

## Gaps rather than decisions

These are absent, they are not the consequence of a decision above, and they are named here so
nobody plans around a surface that is not present.

| Absent | What exists instead |
|---|---|
| **Setting the document properties on an authored file.** `blank_with_properties` is on all three model types and on none of the three facade surfaces | [`Deck::blank`], [`Document::blank`] and [`Workbook::blank`] write both `docProps` parts with this library's defaults; an opened file keeps its own untouched |
| **Writing a formula into a cell.** `mjx_sml::CellFormula` is a read-only view over a cell's `<f>`, and no `set_cell_formula` exists anywhere | A formula round-trips because nothing rewrites a cell it was not asked to; a Rust caller can write the `<c>` markup through [`Workbook::workbook_mut`], and no binding caller can |
| **Removing a sheet.** [`Workbook::add_sheet`] has no opposite | Removing a tab means removing a part, its relationship, its entry and every defined name scoped to it — a decision rather than a convenience method |
| **Listing the parts of a deck or a document.** [`Workbook::part_names`], [`Workbook::part_bytes`] and [`Workbook::content_type_of`] have no `Deck` or `Document` counterpart | Content-specific byte windows are there on all three — [`Deck::chart_part_bytes`], `Document::chart_part_bytes`, [`Deck::ink_part_bytes`] — but the general package door is Excel's alone |

## Built, not yet verified against Microsoft Office

A different list, and it is neither of the two above. Everything on this surface **works** and is
tested — 3,100-odd cases, both feature modes, zero failures — against markup **we wrote**. What none
of it has is a run through real Microsoft Office.

Every fixture under `tests/fixtures/` was authored by this project or by LibreOffice, so every gate
in the workspace proves that our reader agrees with our writer. **LibreOffice is a change detector,
never the reference** — rendering parity is judged against Microsoft Office on Windows and nothing
else. What exists is the road for changing that: `tests/office-authored/` with its redistribution
rule, `cargo run -p xtask -- validation-artefacts --ingest` to report on a file before it is
committed, and `xtask/tests/office_corpus.rs`, which holds whatever lands there to per-part byte
identity at the container *and* through this facade. **The corpus is empty**, and no agent may fill
it: a file's value there is entirely its provenance. `docs/validation/06-the-office-pass.md` is how a
person with Office fills it.

That is also why this project has no tag. `v0.1` asserts validation against real Office, and it has
not happened; `docs/validation/00-method.md` is the method, and it is deliberately a method a person
runs.
