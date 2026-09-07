# The curated surface

This crate is **not** a re-export of the three format crates. It is a selection from each, reshaped
so that the same calls work from Rust, Python and TypeScript — and the selection is the interesting
part, because a method that is here and a method that is not are two different decisions with two
different reasons.

## What changes on the way up

| One layer down | Here | why |
|---|---|---|
| `impl Into<Surface>`, `impl Into<ShapePath>` | [`Surface`], [`ShapePath`] | a generic parameter has no foreign representation |
| `usize` | `u32` | one width on every target, host-independent |
| `&mjx_opc::PartName` | `&str` | a validated handle cannot cross the boundary and come back |
| `Option<&[u8]>` | `Option<Vec<u8>>` | a borrow of the document cannot outlive the call in a binding |
| `impl FnOnce(&T, &Interner) -> R` | a concrete return type | neither PyO3 nor wasm-bindgen accepts a Rust closure |
| `mjx_sml::CellReference`, `mjx_sml::CellRange` | `&str` — `"B7"`, `"A1:C3"` | an address every spreadsheet user already spells |
| `mjx_dml::spreadsheet_drawing::CellMarker` | four plain numbers | wrapping a marker in a class buys a caller nothing |
| `mjx_pptx::PptxError`, `mjx_docx::DocxError`, `mjx_xlsx::XlsxError` | [`Error`] | see [Errors](errors) |

The facade's own count of that reshaping is zero behaviour: **every method here delegates to exactly
one method there**, and `crates/mjx-ooxml/tests/delegate_wiring.rs` and
`crates/mjx-ooxml/tests/document_delegate_wiring.rs` are what prove each one calls the method it is
named after. Every assertion in both is asymmetric on purpose — nothing is set to the same value as
its neighbour and nothing is read at an index that would answer the same at another — so a delegate
wired to `row_height` instead of `column_width` fails there even though the method it wrongly calls
works perfectly.

## What stays behind, and why

Six reasons cover almost all of it. None of them is "we ran out of time".

* **A closure over an interner-bound reference.** `mjx_docx::Document::edit_style_sheet`,
  `mjx_xlsx::Workbook::workbook_markup` and some forty siblings take
  `impl FnOnce(&T, &Interner) -> R`. A foreign function boundary cannot carry a Rust closure, and a
  callback would re-enter the document while it is mutably borrowed. **Where a concrete answer
  exists, this facade builds it by calling the closure-taking method internally** — that is exactly
  what [`Workbook::auto_filter_range`] and [`Workbook::data_validation_ranges`] are, and what the
  whole of `crate::document`'s styles, numbering and header surface is.
* **A borrowed view.** `mjx_pptx::Presentation::shape` returns a cursor holding
  `&'deck mut Presentation`; `mjx_xlsx::Workbook::worksheet` borrows the workbook for a
  caller-controlled lifetime. Neither PyO3 nor wasm-bindgen can express a struct that borrows another
  object for a lifetime the caller chooses.
