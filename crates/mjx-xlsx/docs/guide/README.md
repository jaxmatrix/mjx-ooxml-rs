# Guide

Six pages. Excel is the last of the three formats this workspace takes on: MJXOFF-91 built the
**package** — the container, the part graph, and a `Workbook` that opens and saves without touching a
byte — and the Phase D children after it are building the model reached through it. MJXOFF-102 (D07)
adds the worksheet: a sheet's cells can now be read and one of them written. MJXOFF-112 (D10) adds
the other direction — `Workbook::blank` and the authoring surface, every byte of whose markup comes
from `mjx-sml`. MJXOFF-115 (D11) adds formulas, which this library carries as text and never
calculates. MJXOFF-117 (D12) adds the sheet grid — merging, row and column geometry, outline levels,
page breaks and sheet protection. This guide says exactly that much and no more, so that nobody plans
around a surface that is not here.

| Page | Read it when |
|---|---|
| [Opening and saving a workbook](opening_and_saving) | You want the whole of the current surface, once |
| [Reading and editing cells](reading_and_editing_cells) | You want a value out of a sheet, or one into it |
| [Authoring a workbook](authoring_a_workbook) | You want a workbook this library wrote, rather than one it opened |
| [Formulas and cached values](formulas_and_cached_values) | Before you edit a workbook that has formulas in it |
| [The sheet grid](the_sheet_grid) | You want to merge cells, size a row or a column, or read a protected sheet |
| [Fidelity and the part graph](fidelity_and_the_part_graph) | Before you rely on anything here in production |

Every snippet on every page is a compiled doctest that `cargo test` runs, and every one asserts on a
value it computed — the same rule `mjx-pptx`'s and `mjx-docx`'s guides are held to, and what keeps a
guide from drifting away from the API it describes.

## The shape of the API, in one page

**Bytes in, bytes out.** [`Workbook::open`] takes `&[u8]` and [`save`](Workbook::save) returns
`Vec<u8>`. The library never touches a filesystem, a network or a clock; whoever calls it owns the
file handle, which is also why the same code compiles to WebAssembly and runs in a browser.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("in.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;
std::fs::write("out.xlsx", workbook.save()?)?;
# Ok(())
# }
```

**Excel is two crates, and this is the package half.** `mjx-sml` owns SpreadsheetML *markup* — what a
cell, a row, a shared string or an `xf` is — because an embedded workbook inside a `.pptx` or a
`.docx` is SpreadsheetML too, and `mjx-chart` has to be able to reach it without pointing sideways at
a format crate. `mjx-xlsx` owns OPC structure: parts, content types, relationships, the ZIP, and the
[`Workbook`] a caller holds. So a `mjx_sml::WorksheetPart` is what a sheet's markup *is*, and this
crate is what finds the part it lives in.

**Reading never dirties a part.** Opening a workbook parses exactly one part — `xl/workbook.xml`, to
read its sheet list — and parsing is not mutating: the part keeps its container bytes and `save`
re-emits them verbatim. Reading a worksheet does not even parse it into the package: the bytes are
parsed into a model that outlives the tree, and the part stays exactly as the container held it. Only
[`Workbook::set_cell_value`] and [`Workbook::write_worksheet_markup`] change anything, and each
changes one part.

**Saving validates.** [`Workbook::save`] runs [`Workbook::validate`] first, and refuses to write a
package that breaks a packaging invariant or a SpreadsheetML one. [`Workbook::save_unchecked`] is the
deliberate escape hatch for writing back a container that arrived broken.
