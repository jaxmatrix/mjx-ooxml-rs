# The validation index — every entry, and the artefact it is read against

This page is the **binding half** of the validation harness: it is where an entry id becomes a file
somebody opens. `docs/validation/00-method.md` states the id scheme, the risk levels and the result
convention; this page states, for every area, which artefact answers it.

## How this page is checked

`xtask/tests/validation_index.rs` derives two lists from two places that have nothing to do with each
other and compares them in **both** directions:

* the **entry** side is parsed out of the markdown below — the table rows, hand-written;
* the **artefact** side is a `read_dir` of the directory
  `cargo run -p xtask -- validation-artefacts` just wrote.

So an entry naming an artefact the generator does not produce fails, and an artefact no entry names
fails. Neither list is generated from the other — an index test that compares two lists produced from
one source proves nothing, and this one is written against exactly that trap. The risk level and the
area name in each row are compared against `AREAS` in `xtask/src/validation/mod.rs` for the same
reason.

The `edited` column is checked as an **`if and only if`**: an edited artefact must exist exactly when
`tests/office-authored/` holds an original for that area. The corpus is empty today, so every edited
artefact is required to be *absent* and every edit variant is required to have skipped by name. As
files arrive the same assertion starts requiring the file instead, with no edit here.

Whatever that directory holds is separately held to this library's own promise by
`xtask/tests/office_corpus.rs` — byte identity across an edit-free save at both the container and the
facade, every XML part through the fidelity tree, the package invariants, and the child-order audit
against the generated `xsd:sequence` tables. It reports its file count on every run, so a green over
an empty corpus never reads as a green over a corpus, and it proves itself able to fail by running
the same engine over four deliberately broken packages. `docs/validation/06-the-office-pass.md` §5 is
how a file gets in; `tests/office-authored/README.md` is the redistribution rule it has to satisfy.

## Presentations

| Entry | Risk | Area | Authored artefact | Edited artefact |
|---|---|---|---|---|
| `V-PPTX-01` | high | `text-inheritance` | `v-pptx-01-authored.pptx` | `v-pptx-01-edited.pptx` |
| `V-PPTX-02` | medium | `shape-appearance` | `v-pptx-02-authored.pptx` | `v-pptx-02-edited.pptx` |
| `V-PPTX-03` | high | `tables` | `v-pptx-03-authored.pptx` | `v-pptx-03-edited.pptx` |
| `V-PPTX-04` | medium | `charts` | `v-pptx-04-authored.pptx` | `v-pptx-04-edited.pptx` |
| `V-PPTX-05` | low | `pictures` | `v-pptx-05-authored.pptx` | `v-pptx-05-edited.pptx` |
| `V-PPTX-06` | low | `notes-and-links` | `v-pptx-06-authored.pptx` | `v-pptx-06-edited.pptx` |
| `V-PPTX-07` | medium | `geometry` | `v-pptx-07-authored.pptx` | `v-pptx-07-edited.pptx` |
| `V-PPTX-08` | medium | `chart-decoration` | `v-pptx-08-authored.pptx` | `v-pptx-08-edited.pptx` |

## Documents

| Entry | Risk | Area | Authored artefact | Edited artefact |
|---|---|---|---|---|
| `V-DOCX-01` | high | `text-and-inheritance` | `v-docx-01-authored.docx` | `v-docx-01-edited.docx` |
| `V-DOCX-02` | medium | `sections-and-headers` | `v-docx-02-authored.docx` | `v-docx-02-edited.docx` |
| `V-DOCX-03` | high | `tables` | `v-docx-03-authored.docx` | `v-docx-03-edited.docx` |
| `V-DOCX-04` | medium | `charts` | `v-docx-04-authored.docx` | `v-docx-04-edited.docx` |
| `V-DOCX-05` | low | `pictures` | `v-docx-05-authored.docx` | `v-docx-05-edited.docx` |
| `V-DOCX-06` | medium | `notes-comments-links` | `v-docx-06-authored.docx` | `v-docx-06-edited.docx` |

## Workbooks

| Entry | Risk | Area | Authored artefact | Edited artefact |
|---|---|---|---|---|
| `V-XLSX-01` | low | `cell-values` | `v-xlsx-01-authored.xlsx` | `v-xlsx-01-edited.xlsx` |
| `V-XLSX-02` | high | `cell-formats` | `v-xlsx-02-authored.xlsx` | `v-xlsx-02-edited.xlsx` |
| `V-XLSX-03` | medium | `grid` | `v-xlsx-03-authored.xlsx` | `v-xlsx-03-edited.xlsx` |
| `V-XLSX-04` | medium | `charts` | `v-xlsx-04-authored.xlsx` | `v-xlsx-04-edited.xlsx` |
| `V-XLSX-05` | medium | `drawings` | `v-xlsx-05-authored.xlsx` | `v-xlsx-05-edited.xlsx` |
| `V-XLSX-06` | low | `comments-and-links` | `v-xlsx-06-authored.xlsx` | `v-xlsx-06-edited.xlsx` |

