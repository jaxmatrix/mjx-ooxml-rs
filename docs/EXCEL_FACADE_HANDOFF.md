# The Excel facade and its two bindings — validation checklist for MJXOFF-128 (F2)

**Written by MJXOFF-137 (D20). Handed over UNMARKED.**

Every row below is a claim this workspace makes today about a file it wrote, and **not one of them
has been opened in real Microsoft Excel**. The **Excel says** and **Verdict** columns are
deliberately empty and must stay empty until somebody does that. No agent can: no agent has Excel.

> **Do not mark a row "unverified" as though that were a completed state, and never mark one pass.**
> An empty cell is the honest record of work that has not happened. A column of "unverified" is a
> completed-looking table that says nothing.

## What produces the artefacts

Three walkthroughs write the **same workbook**, and the two bindings are already compared against the
Rust one part by part, byte for byte — so a person only has to open **one** file to validate all
three:

```sh
cargo run -p mjx-ooxml --example build_a_workbook -- facade_build_a_workbook.xlsx
```

The other two, for completeness (they are byte-identical to the above by test, not by assumption):

```sh
cd bindings/mjx-python && pytest tests/test_build_a_workbook.py   # writes python_build_a_workbook.xlsx
node --test bindings/mjx-wasm/tests/node/build_a_workbook.mjs      # writes wasm_build_a_workbook.xlsx
```

All three land in `target/examples/` unless `MJX_OUTPUT_DIR` says otherwise.

LibreOffice Calc opens it in CI (`office-open`), which is a real consumer and a real gate — and it is
*not* Excel. Everything below is what Calc opening a file cannot tell anybody.

## A · The authored workbook, opened in Excel

| id | What this build wrote | What a person must check | Excel says | Verdict |
|---|---|---|---|---|
| D20-A1 | Two tabs, `Summary` then `Notes` | Both tabs appear, in that order, spelled exactly | | |
| D20-A2 | `A1:C1` = `Region`, `Revenue`, `Growth` as **shared strings** | The three headings read correctly, and Excel does not offer to repair | | |
| D20-A3 | `B2:B4` = `1250000`, `980000`, `1410000`, written with no `t` attribute | Each shows as a plain number, right-aligned, not as text | | |
| D20-A4 | `C2:C4` = `0.125`, `0.061`, `0.198` | Each shows as a number, **not** as a percentage — no number format was applied | | |
| D20-A5 | `B5` = boolean `false` (`t="b"`, `<v>0</v>`) | Excel shows `FALSE`, not `0` and not `#VALUE!` | | |
| D20-A6 | `Notes!A1` written as an **inline string** (`t="inlineStr"`) rather than a shared one | The text appears; Excel does not silently move it into the shared table on first save | | |
| D20-A7 | Heading style: Calibri 12 bold, white text, solid `1F3864` fill, medium bottom border | All four aspects render on `A1:C1`, and only on those three cells | | |
| D20-A8 | Row 1 height `22` points with `customHeight="1"` | The row is visibly taller, and Excel does **not** auto-fit it away | | |
| D20-A9 | Column A width `22`, columns B–C width `14`, all `customWidth="1"` | The three widths are honoured and differ from each other | | |
| D20-A10 | `A2` carries an external hyperlink to `https://example.org/north-america` | The cell is clickable; the target is exactly that URL; the cell's own text is unchanged | | |
| D20-A11 | The whole file, after Excel opens and re-saves it | Excel's own save does not report or repair anything, and the tabs/values/styles above survive | | |

## B · The committed fixtures, read through the facade

Each row is something the facade **reports** about a file this project did not author. The check is
whether Excel agrees about the same file.

