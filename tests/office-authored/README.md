# Office-authored originals

**This directory is the slot the validation pass's *edit* variants read from, and it is empty.**

`cargo run -p xtask -- validation-artefacts` produces two artefacts per area. The `authored` one is
built from `Deck::blank` / `Document::blank` / `Workbook::blank`, so every byte in it is one this
library wrote. The `edited` one is produced by *opening an original this library did not write and
editing it* — a different code path, and the only one that exercises edit isolation against markup
we have never seen.

Nothing in this repository has ever read a file real Microsoft Office wrote. Filling this directory
is **MJXOFF-130 (F3)**'s work, together with everything that comes with it: what may be committed,
how a file is checked on the way in, and the `mc:Ignorable` / `CT_Extension` trap the first ingestion
will hit.

## The naming convention the generator uses

One file per area, named by the area's own entry id, lower-cased, with the format's conventional
extension:

```
tests/office-authored/v-pptx-03.pptx     the original V-PPTX-03's edit variant is built from
tests/office-authored/v-docx-01.docx
tests/office-authored/v-xlsx-02.xlsx
```

`cargo run -p xtask -- validation-artefacts --list` prints every area id. An area with no file here
**skips by name**: the run prints the area and the exact path it looked for, never a silent absence.
`MJX_REQUIRE_OFFICE_CORPUS=1` turns any such skip into a hard failure — the same arrangement
`MJX_REQUIRE_SOFFICE=1` makes for the `office_open` canary.

## Why this is not under `tests/fixtures/`

`mjx-fixtures` derives every byte-identity corpus and the schema gate's corpus from a `read_dir` of
`tests/fixtures/`, and `assert_every_fixture_has_a_known_kind` fails on any entry it cannot classify —
a subdirectory included. These files are inputs to a *human* pass, not committed fixtures this
library round-trips, so they live beside that corpus rather than inside it.