## What is carried in from earlier work, unchanged

`V-XLSX-02` is the same question MJXOFF-108 (D09) already wrote out in full.
`docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md` holds a 28-row comparison of what this library answers about
a two-layer cell format, each row with its basis in ECMA-376 Part 1, and its **Excel says** and
**Verdict** columns deliberately **empty and unmarked** — not "unverified", not anything else. That
table is the detail behind `V-XLSX-02`'s checks and it is carried into this pass **exactly as
written**: no verdict in it has been touched, re-worded or re-numbered here, and none may be until
somebody opens the fixture it names in real Microsoft Excel.

## What each entry's checks say

MJXOFF-122 shipped the method page, the generators, the artefacts and the index mechanism above;
**MJXOFF-128 wrote the checks** — the `V-…-NN.n` lines, each with what to look at, the three call
chains, and an unfilled result line — on four pages beside this one:

| Page | What it holds |
|---|---|
| `docs/validation/02-risk-order.md` | the order to work through, R1 first, and the design questions that have no expected result |
| `docs/validation/03-presentations.md` | `V-PPTX-01` … `V-PPTX-08` |
| `docs/validation/04-documents.md` | `V-DOCX-01` … `V-DOCX-06` |
| `docs/validation/05-workbooks.md` | `V-XLSX-01` … `V-XLSX-06` |
| `docs/validation/06-the-office-pass.md` | **how to run the pass**: the order, what a failure looks like against a documented gap, what to save out of Office and where to put it, and the decisions the pass settles rather than checks (MJXOFF-130) |
| `docs/validation/07-the-reference-pack.md` | **the renderer's half of the same morning**: four files to open and export to PDF, nothing to judge, and the seven questions that answers (MJXOFF-207) |
| `docs/validation/08-the-fidelity-oracle.md` | **the one step a machine cannot take**: five pictures to look at and either approve or refuse, why an unapproved baseline fails, and what the oracle will not claim (MJXOFF-165) |

Two of the areas above are MJXOFF-128's own: `V-PPTX-07` and `V-PPTX-08` exist because writing the
checks found harvested expected results — a chevron's adjustment maximum, a slice exploded 25 %, a
polynomial trendline of order 3, the sixteen plot types — with no file to check them against, and
this page's own rule is that an entry may not describe an artefact nobody produces.

Two gates hold those pages to this one. `xtask/tests/validation_index.rs` refuses an entry id that
binds to no artefact, in either direction. `xtask/tests/validation_calls.rs` refuses a call chain
naming a method the facade, the Python stub or the WebAssembly binding does not have — comparing the
documented camelCase name against the `js_name` the binding publishes for that exact Rust method —
and refuses an artefact name that is neither generated, nor a committed fixture, nor an example's
source. A check with no artefact at all has to say **blocked** and name what would unblock it.

## Areas the harness deliberately does not cover, and why

These are recorded here rather than left as silence, because an absence a reader has to notice is an
absence nobody notices.

* **VML legacy content in a presentation.** `Deck`'s VML accessors sit behind `mjx-pptx`'s optional
  `vml` feature. An area that vanished from a default build would be an area the pass silently
  skipped, so no area uses one — the whole catalogue is reachable from a default `cargo build`.
* **Numbered lists in a document.** `Document::attach_paragraph_to_list` writes a `w:numPr`, and
  nothing in this workspace authors a `word/numbering.xml` for it to point at. An artefact built from
  `Document::blank` carrying one would have a reviewer judging a dangling reference rather than this
  library, so `V-DOCX-01` does not write one; `Document::effective_run_properties` raises
  `no w:num with numId 1` on such a paragraph, which is how the omission was found.
* **Word fields.** The facade can re-instruct a `w:fldSimple`/`w:instrText` that already exists but
  cannot author one, so no artefact built from `Document::blank` can contain a field. The edit
  variant of `V-DOCX-06` will reach them once the corpus has an original that has one.
* **Worksheet tables, conditional formatting, data validation and filters.** `Workbook` reads all
  four and authors none of them, which is stated as a deliberate limitation in
  `crates/mjx-xlsx/docs/guide/deliberate_limitations.md`. Per §5 of the method page, a documented gap
  is not what this pass is for.
* **Themes.** Nothing in this workspace authors a theme, in any format; `crates/mjx-xlsx`'s guide
  names it as a gap with no ticket. `V-PPTX-01` and `V-PPTX-02` reach the *blank deck's* theme
  through scheme colours, which is as far as the surface goes.