* **An interner-bound model.** `mjx_xlsx::Workbook::worksheet_markup` and
  `mjx_xlsx::Workbook::write_worksheet_markup` take no closure and borrow nothing — they hand back an
  owned `mjx_sml::WorksheetPart`. What they cannot do is cross a boundary, because every string in
  one is an index into an interner that stays behind. (Both were filed under *closure doors* in this
  crate's own documentation until MJXOFF-214; they are neither.)
* **Part-graph identity.** `mjx_pptx::Presentation::chart_rel_id` and its five siblings hand out
  relationship ids for content whose *bytes* are readable directly through
  [`Deck::chart_part_bytes`] and friends. The part-addressed readers that are the **only** door to
  their content — the ink, VML and diagram byte windows — are kept, with `&str` part names.
* **A spec tree.** `mjx_xlsx::Workbook::add_conditional_formatting` takes a
  `mjx_sml::ConditionalRuleSpec`, which reaches five further spec types; `set_auto_filter` reaches
  five more. Projecting five builder trees is a surface of its own rather than a row in a table —
  and **what each of them produces is readable here**, through
  [`Workbook::conditional_formatting_ranges`], [`Workbook::auto_filter_range`],
  [`Workbook::data_validation_ranges`] and [`Workbook::sheet_tables`], so a caller can see a feature
  it cannot yet author and leave it alone.
* **The sealed package.** `from_package` and `package` take or hand back an `mjx_opc::Package`.
  Sealing it is what makes the fidelity contract this crate's to keep rather than a caller's to
  break.

## The list is checked, not trusted

Each of the three module doc comments — `crates/mjx-ooxml/src/deck.rs`,
`crates/mjx-ooxml/src/document.rs` and `crates/mjx-ooxml/src/workbook.rs` — carries that list for its
own surface, and `xtask/tests/facade_curation.rs` compares it against the **real** difference between
the two surfaces, in both directions. A method added to `mjx_pptx::Presentation` and not projected
fails there until somebody classifies it — with one of the six reasons above, or as a rename, a
supersession, a cluster the curated surface does not carry, or a gap; an entry naming a method since
projected, or since deleted, fails there too.

That gate exists because the prose it replaced was wrong three ways at once. `deck.rs` said
*sixteen* methods were absent when the difference was **eighteen**; `workbook.rs` said
`mjx_xlsx::Workbook` carried *roughly seventy* public methods when it carried **165**; and
`document.rs` and `deck.rs` each quoted an error-variant count that had grown by six and by two.
**A number in prose can only be right on the day it is written; a list can be compared** — which is
MJXOFF-198 §4's *every count is a fact that expires* in its purest form (MJXOFF-214).

## The three escape hatches

[`Deck::presentation_mut`], [`Document::document_mut`] and [`Workbook::workbook_mut`] hand back
`&mut mjx_pptx::Presentation`, `&mut mjx_docx::Document` and `&mut mjx_xlsx::Workbook`. Everything
above is reachable through them, in full, with no loss of fidelity — the facade holds the real value
and never a copy.

```
use mjx_ooxml::Workbook;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;

// The per-sheet editing loop the facade deliberately does not offer: hold the parsed worksheet
// yourself between one read and one write, and pay for the parse once.
let inner = workbook.workbook_mut();
let mut markup = inner.worksheet_markup(0)?.expect("a worksheet part");
markup.set_cell_value(mjx_sml::CellReference::parse("A1")?, mjx_sml::CellValue::Number(1.0))?;
inner.write_worksheet_markup(0, &markup)?;

assert_eq!(workbook.read_range(0, "A1")?.value(0, 0)?.number(), Some(1.0));
# Ok(())
# }
```

**They are Rust-only, and that is the point of the whole page.** Neither binding exposes one, so
anything reachable only through an escape hatch is reachable only from Rust. Two consequences are
worth naming rather than leaving to be discovered:

* **Authoring a formula.** `mjx_sml::CellFormula` is a read-only view over a cell's `<f>` bytes, and
  there is no `set_cell_formula` anywhere in this workspace. A Rust caller can write the `<c>` markup
  through the hatch above; a Python or TypeScript caller cannot write a formula at all, and cannot
  round-trip one they read with [`CellBlock::formula`]. Writing a *value* into a cell that carries a
  formula leaves the `<f>` exactly as it was and replaces only its cached `<v>` — so the written
  value does not survive Excel's next recalculation. [`Workbook::write_cells`] says so on itself.
* **Removing a sheet.** [`Workbook::add_sheet`] has no opposite on any of the four surfaces.
  Removing a tab means removing a part, its relationship, its `sheets` entry and every defined name
  scoped to it, which is a decision rather than a convenience method.

## One gap that is a gap

`mjx_pptx::Presentation`, `mjx_docx::Document` and `mjx_xlsx::Workbook` each carry a
`blank_with_properties` taking an `mjx_opc::doc_props::CoreProperties` and an `ExtendedProperties`,
so a file authored from nothing can name a title, a creator and a created time. **Nothing on this
facade projects it**, so no caller in any of the three languages can set them. It is the **only**
method `xtask/tests/facade_curation.rs`'s ledger classifies as a gap rather than a decision, and it
is on all three of that file's ledgers — the same gap, three times. `blank` writes both `docProps` parts with this library's own
defaults, and a file opened from disk keeps the ones it came with, untouched — which is the standing
rule that a default is supplied only in the absence of the user's own, never in place of it.
