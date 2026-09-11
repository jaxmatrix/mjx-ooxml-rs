# Reaching SpreadsheetML

Who reaches this crate, for what, and where the line with `mjx-xlsx` is drawn. Read this before
deciding that an answer is missing: about half the time it is one tier up, and about half of the rest
it is deliberately absent and [written down as such](fidelity_and_gaps).

## Excel is two crates

| Crate | Owns | Reach for it when |
|---|---|---|
| **`mjx-sml`** (here) | SpreadsheetML *markup*: what a cell, a row, a shared string, an `xf`, a formula or a conditional-formatting rule **is** | You are holding a [`mjx_sml::WorksheetPart`](crate::WorksheetPart) and want the model under it, or you are building a package rather than opening one |
| **`mjx-xlsx`** | OPC structure: parts, content types, relationships, the ZIP, and the `Workbook` a caller holds | You have a `.xlsx` and want to open, read, edit or save it |

The split is not tidiness, and it is not about Excel. It is the layering rule applied to a fact about
the format: **an embedded workbook is SpreadsheetML inside a `.pptx` or a `.docx`.** An authored
PowerPoint chart carries a whole `.xlsx` at `/ppt/embeddings/*.xlsx`, and a Word chart carries the
same, so `mjx-chart` — rank 2.2, shared markup — has to be able to *write* a workbook. An edge from
`mjx-chart` to `mjx-xlsx` (3.0) would point upward and an edge from `mjx-pptx` to `mjx-xlsx` would
point sideways; `CLAUDE.md` forbids both. Putting the markup at rank 2.1 makes
`mjx-chart → mjx-sml → mjx-dml` three legal downward edges, and it is what let `mjx-chart`'s own
duplicate workbook writer (`EmbeddedWorkbook`) be deleted rather than maintained.

`crates/mjx-xlsx/docs/guide/deliberate_limitations.md` says the same thing to the caller who arrived
with a spreadsheet rather than with a layering question.

## Who reaches in, and for what

| Reaching crate | Rank | What it takes |
|---|---|---|
| `mjx-chart` | 2.2 | [`mjx_sml::WorkbookPackage`](crate::WorkbookPackage) — the whole embedded-workbook writer behind a chart's **Edit Data** |
| `mjx-vml` | 2.2 | Nothing directly; a comment box's VML is reached *from* `mjx-xlsx`, beside this crate's [`mjx_sml::Comments`](crate::Comments) |
| `mjx-xlsx` | 3.0 | Everything: every part model here, plus the anomaly and identity views |
| `mjx-pptx`, `mjx-docx` | 3.0 | The embedded workbook behind a chart, through `mjx-chart` |

