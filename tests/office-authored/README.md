# Office-authored originals

**This directory is the corpus of files real Microsoft Office wrote, and it is empty.**

That is not an oversight and it is not a to-do. Everything else in this repository can be built by an
agent; this cannot. A file's value here is **entirely its provenance** — it is interesting precisely
because nobody in this project chose a single byte of it — so a file written by this library and
called "PowerPoint-authored" would not be a shortcut, it would be a permanent lie in the one place
the project has no other defence against one. The corpus is filled by a person with Office in front
of them, working through
[`docs/validation/06-the-office-pass.md`](../../docs/validation/06-the-office-pass.md).

## What this retires

Every fixture under `tests/fixtures/` was written by this project or by LibreOffice, so every gate in
the workspace has only ever proved that our reader agrees with our writer. All three formats' gaps
pages carry the row — *no test in this repository reads a file that Microsoft Office wrote* — and the
risk register calls it **R2**. A file in this directory retires that row for every check that reads
it, which is why one file is worth more here than a hundred more of ours.

## The redistribution rule

**A committed file must be one we authored the content of.** Not "found on the internet", not "a
template that came with Office", not a document from a customer, a colleague or a public dataset. The
file is written by Office; the *content* — the words, the numbers, the pictures, the styles — is ours
to publish.

The rule is checked **per file, before committing, and recorded** in the table below. Concretely, a
candidate qualifies only if all of these hold:

| Question | It qualifies when |
|---|---|
| Who typed the content? | You did, into an empty document. |
| Does it embed anything from elsewhere? | No image, font, logo, chart data, sample text or clip-art that came from somewhere else. |
| Does it start from a template? | No — start from *Blank*. Office's bundled templates are Microsoft's, and a theme lifted from one is redistributed with the file. |
| Does it carry personal data? | No. Office writes the author's name into `docProps/core.xml`; clear it, or accept that the name ships in this repository for ever. |
| Are its rights clear? | **Yes, without argument.** If they are not, the file does not go in. |

> **An unclear case is not resolved quietly.** A file whose rights need a paragraph of reasoning is a
> file to leave out, and to *say* was left out — in the ingestion note below and in the pull request.
> "Probably fine" is how a repository acquires something it cannot relicense.

## What is in here today

| File | Entry | Office version | Content authored by | Checked on |
|---|---|---|---|---|
| *(none yet)* | — | — | — | — |

Add a row when you add a file. The table is the record the redistribution rule asks for, and a file
with no row is a file whose rights nobody stated.

## The naming convention

One file per area, named by that area's own entry id, lower-cased, with the format's conventional
extension:

```
tests/office-authored/v-pptx-03.pptx     the original V-PPTX-03's edit variant is built from
tests/office-authored/v-docx-01.docx
tests/office-authored/v-xlsx-02.xlsx
```

`cargo run -p xtask -- validation-artefacts --list` prints every area id. The name is not decoration:
it is how a file binds to the check it answers, and `xtask/tests/office_corpus.rs` refuses a file it
cannot bind, because an unbound file is one no edit variant reads and no result line refers to.

An area with no file here **skips by name**: the run prints the area and the exact path it looked
for, never a silent absence. `MJX_REQUIRE_OFFICE_CORPUS=1` turns any such skip into a hard failure —
the same arrangement `MJX_REQUIRE_SOFFICE=1` makes for the `office_open` canary and
`MJX_REQUIRE_SCHEMA=1` for the schema gate. **Do not set it until the corpus has files**; an empty
corpus is the honest state of this project today.

## Getting a file in

```sh
# 1. Report on it before it goes anywhere. Nothing is copied, nothing is committed.
cargo run -p xtask -- validation-artefacts --ingest ~/Desktop/whatever-office-saved.xlsx --area 2

# 2. Read the report. If every check `held` or `reported`, put the file where the report says.
cp ~/Desktop/whatever-office-saved.xlsx tests/office-authored/v-xlsx-02.xlsx

# 3. The suite now finds it by walking the directory.
cargo test -p xtask --test office_corpus

# 4. And the edit variant of that area starts being produced instead of skipping.
cargo run -p xtask -- validation-artefacts --format xlsx --area 2
```

## What the checks mean, and which of them may fail

An ingested file is **not ours**, and that single fact decides what a check is allowed to do with it.

| Verdict | Means |
|---|---|
| `held` | The check ran and held. |
| `reported` | The check found something that belongs to **the file**, not to this library. It is printed in full and fails nothing. |
| `FAILED` | The check found something that belongs to **this library**. |

Four checks are ours and fail: per-part byte identity across an edit-free save, every XML part
through the fidelity tree, the same round-trip through the *facade* (`Deck` / `Document` /
`Workbook`, so the format model is held to markup nobody here wrote), and the child-order audit
against our generated `xsd:sequence` tables. Two are the file's and are reported: a package defect it
**arrived** with — A7b's rule is that such a file must still open and re-save unchanged — and a part
its producer wrote that the ECMA-376 XSDs reject. That second one is not hypothetical: MJXOFF-103
measured Apache POI 5.5.1 writing an empty `<c:tx/>` for an unnamed chart series, which
`dml-chart.xsd` rejects outright.

## The one failure to expect first, and why it is ours

**An `<ext>` in a namespace your file declares `mc:Ignorable` is reported on by nobody, and that is a
residue of the gate rather than a clean bill of health.**

`mjx-schema-gate` validates the *markup-compatibility-resolved* view of a part, because
`mc:Ignorable` names attributes the base schema has no declaration for. Resolution removes an
ignorable element **together with its content** — and `sml.xsd`'s `CT_Extension` and
`dml-chart.xsd`'s declare their whole content model as a bare `<xsd:any processContents="lax"/>`,
whose `minOccurs` therefore defaults to **1**. An `<ext>` whose only child was ignorable was emptied
by the resolution and then rejected, on every conformant file Office has written since 2010:

```
Element '{…/spreadsheetml/2006/main}ext': Missing child element(s). Expected is one of ( {*}*, * ).
```

MJXOFF-196 closed that. An extension slot markup-compatibility resolution empties is now dropped
along with the extension it held: such an element exists *only* to carry that extension, and
ignoring the extension without ignoring the slot is half a resolution. The rule fires only on the
elements `crates/mjx-schema-gate/src/wildcard_slots.rs` names — **derived from the pinned XSDs by
test**, five of them — and only when the source element had children, so an `<ext/>` written empty
is still reported. `xtask/tests/mce_extension_seam.rs` holds every part of that.

**What is left is a residue worth knowing before you read a report.** The gate says nothing about
markup *inside* an ignorable extension, and it never could: `CT_Extension`'s wildcard is
`processContents="lax"` and no schema for such a namespace is loaded, so a validator handed the
content would accept it unread. `pml.xsd`'s own `CT_Extension` and `dml-main.xsd`'s
`CT_OfficeArtExtension` both say `minOccurs="0"`, which is why the seam reached presentations and
documents through their *charts* rather than through their main parts.

## Why this is not under `tests/fixtures/`

`mjx-fixtures` derives every byte-identity corpus and the schema gate's corpus from a `read_dir` of
`tests/fixtures/`, and `assert_every_fixture_has_a_known_kind` fails on any entry it cannot classify —
a subdirectory included. These files are inputs to a *human* pass rather than committed fixtures this
library round-trips as a matter of course, so they live beside that corpus rather than inside it.