| id | Fixture | The facade's report | What a person must check | Excel says | Verdict |
|---|---|---|---|---|---|
| D20-B1 | `tests/fixtures/sample.xlsx` | `used_range(0)` | Excel's own used range (Ctrl+End) is the same rectangle | | |
| D20-B2 | `conditional_formatting.xlsx` | `conditional_formatting_ranges(0)` and `conditional_formatting_rule_count(0, 0)` | Excel's Conditional Formatting Rules Manager lists the same regions, and the same number of rules in the first block | | |
| D20-B3 | `validation_and_filters.xlsx` | `auto_filter_range(0)` | Excel's autofilter covers exactly that range | | |
| D20-B4 | `validation_and_filters.xlsx` | `data_validation_ranges(0)` | Excel's Data Validation dialog claims the same cells | | |
| D20-B5 | `worksheet_tables.xlsx` | `sheet_tables(0)` — display name, range, header/totals counts, column names | Excel's Table Design tab shows the same name, the same range and the same columns in the same order | | |
| D20-B6 | `hyperlinks.xlsx` | `sheet_hyperlinks(0)` — range, target, tooltip, display | Each link points where Excel says it points | | |
| D20-B7 | `preserved_parts.xlsx` | `preserved_parts()` and `pivot_tables()` | Excel opens the file with its pivot tables intact **after a facade round trip** (open, save, reopen) | | |
| D20-B8 | `effective_cell_format.xlsx` | `effective_cell_format(0, …)` | See `docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md` — that table is the detailed one; this row only records that the facade answers the same as the layer below it | | |
| D20-B9 | `sheet_grid.xlsx` | `merged_ranges(0)` and `grid_anomalies(0)` | Excel merges the same regions; and where the facade reports an anomaly, Excel either shows the same oddity or repairs it (**either answer is information** — record which) | | |

## C · The claims only a person can falsify

These have no fixture and no assertion. They are the statements this project makes in prose.

| id | The claim | What a person must check | Excel says | Verdict |
|---|---|---|---|---|
| D20-C1 | A workbook opened, untouched and saved through the facade is byte-identical part by part | Excel opens the round-tripped `sample.xlsx` with no repair dialogue, and nothing visibly differs | | |
| D20-C2 | `write_cells` leaves every row it did not name byte-identical | Write one cell into a large real workbook through the facade, reopen in Excel, and confirm nothing else changed — including things this library does not model | | |
| D20-C3 | `rename_sheet` does **not** rewrite formulas that reference the old name | Rename a tab a formula references, open in Excel, and record what Excel does with the now-dangling reference. **This is a known limitation, not a bug to fix here** — the record is what it costs a user | | |
| D20-C4 | `.xlsb` is refused by design | Confirm a real `.xlsb` is refused with the MS-XLSB message and not with a parse failure | | |
| D20-C5 | `CellInput::SharedText` and `CellInput::InlineText` are both legitimate | Confirm Excel treats an authored `inlineStr` cell as ordinary text — it is legal markup that Excel itself rarely writes | | |
| D20-C6 | A number written by `CellInput::Number` uses Rust's shortest round-tripping spelling | Confirm Excel reads `0.061` and `1250000` back as exactly those values, not as approximations | | |

## D · What has no runtime coverage anywhere

Written down because a checklist that only lists what *is* covered is the failure mode this project
keeps finding.

* **`maturin`, `pytest` and `mypy` are not installed on the machine MJXOFF-137 ran on**, so
  `test_build_a_workbook.py`, `test_workbook_surface_coverage.py` and the stub-parity check for the
  Excel classes have **never executed locally**. CI's three `python-wheel` jobs are the first thing
  that runs them. MJXOFF-139 hit exactly this and CI found a real failure.
* **`soffice` is not installed either**, so the `office-open` job is the first thing that opens the
  authored workbook in any consumer at all.
* **No file in `tests/fixtures/` was authored by Microsoft Excel.** Every `.xlsx` there was written by
  this project or by a script. That is MJXOFF-130's (F3) whole subject, and every row above is
  weaker for it: agreement with a file we wrote is not agreement with Excel.
* **`mjx-schema-gate` cannot validate an Office-authored worksheet carrying `mc:Ignorable`** — MCE
  resolution empties `<ext>`, whose `xsd:any` is `minOccurs="1"`. The first real Excel file added to
  the corpus will trip this and it will present as a fixture problem rather than a gate problem. It
  is a prerequisite for MJXOFF-130 and it is unchanged by this child.