And what this crate reaches *down* for: [`mjx-dml`](https://docs.rs/mjx-dml) for the anchor geometry a
sheet drawing is placed with and for theme colour resolution, `mjx-ooxml-types` for every generated
simple type and for the `xsd:sequence` position of every child of all 367 SpreadsheetML complex
types, `mjx-mce` for markup compatibility, and `mjx-xml`/`mjx-ooxml-core` for the preservation tree
itself.

## Writing a package from nothing

[`mjx_sml::WorkbookPackage`](crate::WorkbookPackage) is a *writer*: a thing you fill and then
serialize once, as against `mjx_xlsx::Workbook`, which is a package you opened. It authors
`[Content_Types].xml`, `_rels/.rels`, `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`,
`xl/worksheets/sheetN.xml`, `xl/sharedStrings.xml` and `xl/styles.xml`, and optionally both
`docProps` parts.

```
# fn main() -> Result<(), mjx_sml::SmlError> {
use mjx_sml::write::{AuthoredCellValue, WorkbookPackage};

let mut workbook = WorkbookPackage::new()?;
workbook.push_row(0, &[
    AuthoredCellValue::Blank,
    AuthoredCellValue::SharedText("Revenue".to_owned()),
])?;
workbook.push_row(0, &[
    AuthoredCellValue::SharedText("Q1".to_owned()),
    AuthoredCellValue::Number(19.2),
])?;
workbook.recompute_dimensions();

let bytes = workbook.to_package_bytes()?;
assert_eq!(&bytes[..2], b"PK", "a workbook is a ZIP package");
# Ok(())
# }
```

`crates/mjx-sml/tests/package_writer.rs` exercises it from a crate whose dependency graph holds no
`mjx-xlsx` at all, which is what says the writer needs nothing above it — the mechanical half being
`xtask/tests/layering.rs`.

## The reading vocabulary, by subject

Every one of these is a part model or a view over one element of a part, and none of them resolves a
relationship.

| Subject | Start at | Notes |
|---|---|---|
| Cells and rows | [`mjx_sml::SheetData`](crate::SheetData), [`mjx_sml::Cell`](crate::Cell), [`mjx_sml::Row`](crate::Row) | [The cell store](the_cell_store) |
| Addresses | [`mjx_sml::CellReference`](crate::CellReference), [`mjx_sml::CellRange`](crate::CellRange), [`mjx_sml::CellRangeList`](crate::CellRangeList) | A1 and R1C1, `sqref`, `spans` |
| Text | [`mjx_sml::SharedStringTable`](crate::SharedStringTable), [`mjx_sml::InlineString`](crate::InlineString) | [Shared strings](shared_strings) |
| Formatting | [`mjx_sml::StylesheetPart`](crate::StylesheetPart), [`mjx_sml::EffectiveCellFormat`](crate::EffectiveCellFormat) | [The stylesheet](the_stylesheet) |
| Formulas | [`mjx_sml::CellFormula`](crate::CellFormula), [`mjx_sml::SharedFormulaGroups`](crate::SharedFormulaGroups), [`mjx_sml::CalculationChain`](crate::CalculationChain) | Text; never evaluated |
| Sheet geometry | [`mjx_sml::SheetDimension`](crate::SheetDimension), [`mjx_sml::ColumnBlock`](crate::ColumnBlock), [`mjx_sml::MergedCells`](crate::MergedCells), [`mjx_sml::SheetViews`](crate::SheetViews) | Reported, never repaired |
| Conditional formatting | [`mjx_sml::ConditionalFormatting`](crate::ConditionalFormatting), [`mjx_sml::ConditionalRuleChain`](crate::ConditionalRuleChain) | `@priority` orders *across* blocks |
| Filters and validation | [`mjx_sml::AutoFilter`](crate::AutoFilter), [`mjx_sml::SortState`](crate::SortState), [`mjx_sml::DataValidations`](crate::DataValidations) | Recorded, never applied |
| Tables | [`mjx_sml::WorksheetTable`](crate::WorksheetTable), [`mjx_sml::TableParts`](crate::TableParts) | The sheet holds `r:id`s; `mjx-xlsx` resolves them |
| Comments | [`mjx_sml::Comments`](crate::Comments), [`mjx_sml::parse_comments`](crate::parse_comments) | The box that draws one is VML, above this crate |
| Print and page setup | [`mjx_sml::PageSetup`](crate::PageSetup), [`mjx_sml::PageMargins`](crate::PageMargins), [`mjx_sml::HeaderFooter`](crate::HeaderFooter) | Reported, never paginated |
| Objects and controls | [`mjx_sml::EmbeddedObjects`](crate::EmbeddedObjects), [`mjx_sml::FormControls`](crate::FormControls), [`mjx_sml::ObjectAnchor`](crate::ObjectAnchor) | `r:id`s and anchors |
| What is preserved, not modelled | [`mjx_sml::PivotTableIdentity`](crate::PivotTableIdentity) and its eight siblings | [Fidelity and the known gaps](fidelity_and_gaps) |

## What is *not* here, and where it is

* **Opening or saving a package.** `mjx-opc` models the container;
  `crates/mjx-opc/docs/guide/the_package.md` is its guide.
* **Resolving an `r:id`.** This crate holds the identifier as the string the file wrote and answers
  which prefix that file bound the relationship namespace to; turning it into a part is `mjx-xlsx`'s,
  in `crates/mjx-xlsx/src/worksheet/`.
* **A drawing's contents.** `x:drawing` is one `r:id`; the SpreadsheetDrawingML in the part it names
  is `mjx_dml::spreadsheet_drawing`'s and the chart inside that is `mjx-chart`'s.
* **The VML behind a comment box or a form control.** `mjx-vml`, reached from `mjx-xlsx`.
* **The `Workbook` surface a caller holds** — `mjx-xlsx`, and through it `mjx-ooxml` and both
  bindings. `crates/mjx-xlsx/docs/guide/through_the_facade.md` is the same workbook seen from there.
