# The embedded workbook

**Read this before calling anything that changes a chart's data.** A chart in a real file does not
only carry cached numbers: it embeds a whole spreadsheet package at `/ppt/embeddings/*.xlsx`, related
from the chart part by `.../relationships/package` and named by the chart's `c:externalData@r:id`.
That workbook is a part **the producer wrote**, and it is what Edit Data opens.

## What used to happen, and why it was wrong even though it was documented

Until MJXOFF-208, a data edit threw the producer's workbook away. `set_chart_series_values` and
`set_chart_series_categories` called `refresh_chart_workbook` for you; that built a *fresh* one-sheet
package with [`embedded_workbook_for_chart_space`](crate::embedded_workbook_for_chart_space) and wrote
it over the part. Every extra sheet, cell format, defined name, macro and document property the
workbook carried was gone — from a call that only said *set this series to these numbers*.

It was disclosed: the doc comment said the workbook was *regenerated, not patched* and named what was
lost. The disclosure was honest and the **default** was inverted. Under this project's standing rule —
*supply a default only in the absence of the user's own, never in place of it* — the preserving branch
is what has to happen when the caller says nothing.

The stated justification was also wrong, not merely weak. Reconciling a third-party workbook with
edited chart data was called *"a merge problem with no correct answer"*. It is not a merge problem:
the chart already states where its data lives — the `c:f` beside each cache — so putting the new
numbers there is an address lookup.

## Patch, or refuse. Never guess, and never regenerate behind the caller's back

`crates/mjx-chart/src/embedding/patch.rs` reads the `c:f` beside each cache, resolves it to cells, and
writes the chart's numbers into **those cells**. Everything else in the package comes back byte for
byte because it is never touched: [`mjx_opc::Package`](mjx_opc::Package) re-emits an unedited part
from the buffer it was read from, and [`mjx_sml::WorksheetPart`](mjx_sml::WorksheetPart) rewrites only
the row a cell lands in.

The tempting third option — *look at the workbook, decide whether it seems producer-written, and
regenerate when it does not* — is a heuristic, and a heuristic is wrong on somebody's file silently
and in the destructive direction. **There is no such branch.** Either the reference resolves to cells
this library can write, or the patch refuses with
[`ChartAccessError::EmbeddedWorkbookNotWritable`](crate::ChartAccessError::EmbeddedWorkbookNotWritable),
naming the `c:f` and saying what about it could not be resolved.

## The eight refusals, and why each one is a property of the producer's own text

[`ReferenceProblem`](crate::ReferenceProblem) is the whole list. Every entry is a **shape of
reference**, decided from the text the producer wrote — never a guess about provenance:

| Variant | The `c:f` looks like | Why it is refused |
|---|---|---|
| [`NotACellReference`](crate::ReferenceProblem::NotACellReference) | `SUM(Sheet1!$B:$B)`, a defined name | resolving it needs a formula evaluator, a permanent non-goal of this project |
| [`NoSheetNamed`](crate::ReferenceProblem::NoSheetNamed) | `$B$2:$B$5` | it does not say which of the workbook's sheets it means |
| [`AnotherWorkbook`](crate::ReferenceProblem::AnotherWorkbook) | `[1]Sheet1!$A$1` | that book is not this package's to edit, and this library never opens one |
| [`SeveralSheets`](crate::ReferenceProblem::SeveralSheets) | `Sheet1:Sheet3!$A$1` | a point in the cache then does not name one cell |
| [`WholeColumnsOrRows`](crate::ReferenceProblem::WholeColumnsOrRows) | `Sheet1!$B:$B` | which cells of a whole column the points occupy is Excel's decision about populated data, not an address; writing at a guessed offset would put numbers in cells the chart never named |
| [`Rectangular`](crate::ReferenceProblem::Rectangular) | `Sheet1!$A$1:$C$9` | more than one column wide *and* more than one row tall, so the order points map onto cells is not stated. A `c:multiLvlStrRef` reaches here deliberately: its level-to-column order is a convention, not something the reference says |
| [`NoSuchSheet`](crate::ReferenceProblem::NoSuchSheet) | `Deleted!$B$2` | the sheet is gone, or the name reaches a chart sheet or a dialog sheet, which have no cells |
| [`FewerCellsThanPoints`](crate::ReferenceProblem::FewerCellsThanPoints) | `Sheet1!$B$2:$B$3` for four points | writing them all would put values outside the range the chart itself says its data lives in. Growing the range would mean rewriting the producer's own `c:f`, which is the class of act this module exists to stop |

**Eight, not nine.** MJXOFF-221's own ticket says the patch "refuses nine reference shapes by name",
and `CHANGELOG.md`'s MJXOFF-208 entry lists six of them in prose. Neither is the count: the
enumeration has eight variants, and a count that is not read off the declaration is a count that
expires. This table is the declaration, row for row.

A caller who genuinely would rather have a fresh workbook than the producer's one says so, with the
`regenerate_chart_workbook` method each host crate exposes. That is the old behaviour under a name
that says what it does, and it is now the thing a caller has to ask for rather than the thing that
happens to them.

## Two functions, because the interner and the package cannot be borrowed at once

Planning reads the chart, which lives behind the host package's part tree; applying reads the embedded
workbook, which lives in the same package. A host cannot hold both borrows, so
[`plan_workbook_patch`](crate::plan_workbook_patch) finishes with the part tree and
[`apply_workbook_patch`](crate::apply_workbook_patch) starts with the bytes.

That split is also what gives a data edit its **all-or-nothing** shape: the half that can refuse runs
before the chart part is touched, and the half that writes runs after. A refusal therefore changes
neither part.

[`WorkbookPatch`](crate::WorkbookPatch) names what to write, and its three variants are its three
callers. `EveryReference` is what *refresh* means — make the workbook say what the chart draws.
`SeriesValues` and `SeriesCategories` name exactly one series and one kind of data, because a second
series whose `c:f` this library cannot resolve must not make an unrelated edit fail.

## A cell that already says the right thing is not written

[`apply_workbook_patch`](crate::apply_workbook_patch) compares before it writes and answers
`Ok(None)` — *leave the part alone* — when every planned cell already holds its value. That is not the
same answer as *there is no workbook*, and the host crates keep the two apart: a refresh over a
workbook that already agrees still reports that it found one. Two things turn on it:

* **A refresh over a file that already agrees writes nothing at all.** Re-saving a package rewrites
  its ZIP container even when every part inside is identical, so the host's byte-identity guarantee
  for that part survives only if the write is skipped entirely.
* **A shared string stays shared.** New text is written as an inline string (`t="inlineStr"`), because
  interning it would mean editing `xl/sharedStrings.xml` too — a second part of somebody else's file
  the caller never named. Comparing first means only the labels that actually changed convert.

## A point is written at its `c:idx`, not at its position in the file

A sparse cache — a series with a blank third value writes points `0`, `1`, `3` — would otherwise slide
every later value one cell up somebody else's column. The old regenerator had the same flaw in its own
grid.

## Reaching the workbook part at all

[`embedded_workbook_part`](crate::embedded_workbook_part) is the *chart part → relationship id →
workbook part* walk, which `mjx-pptx`, `mjx-docx` and `mjx-xlsx` each used to carry their own copy of.
It is the one place in these three crates that names [`mjx_opc::Package`](mjx_opc::Package) — see the
guide index's last section.
