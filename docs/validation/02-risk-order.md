# The order to work through, and why

The checklist is three pages of areas in numeric order, because that is how a reader finds a check
again. **This page is the order to actually do them in.** The two are different on purpose: numeric
order is for navigation, risk order is for the afternoon.

The rule the ordering follows is the one `docs/validation/00-method.md` states: risk is **how much
of the answer is a judgement Office could disagree with**, not how important the feature is. A
`low` area is not unimportant; it is one where two conformant implementations could hardly differ.

> **Stop at the first `differs`.** Everything below R1 was written assuming R1 resolves; if the
> tier-5 answer is wrong, the text ladder underneath it is being validated against a resolver that
> is about to change.

## The seven, in order

These are `MJXOFF-63`'s risk list, carried forward unchanged. **R1 is still the single
highest-risk item in the repository.**

### R1 — the 0.0.58 tier-5 change

A non-placeholder shape now takes the master's `p:otherStyle` / `p:bodyStyle` per ECMA-376
§19.3.1.35. This follows the spec, it changes effective text on essentially every real deck,
**real PowerPoint is believed to match the *previous* behaviour**, and it is isolated in one
revertible commit. Gaps-page owners: `MJX-211` R1, `MJX-208`. Shipped by `MJX-22` at 0.0.58.

* **`V-PPTX-01.1`** — the check itself, and a design question rather than an expectation.
* **`V-PPTX-01.6`**, **`V-PPTX-01.7`** — the rest of the ladder, which only means anything once R1 is settled.

### R2 — every fixture is hand-crafted

No test in this repository has ever read a file Microsoft Office wrote. LibreOffice confirms files
*open*; nothing yet confirms they *render as intended*. `MJXOFF-130` built the road — the corpus slot
at `tests/office-authored/`, the ingest command, and `xtask/tests/office_corpus.rs`, which holds
whatever lands there to byte identity at the container *and* through the facade, to the fidelity
tree, to the package invariants and to the child-order audit. **It cannot supply the traffic.** Every
artefact this pass re-saves from Office is a candidate for that corpus, and
`docs/validation/06-the-office-pass.md` §5 is the loop that puts one in.

* **`V-PPTX-04.4`** — re-save each chart from Office and read it back. This is the one that produces the corpus's first files.
* Every entry marked **blocked** on this page's sibling pages names `MJXOFF-130` because of R2:
  `V-PPTX-01.8`, `V-PPTX-02.13`, `V-PPTX-02.17`, `V-PPTX-07.6`, `V-PPTX-08.12`. (`V-PPTX-02.4` was on
  that list and is not any more: `MJXOFF-219` gave it an authored artefact, which is a different
  answer from an Office-authored one and enough to make the entry readable.)

### R3 — the colour transforms

`comp` / `gray` / `gamma` / `invGamma` are documented-interpretation, not Office-exact: implemented
from the ECMA-376 prose and unit-tested against it, never against a renderer. Gaps-page owner:
`MJX-211` R3.

* **`V-PPTX-02.4`** — compare `effective_shape_fill`'s RGB against PowerPoint's eyedropper. **No longer blocked** (`MJXOFF-219`): `ColorSpec` now carries the whole of `EG_ColorTransform`, and `v-pptx-02-authored.pptx` opens with two rows of swatches — the four transforms named above plus `a:inv` over a fixed `4472C4`, and `tint`/`shade`/`satMod`/`lumMod`+`lumOff`/`alpha` over the theme's accent 1, each row led by an untransformed baseline. It was the only entry in this pass with no artefact at all, and the reason was that nothing in the workspace could author a colour transform; no committed fixture has one of the four either, and none ever will unless Office writes it.

### R4 — chart workbook staleness

`MJXOFF-57` closed it for anything this library writes; **`MJXOFF-99` moved the writer to
`mjx-sml`** — not to `mjx-xlsx`, which would have been an illegal upward edge, and `MJXOFF-132` and
`MJXOFF-112` resolved that; `MJXOFF-103` extended it to Word and `MJXOFF-111` to Excel. **Confirm all
four.**

* **`V-PPTX-04.1`** — *Edit Data* on an authored deck.
* **`V-PPTX-04.3`** — the same question asked of all three hosts at once.
* **`V-DOCX-04.2`** — Word's host.
* **`V-XLSX-04.1`** — Excel's, where the chart reads a **live range** instead.
* **`V-PPTX-08.10`** — the detached case, which is the escape hatch the non-goal depends on.
* **`V-XLSX-04.2`** — a cache that has gone stale against its own cells.

### R5 — the effective readers, as a class

Each walks a multi-tier ladder, and each is a place where a reading of the prose rather than a schema
decides the answer. Word's ladder (`MJXOFF-106`) and Excel's `xf` indirection (`MJXOFF-108`) join
PowerPoint's. **`MJXOFF-108` handed forward an *unmarked* comparison table specifically for this
pass — its Excel column is the deliverable of the human run, not of any agent.**

* **`V-XLSX-02.1`** — `docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md`, 28 rows, two empty columns.
* **`V-DOCX-01.3`** — the toggle-property XOR rule, which the Word gaps page calls the single most surprising answer this library gives.
* **`V-DOCX-01.2`** — the six-tier ladder order.
* **`V-PPTX-01.1`**, **`V-PPTX-01.6`** — PowerPoint's, which is R1.
* **`V-XLSX-02.2`**, **`V-XLSX-02.3`**, **`V-XLSX-02.4`** — the `xf` indirection in a file we authored, the number-format code, and a merged range.
* **`V-DOCX-03.3`** — Word's cell readers, resolved through a table style.

