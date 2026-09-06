# Through the facade and the bindings

Every other page in this guide is written against [`Workbook`] — this crate's own type, the one a
Rust caller reaches for. This page is about the *other* three surfaces the same workbook is reached
through, and about the one place where they deliberately do not agree with this one.

| Surface | Type | Written against |
|---|---|---|
| Rust, this crate | [`Workbook`] | every other page here |
| Rust, the facade | `mjx_ooxml::Workbook` | `crates/mjx-ooxml/examples/build_a_workbook.rs` |
| Python | `mjx_ooxml.Workbook` | `bindings/mjx-python/tests/test_build_a_workbook.py` |
| TypeScript / JavaScript | `Workbook` from `@mjx/ooxml` | `bindings/mjx-wasm/tests/node/build_a_workbook.mjs` |

Those three walkthroughs are the *same walkthrough*, and the last two are compared against the first
**part by part, byte for byte**. A method wired to the wrong `Workbook` method changes one payload
and fails there.

## The translation, in one table

The facade is this crate's surface with its Rust ergonomics traded for portability — the same trade
`mjx_ooxml::Deck` makes for `mjx_pptx::Presentation` and `mjx_ooxml::Document` for
`mjx_docx::Document`:

| `mjx_xlsx::Workbook` | `mjx_ooxml::Workbook` | why |
|---|---|---|
| [`mjx_sml::CellReference`] | `&str` — `"B7"` | an address a binding would have to wrap in a class, for a value every spreadsheet user already spells |
| [`mjx_sml::CellRange`] | `&str` — `"A1:C3"` | likewise |
| [`mjx_sml::CellValue`] | `CellData` / `CellInput` | a borrowed enum cannot outlive the call in a binding |
| `usize` | `u32` | one width on every target |
| [`PartName`] | `&str` | a validated handle cannot cross the boundary and come back |
| `impl FnOnce(&T, &Interner) -> R` | a concrete return type | neither PyO3 nor wasm-bindgen can accept a Rust closure |
| [`XlsxError`] | `mjx_ooxml::Error` | eleven variants, and the forty-two below them, collapse to eleven stable codes |

Everything else is the same call with the same name. `sheets`, `add_sheet`, `rename_sheet`,
`merged_ranges`, `merge_cells`, `append_font`, `set_cell_style`, `effective_cell_format`,
`sheet_hyperlinks`, `sheet_tables`, `defined_names`, `preserved_parts` — all of them are there,
spelled identically in Rust and Python and in `camelCase` in TypeScript.

## ⚠ The one place they do not agree: cells cross a **range** at a time

**There is no per-cell reader or writer on the facade, or in either binding.** This crate has both —
[`Workbook::cell_text`] and [`Workbook::set_cell_value`] — and they are the right calls for the
operation they are named for. They are the wrong calls to project.

[Large workbooks](large_workbooks) has the measurements. In short: this crate's `Workbook` holds no
parsed worksheet, so every per-sheet accessor parses that sheet's part again — **387 ms to read one
cell** of the 300,000-cell corpus workbook, against 14.8 ms to open the whole file, and a 4,000-cell
write loop at **18.39 s** where one read, 4,000 edits and one write take **1.12 ms**. That page also
gives the answer: hold the [`mjx_sml::WorksheetPart`] yourself between one
[`Workbook::worksheet_markup`] and one [`Workbook::write_worksheet_markup`].

**That answer does not cross a foreign function boundary.** `WorksheetPart` is an interner-bound
model; a binding cannot hand it out, and a Python or JavaScript caller has no escape hatch to reach
it with. A facade that offered `cell_value(sheet, "A1")` would be shipping the 387 ms-per-cell loop
as the natural idiom, in the one place a caller *cannot* reach past it — so the facade offers a range
in both directions instead:

| call | parses | serializes |
|---|---:|---:|
| `read_range(sheet, range)` | 1 | 0 |
| `read_sheet(sheet)` | 1 | 0 |
| `write_cells(sheet, cells)` | 1 | 1 |

Those counts do not depend on how many cells were asked for, and
`crates/mjx-ooxml/tests/workbook_boundary.rs` holds them to a **deterministic allocation ratio**
rather than to a stopwatch: on a 2,000-cell sheet, 200 cells written batched allocate **209× less**
than the same 200 written one at a time, and 200 read batched **199× less**. A change that quietly
made either call loop per-cell would still produce the same file, byte for byte — which is exactly
why the gate is not about correctness.

Nothing is lost for the one-cell case. `read_range(0, "A1")` is one call and one parse, exactly what
a per-cell reader would have cost. What is lost is the *shape* that invites the loop.

Rust callers who want the per-cell calls anyway still have them, in full, through
`mjx_ooxml::Workbook::workbook_mut` — the same escape hatch `mjx_ooxml::Deck::presentation_mut` is.

## Reading a block

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

// Built here through this crate, so the page needs no fixture; the facade opens the same bytes.
let mut workbook = Workbook::blank()?;
let mut markup = workbook.worksheet_markup(0)?.expect("a worksheet part");
markup.set_cell_value(CellReference::parse("A1")?, CellValue::InlineString("Region"))?;
markup.set_cell_value(CellReference::parse("B1")?, CellValue::Number(12.5))?;
workbook.write_worksheet_markup(0, &markup)?;
let bytes = workbook.save()?;

