# Guide

Two pages. Excel is the last of the three formats this workspace takes on, and `mjx-xlsx` is at the
start of it: MJXOFF-91 builds the **package** — the container, the part graph, and a `Workbook` that
opens and saves without touching a byte — and the eighteen Phase D children after it build the model
that is reached through it. This guide says exactly that much and no more, so that nobody plans
around a surface that is not here.

| Page | Read it when |
|---|---|
| [Opening and saving a workbook](opening_and_saving) | You want the whole of the current surface, once |
| [Fidelity and the part graph](fidelity_and_the_part_graph) | Before you rely on anything here in production |

Every snippet on both pages is a compiled doctest that `cargo test` runs, and every one asserts on a
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
[`Workbook`] a caller holds. If you are looking for a cell, you are looking for `mjx-sml`, and for
the moment you will not find one there either.

**Reading never dirties a part.** Opening a workbook parses exactly one part — `xl/workbook.xml`, to
read its sheet list — and parsing is not mutating: the part keeps its container bytes and `save`
re-emits them verbatim. Nothing in this crate's current surface can dirty a part at all, which is why
open-then-save is a byte-exact round trip of the whole container.

**Saving validates.** [`Workbook::save`] runs [`Workbook::validate`] first, and refuses to write a
package that breaks a packaging invariant or a SpreadsheetML one. [`Workbook::save_unchecked`] is the
deliberate escape hatch for writing back a container that arrived broken.
