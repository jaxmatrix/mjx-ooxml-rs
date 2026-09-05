# Effective cell formatting — comparison table for MJXOFF-122 (F1)

**Written by MJXOFF-108 (D09). Handed over UNMARKED.**

Every row below is an answer this workspace gives today, produced by
`mjx_sml::styles::effective::CellFormatResolver` against
`tests/fixtures/effective_cell_format.xlsx`. The **Excel says** and **Verdict** columns are
deliberately empty and must stay empty until somebody opens the fixture in real Microsoft Excel and
fills them in. No agent can do that.

> **Do not mark a row "unverified" as though that were a completed state, and never mark one pass.**
> An empty cell is the honest record of work that has not happened. A column of "unverified" is a
> completed-looking table that says nothing, which is the failure mode this child's own ticket was
> rewritten to avoid.

## How to reproduce a row

```rust
let bytes = std::fs::read("tests/fixtures/effective_cell_format.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;
let formatting = workbook.sheet_formatting(0)?.expect("sheet 0 has formatting");
let resolver = formatting.resolver()?;
let format = resolver.effective_cell_format(mjx_sml::CellReference::parse("A1")?)?;

format.font();                                  // which layer, which flag, which fontId
resolver.font(&format);                         // the <font> element itself
resolver.format_code(&format)?;                 // the number format code in force
```

Every row is also asserted mechanically in `crates/mjx-sml/tests/effective_cell_format.rs`, so a
change in behaviour fails a test before it reaches this table.

## The fixture in one paragraph

`cellXfs[1]` and `cellStyleXfs[1]` state a **different value for every one of the six aspects**, so
reading the wrong layer gives a visibly wrong answer. `cellXfs` records 1–4 name the same `xfId="1"`
and the same four resource indices and differ **only** in their `applyX` attributes — false, absent,
true, and mixed — which is the only arrangement that can tell the three states apart. Record 5 sits
on a `cellStyleXfs` record that suppresses `applyFont` itself. Columns B–D carry `col@style="7"`;
row 2 writes `customFormat="1" s="6"`; row 3 writes `s="6"` with **no** `customFormat`.

| | direct — `cellXfs[1]` | beneath — `cellStyleXfs[1]` |
|---|---|---|
| number format | `numFmtId="164"` (custom USD code) | `numFmtId="165"` (`0.000%`) |
| font | `fontId="1"` — DirectFont, bold, 12 pt | `fontId="2"` — StyleFont, italic, 13 pt |
| fill | `fillId="2"` — solid `FF112233` | `fillId="3"` — solid `FF445566` |
| border | `borderId="1"` — thin **left** edge | `borderId="2"` — thick **right** edge |
| alignment | `left` / `bottom` / `wrapText="0"` | `right` / `top` / `wrapText="1"` |
| protection | `locked="1" hidden="0"` | `locked="0" hidden="1"` |

## The table

`Basis` says where the answer comes from: **normative** is a sentence of ECMA-376 Part 1;
**reading** is an inference from a worked example, and is the part of this that most needs Excel.