// Through the facade, the same two cells arrive in one call:
//
//     let workbook = mjx_ooxml::Workbook::open(&bytes)?;
//     let block = workbook.read_range(0, "A1:B1")?;
//     assert_eq!(block.value(0, 0)?.text(), Some("Region"));
//     assert_eq!(block.value(0, 1)?.number(), Some(12.5));
//
// It is written as a comment because this crate sits *below* the facade and cannot name it — the
// same reason `mjx-pptx`'s guide cannot name `mjx_ooxml::Deck`. The executable copy is
// `crates/mjx-ooxml/examples/build_a_workbook.rs`.
let reopened = Workbook::open(&bytes)?;
assert_eq!(reopened.cell_text(0, CellReference::parse("A1")?)?.as_deref(), Some("Region"));
assert_eq!(reopened.cell_text(0, CellReference::parse("B1")?)?.as_deref(), Some("12.5"));
# Ok(())
# }
```

In Python the block comes back as native values:

```python
block = workbook.read_range(0, "A1:B1")
assert block.rows() == [["Region", 12.5]]
assert block.kinds() == [["text", "number"]]
```

and in TypeScript as arrays of `string | number | boolean | null`:

```js
const block = workbook.readRange(0, "A1:B1");
assert.deepEqual(block.rows(), [["Region", 12.5]]);
block.free();
```

`rows()` cannot tell a text cell from an error cell — both arrive as a string — so `kinds()` is the
disambiguator, built only when asked.

## Writing a batch

```python
workbook.write_cells(0, [
    mjx_ooxml.CellWrite.shared_text("A1", "Region"),
    mjx_ooxml.CellWrite.shared_text("B1", "Growth"),
    mjx_ooxml.CellWrite.number("B2", 12.5),
])
```

`shared_text` interns into `xl/sharedStrings.xml`, which is what Excel writes when the same text
repeats; `inline_text` stores the string in the cell itself and touches no other part. Both read back
as the same `CellData.text`, and the two exist as separate calls because the choice is a real
difference in the file rather than something this library should pick for you.

A batch is applied **in the order given**, so prefer top to bottom, left to right: that is
append-only in the cell arena, where an out-of-order insertion moves the arena's tail. A batch
carrying an address that does not parse writes **nothing** — every reference is parsed before the
worksheet is opened.

## What the facade does *not* carry

The facade is curated, not a re-export. Four clusters stay behind, each reachable through
`mjx_ooxml::Workbook::workbook_mut`:

* **The closure-taking markup doors** — [`Workbook::workbook_markup`],
  [`Workbook::edit_workbook_markup`], [`Workbook::worksheet_markup`], [`Workbook::table_markup`],
  [`Workbook::auto_filter`] and the rest. A closure over an interner-bound reference is the one shape
  a foreign function boundary cannot carry.
* **The borrowed views** — [`Workbook::worksheet`], [`Workbook::sheet_formatting`],
  [`Workbook::package`]. Each borrows the workbook for a caller-controlled lifetime, which neither
  PyO3 nor wasm-bindgen can express. What they *answer* is on the facade as owned data.
* **The authoring vocabularies for conditional formatting, autofilters, data validation and
  worksheet tables** — each takes an `mjx-sml` spec *tree* rather than a flat struct. What each of
  them produces is readable through the facade (`auto_filter_range`, `data_validation_ranges`,
  `conditional_formatting_ranges`, `sheet_tables`), so a caller can see a feature it cannot yet
  author, and leave it alone.
* **The cell-format vocabulary is projected**, because it is four flat structs rather than a tree —
  and because a caller who cannot make a style index cannot use `set_cell_style`.

## `.xlsb` is refused by design, not by schedule

`mjx_ooxml::detect_format` recognises five spreadsheet formats. Four of them open. The fifth,
`Format::WorkbookBinary` (`.xlsb`), is a conforming OPC package whose main part is the MS-XLSB binary
record stream and **not SpreadsheetML at all** — there is no markup in it for this crate to read. It
is refused with an `UnsupportedFormat` error whose message says exactly that, in different words from
the one a `.pptx` gets, because it is a different fact. Nothing later in this project's roadmap
changes it.

## The fidelity contract survives the boundary

A workbook opened, untouched and saved **through either binding** is byte-identical part by part —
asserted by `test_build_a_workbook.py` and `build_a_workbook.mjs`, each of which reads
`tests/fixtures/sample.xlsx`, saves it and compares every decompressed payload. Everything
[Fidelity and the part graph](fidelity_and_the_part_graph) states is inherited whole; the facade adds
no re-serialization of its own.

[`Workbook`]: crate::Workbook
[`Workbook::cell_text`]: crate::Workbook::cell_text
[`Workbook::set_cell_value`]: crate::Workbook::set_cell_value
[`Workbook::worksheet_markup`]: crate::Workbook::worksheet_markup
[`Workbook::write_worksheet_markup`]: crate::Workbook::write_worksheet_markup
[`Workbook::workbook_markup`]: crate::Workbook::workbook_markup
[`Workbook::edit_workbook_markup`]: crate::Workbook::edit_workbook_markup
[`Workbook::table_markup`]: crate::Workbook::table_markup
[`Workbook::auto_filter`]: crate::Workbook::auto_filter
[`Workbook::worksheet`]: crate::Workbook::worksheet
[`Workbook::sheet_formatting`]: crate::Workbook::sheet_formatting
[`Workbook::package`]: crate::Workbook::package
[`XlsxError`]: crate::XlsxError
[`PartName`]: crate::PartName
