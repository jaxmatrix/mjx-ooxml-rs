# Large workbooks

The memory model in a caller's terms: what a sparse sheet costs, what a big one costs, **what is paid
on every call rather than once**, and what to do instead when that matters.

Read this before writing a loop that touches a lot of cells. The surface will not stop you, and at
small sizes nothing here is visible.

## The model in one sentence

**A worksheet is cheap to hold and expensive to open, and this library holds nothing between calls.**

`mjx-sml`'s cell store — the thing [`Workbook::worksheet_markup`] answers with — is three flat
vectors over one byte arena, so a populated cell costs **36.8 bytes** and an unpopulated one costs
nothing at all. But the store is built *from* a parsed document, that parse is where the memory and
the time go, and [`Workbook`] keeps no cache: it holds the package, the workbook part, the part graph
and the sheet list, and **not one parsed worksheet**. So every call that reaches into a sheet's cells
parses that sheet's part again.

## What a sparse sheet costs

Nothing proportional to the grid. A sheet whose only populated cell is `XFD1048576` — column 16,384,
row 1,048,576 — holds one row and one cell:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

let far_corner = CellReference::parse("XFD1048576")?;
let mut workbook = Workbook::blank()?;
workbook.set_cell_value(0, far_corner, CellValue::Number(1.0))?;

let markup = workbook.worksheet_markup(0)?.expect("a worksheet part");
assert_eq!(markup.row_count(), 1, "one populated row, not 1,048,576");
assert_eq!(markup.cell_count(), 1);
assert_eq!(markup.cell(far_corner).and_then(|cell| cell.number()), Some(1.0));
# Ok(())
# }
```

That is measured rather than asserted by inspection, because a `Vec` that reserved a million slots
reports the same twenty-four bytes as an empty one and would satisfy every structural check above.
`crates/mjx-sml/tests/cell_store_allocation.rs` installs a counting global allocator and holds the
whole part to **32 KiB** — against the 4 MiB a dense index over the addressable rows would cost — and
`crates/mjx-xlsx/examples/large_sparse_sheet.rs` asserts the same thing end to end through a real
`.xlsx` that was saved and reopened.

## What a populated cell costs

| Held as | Bytes per cell | Measured by |
|---|---:|---|
| A `RawElement` tree (the generic fidelity model) | **802** allocated / **913** peak RSS | `docs/BENCHMARKS.md` |
| The cell store, value cells | **36.8** | `cell_store_allocation.rs` case 2, bound 48 |
| The cell store, formula cells | **76.8** | case 5, bound 96 — 36 B of `PackedCell` plus 40 B of `CellExtras` |

A formula cell costs more because that is where the `<f>`'s byte range lives, and a **shared-group
member costs no more than any other formula cell**: there is no group table and no per-cell index
into one, so five hundred thousand cells in one group cost five hundred thousand cells and one
entry when you ask for the report.

A worksheet nobody has edited also owns **no bytes of its own**: every value it preserves is a range
into the part's buffer, which is what [`SheetData::edited_bytes`](mjx_sml::SheetData::edited_bytes)
reports as zero until something is written.

## ⚠ The cliff: every call re-parses the sheet

This is the awkward figure, and it is stated plainly because a caller who meets it without warning
will read it as a bug.

`target/corpus/workbook_large.xlsx` (`cargo run --release -p xtask -- corpus`) is 5,000 rows × 60
columns — **300,000 cells, 610,005 elements, an 8.60 MiB worksheet part** in a 1,235 KiB package. On
the machine `docs/BENCHMARKS.md` records, in a **release** build:

| Call | Time | Peak allocation | Retained |
|---|---:|---:|---:|
| [`Workbook::open`] | **14.8 ms** | 16.8 MB | the package's bytes |
| the first [`Workbook::worksheet_markup`] | **507 ms** | **269 MB** | 20.1 MB |
| [`Workbook::cell_text`] for **one** cell, on that same open workbook | **387 ms** | the same again | nothing |
| [`Workbook::set_cell_value`] for **one** cell | **485 ms** | the same again | nothing |
| [`Workbook::save`], nothing edited | 314 ms | — | — |
| [`Workbook::save`], after one edit | 930 ms | — | — |

Three things to take from that table:

1. **Opening is cheap.** 14.8 ms and 16.8 MB for a 1.2 MB package: `open` parses exactly one part,
   `xl/workbook.xml`, to read the sheet list. Nothing else is looked at.
2. **The first reach into a sheet's cells is not.** 507 ms and a 269 MB allocation peak, against
   20.1 MB retained. The peak is the parse: the whole part becomes a `RawElement` document, the cell
   store is built from its `sheetData`, and that tree is then dropped — so what survives is the
   store's 11.04 MB of records plus the 9.02 MB copy of the part's own bytes.
   `docs/BENCHMARKS.md` records the same shape from the other side — a `RawElement` tree over this
   sheet is 240 MB live, 802 B/cell — and says what it would take to avoid it: building the store
   straight from the part's bytes, which needs a streaming reader in `mjx-xml` that is not built.
3. **It is paid again on the next call.** `cell_text` on an *already open* workbook costs 387 ms,
   because it calls `worksheet_markup` itself. So does `set_cell_value`, `sheet_tables`,
   `merged_ranges`, `auto_filter`, `sheet_hyperlinks` and every other per-sheet accessor. This is a
   deliberate consequence of `Workbook` holding no mutable cache — reading never dirties a part, and
   there is nothing to invalidate — but at 300,000 cells it is the dominant cost of anything that
   asks twice.

### What to do about it

**Hold the [`mjx_sml::WorksheetPart`] yourself.** [`Workbook::worksheet_markup`] hands you an owned
model; [`Workbook::write_worksheet_markup`] takes it back. Between those two calls, reads and edits
go straight to the store and cost microseconds.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::blank()?;

// One parse, N edits, one serialize.
let mut markup = workbook.worksheet_markup(0)?.expect("a worksheet part");
let sheet = markup.sheet_data_or_insert();
for row in 0..500u32 {
    sheet.set_cell_value(CellReference::relative(0, row)?, CellValue::Number(f64::from(row)))?;
}
workbook.write_worksheet_markup(0, &markup)?;

let reopened = Workbook::open(&workbook.save()?)?;
assert_eq!(reopened.cell_text(0, CellReference::parse("A500")?)?.as_deref(), Some("499"));
# Ok(())
# }
```

