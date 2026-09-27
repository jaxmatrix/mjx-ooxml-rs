# The reference pack's Office exports

**This directory holds the four PDFs Microsoft Office exported from the reference pack, and it is
empty.**

That is not an oversight and it is not a to-do. It is the same rule, for the same reason, as
`tests/office-authored/`: **a file's value here is entirely its provenance.** These four PDFs are
interesting precisely because nobody in this project chose a single glyph position in them. A PDF
this library produced — or LibreOffice produced — and called "PowerPoint-exported" would not be a
shortcut, it would be a permanent lie in the one place the project has no other defence against one.

**No agent may fill this directory.** It is filled by a person with Office in front of them, working
through [`docs/validation/07-the-reference-pack.md`](../../docs/validation/07-the-reference-pack.md).

## Why this is beside `tests/office-authored/` and not inside it

The two hold different things. That one holds Office-**authored originals** — a `.pptx` whose markup
nobody in this project chose, which is what retires the risk register's R2. This one holds Office's
**rendering** of files this project authored, which is what answers a question about geometry and
metrics. Nesting them conflated the two, and `xtask`'s corpus walker says so itself: *"the corpus is
one flat directory of Office-authored packages; a subdirectory has no meaning here and would sit
outside every check."* It is right — this directory is outside its checks and has its own.

## What goes in here

Exactly four files, named after the artefacts they came from:

```
01-presets-at-their-defaults.pdf
02-presets-at-their-extremes.pdf
03-type-specimens-and-hatches.pdf
04-hanging-punctuation.pdf
```

Generate the artefacts with `cargo run -p mjx-reference-pack -- generate target/reference-pack`, open
each in the real Microsoft application, and use **File → Export → Create PDF/XPS**. Do not print to a
PDF printer: a print driver rasterises the page, and the vectors and glyph positions are the entire
measurement.

## What is in here today

| File | Office version and build | Exported on | By |
|---|---|---|---|
| *(none yet)* | — | — | — |

Add a row when you add a file. A file with no row is a measurement whose provenance nobody stated,
which is the only thing this directory has.

## The redistribution rule

Every one of these PDFs is a rendering of content **this project authored** — 187 shape outlines, a
type specimen of characters, fifty-four hatches and a paragraph of made-up text. Nothing in the pack
came from a template, a sample document, a customer or the internet, and nothing in it is anybody
else's to publish. That is a deliberate property of the generator and it is what makes committing the
exports straightforward where committing an Office-*authored* original needs the checklist one
directory up.

The one thing an export does carry that the artefact did not: **font programs are not embedded by
this rule.** A PDF export embeds subsets of the faces used. Arial, Times New Roman, Courier New,
Calibri, Cambria and whatever Word substitutes for the Japanese text are Microsoft's, and their
outlines would ride along inside the file. **Export with font embedding turned off**, or accept that
those subsets ship in this repository — and if the exporter will not let you turn it off, say so in
the pull request rather than committing quietly. Advance widths are facts about a font; a font
program is not.

## Until then

`cargo run -p mjx-reference-pack -- preliminary` runs against LibreOffice instead, and every row it
produces is `Provisional`. The count of rows that may be called parity is **zero by construction**.
That is the honest state of the project, and it stays that way until somebody exports four files out
of Microsoft Office.