| # | Cell (`cellXfs`) | Question | mjx-ooxml-rs answers | Basis | Excel says | Verdict |
|---|---|---|---|---|---|---|
| 1 | `A1` (1) | number format | `0.000%` — from `cellStyleXfs[1]` | normative §18.8.10 | | |
| 2 | `A1` (1) | font | `StyleFont`, italic, 13 pt — from `cellStyleXfs[1]` | normative §18.8.10 | | |
| 3 | `A1` (1) | fill | solid `FF445566` — from `cellStyleXfs[1]` | normative §18.8.10 | | |
| 4 | `A1` (1) | border | thick **right** edge — from `cellStyleXfs[1]` | normative §18.8.10 | | |
| 5 | `A1` (1) | alignment | `horizontal="right"`, `vertical="top"`, `wrapText` true | normative §18.8.10 | | |
| 6 | `A1` (1) | protection | `locked` false, `hidden` true | normative §18.8.10 | | |
| 7 | `B1` (2) | font, with `applyFont` **absent** | `DirectFont` — the **direct** layer | **reading** §18.8.9 | | |
| 8 | `B1` (2) | fill, with `applyFill` **absent** | solid `FF112233` — the **direct** layer | **reading** §18.8.9 | | |
| 9 | `B1` (2) | number format, `applyNumberFormat` absent | the custom code (id 164) | **reading** §18.8.9 | | |
| 10 | `B1` (2) | alignment, `applyAlignment` absent | `left` / `bottom` / no wrap | **reading** §18.8.9 | | |
| 11 | `C1` (3) | font, with `applyFont="1"` | `DirectFont` | normative §18.8.45 | | |
| 12 | `C1` (3) | number format, `applyNumberFormat` absent | the custom code (id 164) | **reading** §18.8.9 | | |
| 13 | `D1` (4) | font (`applyFont="0"`) **and** fill (`applyFill="1"`) on one record | font `StyleFont`, fill `FF112233` — different layers | normative §18.8.45 | | |
| 14 | `E1` (5) | font, suppressed on **both** layers | nothing supplies it (`FormatLayer::Neither`) | **reading** — no sentence covers it | | |
| 15 | `E1` (5) | fill, which record 5 does not suppress | `fillId="0"` — the `none` pattern fill, from the **direct** layer | normative §18.8.45 | | |
| 16 | `C2` (3) | which layer states the style index | the **cell**'s `@s` → `DirectFont` | normative §18.3.1.4 | | |
| 17 | `B2` (6) | cell has no `@s`; row has `customFormat="1" s="6"`; column has `style="7"` | the **row** → `RowFont` | normative §18.3.1.73 | | |
| 18 | `B3` (7) | cell has no `@s`; row has `s="6"` and **no** `customFormat`; column has `style="7"` | the **column** → `ColumnFont` | normative §18.3.1.73 | | |
| 19 | `F4` (0) | cell, row and column all silent | the default record `cellXfs[0]` → `Calibri` | normative | | |
| 20 | — | `numFmtId="164"`'s format code | `[$-409]#,##0.00"  USD"\;;[Red]-#,##0.00` — character for character, two spaces and all | normative §18.8.30 | | |
| 21 | — | `numFmtId="0"`, declared nowhere in the file | `General` — implied by §18.8.30 | normative §18.8.30 | | |
| 22 | — | `numFmtId="37"` | `#,##0 ;(#,##0)` — **with** the space before the semicolon | normative §18.8.30 (transcribed) | | |
| 23 | — | `numFmtId="39"` | `#,##0.00;(#,##0.00)` — **without** a space | normative §18.8.30 (transcribed) | | |
| 24 | — | `numFmtId="5"` | not built in; the file must declare it | normative §18.8.30 | | |
| 25 | — | `numFmtId="30"`, `zh-tw` / `zh-cn` / `ja-jp` / `ko-kr` | `m/d/yy` / `m-d-yy` / `m/d/yy` / `mm-dd-yy` | normative §18.8.30 | | |
| 26 | — | `numFmtId="30"` with no UI language known | no answer (`None`) | this crate's choice | | |
| 27 | `A1` (1) | the named style beneath the cell | `Explanatory Text`, `builtinId="53"` | normative §18.8.7 | | |
| 28 | — | `builtinId="1"` | `RowLevel_` + the style's `@iLevel` | normative Annex G.2 | | |

## The two answers that most need a real Excel

Both are marked **reading** above. Neither is guessed, and neither is stated outright by the
specification either.

1. **Absent `applyX` behaves as applied** (rows 7–12). §18.8.45 gives each flag one sentence and says
   nothing about absence; the schema declares all six `use="optional"` with **no `default=`**, so the
   three states are real. The reading comes from §18.8.9's worked example: *"the 0th record does not
   express any 'apply' attributes, while the other records do express 'apply' attribute values"* —
   and the record expressing none is `Normal`, which is applied.
   **What to check in Excel:** open the fixture and compare `A1` (all flags `"0"`) with `B1` (all
   flags absent). If they render the same, this reading is wrong.
2. **A `cellStyleXfs` record's own `applyX` is honoured** (row 14). §18.8.9 says master formatting
   records *"also specify whether to apply or ignore particular aspects of formatting"*, so honouring
   them is faithful — but no sentence says what happens when **both** layers suppress an aspect.
   This crate answers `FormatLayer::Neither` and reports the font as unresolved rather than falling
   back to font 0.
   **What to check in Excel:** what font does `E1` render in? If it is Calibri (font 0), the fallback
   belongs in the resolver and `Neither` should become `Default`.

## Out of scope here and forever

Applying a format code to a value — rendering `3.14159` through `0.00` — is a programme non-goal.
The resolver reports the code in force and stops. Nothing in this table is about display strings,
column widths or measured text.

## Not yet layered in

A conditionally formatted cell has a `dxf` applied **on top of** everything in this table
(§18.8.15). That layer is MJXOFF-120's (D13). `mjx_sml::DifferentialFormat` is already built and
`EffectiveCellFormat` is already the value it deltas, so the seam is in place and nothing here has to
change for it to arrive — but until D13 lands, no row above accounts for conditional formatting.