The same 500 cells written one [`Workbook::set_cell_value`] at a time produce the identical file and
take **two thousand times longer**, because each call parses and re-serializes the whole part. The
cost is quadratic in the number of cells, and it is measurable at sizes a caller will reach:

| Cells | One `set_cell_value` per cell | One read / N edits / one write | Ratio |
|---:|---:|---:|---:|
| 500 | 243.5 ms | 121.8 µs | 1,999× |
| 1,000 | 1.02 s | 240.8 µs | 4,220× |
| 2,000 | 4.25 s | 579.7 µs | 7,328× |
| 4,000 | 18.39 s | 1.12 ms | 16,427× |

Doubling the cells quadruples the left-hand column and doubles the right-hand one. Both were measured
in a release build on the machine `docs/BENCHMARKS.md` describes.

**[`Workbook::set_cell_value`] is the right call for the operation it is named for** — one cell, on a
workbook you opened to change one cell — and the wrong one for filling a sheet. Nothing here will
warn you; that is what this page is for.

### One copy you cannot avoid today

[`Workbook::worksheet_markup`] reads the part through
[`WorksheetPart::read_part`](mjx_sml::WorksheetPart::read_part), which takes `&[u8]` and copies it
into an `Arc<[u8]>` for the store to point into. The package is already holding those bytes for its
own copy-on-write, and [`WorksheetPart::read_shared`](mjx_sml::WorksheetPart::read_shared) exists
precisely for a caller who can hand them over shared — but [`mjx_opc::Package::part_bytes`] answers
`&[u8]`, so there is no way to reach the shared path from here. On the 8.60 MiB worksheet above that
copy is 8.60 MiB of the 20.1 MB the store retains, and it is why the example measures **66.9 B/cell**
against the store's own **36.8**.

It is written down rather than fixed here because closing it means a new accessor on `mjx-opc`, which
is a change to the packaging tier and not a documentation child's to make.

## Saving is compression, not this library

`save` on the 300,000-cell workbook is **314 ms untouched** and **930 ms after one edit**, and
`docs/BENCHMARKS.md` explains the floor under both: DEFLATE-compressing 8.6 MiB runs at roughly
28 MiB/s, against `inflate`'s 738 MiB/s on the way in. The copy-on-write saving between an untouched
save and a fully-materialised one is real and is a minority of the total.

So for a large workbook, most of the wall clock a caller experiences on `save` is the ZIP, not this
crate's serialization — worth knowing before optimising the smaller share.

## The shape to reach for

* **Reading a few cells of a big sheet:** take [`Workbook::worksheet_markup`] once and read from the
  model. `cell_text` per cell is a parse per cell.
* **Writing many cells:** one `worksheet_markup`, N `SheetData::set_cell_value`, one
  `write_worksheet_markup`. Write **top to bottom, left to right** — that is append-only in the cell
  arena; inserting backwards moves the arena's tail, which is a `memmove` proportional to the cells
  after the insertion point.
* **Asking whether a workbook has pivot tables, external links or connections:**
  [`Workbook::preserved_parts`] and the reports beside it resolve from **relationships alone, with no
  markup parsed at all** — see [Fidelity and the part graph](fidelity_and_the_part_graph).
* **Opening a file to look at its tabs:** [`Workbook::open`] and [`Workbook::sheets`] never touch a
  worksheet part.
