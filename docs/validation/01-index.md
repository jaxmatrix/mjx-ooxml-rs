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
artefact is required to be *absent* and every edit variant is required to have skipped by name. When
MJXOFF-130 fills the corpus, the same assertion starts requiring the file instead, with no edit here.

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

Nothing yet. This child (MJXOFF-122) ships the method page, the generators, the artefacts and the
index mechanism above; **MJXOFF-128 (F2) writes the checks** — the `V-…-NN.n` lines, each with what to
look at and an unfilled result line, on per-format pages beside this one. Until then this page is the
complete list of what the pass covers, and the mechanism that will bind those checks to files is
already load-bearing: an id F2 invents that names no artefact fails the same test that guards the
rows above.

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
