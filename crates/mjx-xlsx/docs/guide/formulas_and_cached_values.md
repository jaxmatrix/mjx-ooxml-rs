# Formulas and cached values

**This library carries formulas as text and never calculates.** There is no expression parser, no
dependency graph and no calculation engine, and none is planned — `PLAN.md` settles that as scope
rather than as an omission. What follows is what that means when you edit a workbook, stated plainly
enough that nobody plans around a behaviour this crate does not have.

## The one thing to know before you edit anything

Change a cell that a formula depends on, and **the formula's cached value is left exactly as it
was**. It is now out of date, and that is deliberate:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

let reference = |text: &str| CellReference::parse(text).expect("a reference");
let mut workbook = Workbook::open(&mjx_fixtures::fixture("formulas.xlsx"))?;

// B2 holds `=A2*2`, and A2 holds 1, so Excel last cached 2 in B2.
assert_eq!(workbook.cell_text(0, reference("B2"))?.as_deref(), Some("2"));

// Change what B2 depends on.
workbook.set_cell_value(0, reference("A2"), CellValue::Number(50.0))?;

// B2's cached value is still 2. Nothing here recalculated it, and nothing blanked it.
assert_eq!(workbook.cell_text(0, reference("A2"))?.as_deref(), Some("50"));
assert_eq!(workbook.cell_text(0, reference("B2"))?.as_deref(), Some("2"));
# Ok(())
# }
```

The alternative — blanking `<v>` whenever a dependency might have changed — sounds helpful and is
the most damaging thing a fidelity library can do. It destroys data in a file the caller opened to
change a label, in cells they never named, and it cannot be undone from the saved file. Excel
recalculates on open when it needs to; this crate's job is fidelity, not arithmetic.

If you need a workbook to recalculate the moment it opens, say so in the file rather than asking this
library to compute: `x:calcPr/@fullCalcOnLoad` is the switch Excel itself uses, and
`mjx_sml::CalculationProperties` carries it.

## Reading a formula

[`mjx_sml::CellFormula`] is a borrowed view over the `<f>` element's own bytes — nothing is copied,
and nothing it reports can drift from what will be written back.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellFormula, CellReference, FormulaKind};
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("formulas.xlsx"))?;
let sheet = workbook.worksheet_markup(0)?.expect("tab 0 is a worksheet");

let c2 = sheet.cell(CellReference::parse("C2")?).expect("C2 is populated");
let formula = c2.formula().expect("C2 carries a formula");

assert_eq!(formula.text()?, "SUM(A2:A6)");
assert_eq!(formula.kind()?, FormulaKind::Normal);
// The cached result, read through the `c@t` beside it.
assert_eq!(c2.cached_value().expect("a cache").as_number(), Some(15.0));

// `t` is declared `default="normal"`, so C2 writing nothing and C3 writing `t="normal"` mean the
// same thing — and must not come back written the same way.
assert!(!formula.has_written_kind());
let c3 = sheet.cell(CellReference::parse("C3")?).expect("C3 is populated");
assert!(c3.formula().expect("a formula").has_written_kind());
# let _ = CellFormula::parse(b"<f>A1</f>");
# Ok(())
# }
```

A cell's `<v>` is a **cache** only when there is an `<f>` beside it.
[`mjx_sml::Cell::cached_value`] answers `None` for a plain value, because a number somebody typed and
a result Excel computed are different facts about the file.

## Shared formulas: the text lives on one cell

A shared group is written once. The **host** carries the expression, an `@si` naming the group and a
`@ref` naming the range; every other member carries the same `@si` and **no text at all**.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("formulas.xlsx"))?;
let sheet = workbook.worksheet_markup(0)?.expect("tab 0 is a worksheet");
let data = sheet.sheet_data().expect("the sheet has cells");

let groups = data.shared_formula_groups()?;
let group = groups.get(0).expect("group 0");
assert_eq!(group.cell_count(), 5);
assert_eq!(group.host(), Some(CellReference::parse("B2")?));
assert_eq!(group.host_formula().expect("the host").raw_text(), b"A2*2");

// A member carries the index and nothing else.
let b4 = sheet.cell(CellReference::parse("B4")?).expect("B4 is populated");
let member = b4.formula().expect("B4 carries an f element");
assert!(!member.has_text());
assert_eq!(member.shared_group_index()?, Some(0));
# Ok(())
# }
```

**That distribution is preserved exactly.** Writing the host's text into the members on the way out
would change bytes in a part nobody asked to edit, inflate a sheet whose whole reason for sharing was
size, and — because a shared formula's references are written relative to the *host* — state a
different formula from the one that cell actually has.

For the same reason there is no `member.text()`: a member's own expression is the host's text shifted
by the offset between the two cells, and shifting references is translation, which this crate does
not do. [`mjx_sml::SharedFormulaGroup::host`] gives you the host's address, which is everything a
caller needs to do the shift itself.

## Array and data-table formulas

An **array formula** carries `t="array"` and a `@ref` naming the range it spills over. Only the
top-left cell carries the `<f>`; the other cells of the range carry a `<v>` and no formula, which is
worth knowing before you write a loop that expects one formula per covered cell.

A **data-table formula** carries `t="dataTable"`, the input cells `@r1` and `@r2`, and the flags
`@dt2D`, `@dtr`, `@del1` and `@del2`. It has no expression text — the formula is the implied
`TABLE()`. Rare, and it survives here exactly as written.

## `calcChain.xml`: left alone

`xl/calcChain.xml` records the order a producer last calculated in. ECMA-376 §18.6 is explicit that a
consumer *"is free to perform calculations in a different order at run time"* and that the part is
optional.

This library **leaves it exactly as it found it**. Nothing is added when you author a formula,
nothing is removed when you delete a cell, and the part is never dropped on save.
[`Workbook::calculation_chain`] reads it; there is deliberately no writer.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let mut workbook = Workbook::open(&mjx_fixtures::fixture("formulas.xlsx"))?;
let entries = workbook
    .calculation_chain(|chain, _| chain.len())?
    .expect("this workbook has a calculation chain");
assert_eq!(entries, 11);

// A workbook without one answers `None` rather than failing.
let mut plain = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
assert!(plain.calculation_chain(|_, _| ())?.is_none());
# Ok(())
# }
```

Maintaining the chain would mean computing a dependency order, which means parsing formula
expressions — a calculation engine under another name. Dropping the part would be an edit to the
package you did not ask for, made on every save, and it would lose a record you may be reading the
file precisely to inspect. So a workbook whose formulas you edited here carries a chain that is out
of date, and a consumer rebuilds it: the same thing it does when the part is absent.

## The limits, in one list

| What | Status |
|---|---|
| Formula text | Preserved byte for byte, never reformatted or re-derived |
| Cached `<v>` | Preserved byte for byte; **goes stale after an edit, deliberately** |
| `A1` ↔ `R1C1` translation | Not done. `x:calcPr/@refMode` says which syntax the file uses; the text is carried in it |
| Shared-group expansion | Never. The host keeps the text and the members keep none |
| `calcChain.xml` | Read; never maintained, never dropped |
| Evaluation of any kind | Not done, and not planned |

The same boundary holds one page over. A conditional-formatting rule's condition is a formula too, so
[Conditional formatting](conditional_formatting) reports **which rules apply to a cell** and never
whether any of them is true.