### R6 — table styles

Built-in style ids and `tableStyles.xml` interact in ways LibreOffice does not reproduce, so the
canary is worth **nothing** here.

* **`V-PPTX-03.2`** — an authored table style, in the gallery under its own name.
* **`V-PPTX-03.1`**, **`V-PPTX-03.3`**, **`V-PPTX-03.4`** — the merge, the cell formats, and what a selection does across a merge.
* **`V-DOCX-03.3`** — Word's conditional table formatting.

### R7 — `effective_shape_bounds == None` for a rotation-only transform

A **documented non-goal** with a stated reason: *a transform is inherited whole … a partial transform
therefore places nothing.* The entry asks whether **the documentation is right** — a different
question from whether the code matches it, and the one place in this pass where interrogating a
non-goal is the point.

* **`V-PPTX-07.6`**. **Blocked**: `set_shape_transform` writes only the fields its argument names — an unset field means *leave it alone*, never *clear it* — so no facade call can author the shape, and no committed fixture carries one.

## Word's five, in order

From `MJXOFF-74`'s specification. Do these after R1–R7, and in this order.

| # | Area | Shipped by | Checks |
|---|---|---|---|
| 1 | The style and numbering ladder, resolving the way Word actually resolves it | `MJXOFF-106` | `V-DOCX-01.2`, `V-DOCX-01.3`, `V-DOCX-01.4`, `V-DOCX-01.6` |
| 2 | Revision marks surviving edits | `MJXOFF-126` | `V-DOCX-01.7` |
| 3 | Header and footer variant selection | `MJXOFF-113` | `V-DOCX-02.2`, `V-DOCX-02.3` |
| 4 | Field instruction versus cached result | `MJXOFF-121` | `V-DOCX-06.4` |
| 5 | DrawingML anchoring and wrapping — **where LibreOffice diverges from Word most visibly** | `MJXOFF-131` | `V-DOCX-04.1`, `V-DOCX-05.1` |

## Excel's five, in order

From `MJXOFF-79`'s specification. **The format is the least forgiving of the three about structural
detail**, so the last row is really the first thing to try.

| # | Area | Shipped by | Checks |
|---|---|---|---|
| 1 | Number-format rendering versus what we report | `MJXOFF-108` | `V-XLSX-02.3` |
| 2 | The `xf` indirection, resolving the way Excel resolves it | `MJXOFF-108` | `V-XLSX-02.1`, `V-XLSX-02.2`, `V-XLSX-02.4` |
| 3 | Shared and array formulas surviving an edit | `MJXOFF-115` | `V-XLSX-01.4` |
| 4 | Conditional-formatting rule priority | `MJXOFF-120` | `V-XLSX-02.5` |
| 5 | **Whether Excel is content with a workbook we authored from nothing** | `MJXOFF-112` | `V-XLSX-01.1` |

## The design questions

Six checks on these pages have **no expected result**, because what they establish is a decision
rather than a fact. Each says *record which happens* and names the decision that follows. **None of
them may be marked `differs`** — there is nothing to differ from.

| Check | The question | Who decides what follows |
|---|---|---|
| `V-PPTX-01.1` | Does PowerPoint take the master's `p:bodyStyle` for a text box, or the previous hard default? | The user — the change is one revertible commit |
| `V-PPTX-01.7` | Does PowerPoint honour `a:lstStyle` > `a:defPPr` at every level? | The user — the prose or the renderer, in a resolver three formats share |
| `V-PPTX-01.8` | Is answering `+mj-sym` verbatim the right report for a slot the theme does not define? | The user |
| `V-PPTX-02.12` | Does PowerPoint drop a plain `p:contentPart`? | Already stated: if it does, `add_ink` switches to the `mc:AlternateContent` form and the MCE skip list grows by one |
| `V-PPTX-07.6` | Is `None` the right answer for a rotation-only transform? | The user — keep `None`, or resolve position and size from different tiers |
| `V-PPTX-08.7` | Does PowerPoint ignore a dangling `c:dPt`, or offer a repair? | The user — drop dangling anchors on save, or keep reporting them |

One more sits with them without being one of them: **`V-PPTX-02.6`** is not a question but a standing
instruction — *if PowerPoint ever repairs a deck the schema job passed, capture the part and add a
case.* Schema validity is necessary, not sufficient, and that boundary is where this pass's evidence
belongs.

## The platform checks

Four entries are about a machine rather than a file, and none of them can be done on the workstation
that generated the artefacts.

| Check | Platform |
|---|---|
| `V-PPTX-02.7` | The `References/` fetch script and the schema suite, on **Windows** and on **macOS** |
| `V-DOCX-01.8`, `V-XLSX-06.5` | `pip install` of the built wheel in a clean environment on **macOS** and **Windows**; and the npm package in a **real browser** — not headless — doing `File` → open → edit → `Blob` download |

## What is deliberately not covered, and why

`docs/validation/01-index.md` records the whole list and the reason for each: VML in a deck (behind a
Cargo feature, and an area that vanished from a default build would be an area the pass silently
skipped), numbered lists in a document (nothing authors a `word/numbering.xml`), Word fields
(authorable only where one already exists), worksheet tables, conditional formatting, data validation
and filters (read-only on `Workbook`), and themes (nothing in this workspace authors one, in any
format).

Those are absences by decision. The **blocked** entries listed under R2 and R3 above are different:
each is a question this pass wants answered and cannot yet ask, and each names what would unblock it.
