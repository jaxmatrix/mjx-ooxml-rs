# The reference pack — the renderer's half of the same morning

**This page is addressed to the same one person `06-the-office-pass.md` is: you, with Microsoft
Office in front of you.** That page is about *opening* artefacts and writing down what you see. This
one asks for something narrower and much more mechanical: **open four files, export each one to PDF,
and put the PDFs in one directory.** Nothing here asks you to look at anything, judge anything, or
decide whether a picture is right. A machine reads the answers out of the PDFs afterwards.

It exists because seven questions have accumulated that no agent can answer, and arranging a licensed
Windows machine with Office on it is expensive. So they are answered **once, together**, and the
whole cost to you is four exports.

---

## 1 · Before you sit down

```sh
cargo test --workspace
cargo run -p mjx-reference-pack -- generate target/reference-pack
```

Five files land in `target/reference-pack/`: the four artefacts and an `INSTRUCTIONS.md` generated
from the same tables the artefacts are, so it can never describe a pack that does not exist. Copy the
whole directory to the Windows machine.

Note the workspace version from `Cargo.toml` before you start. Every answer you bring back is a
statement about *that build*.

**The generator is deterministic**: running it twice produces the same bytes. If you generate the
pack on a Monday and export it on a Thursday, `cargo run -p mjx-reference-pack -- generate` again and
compare — the harness's plate map is only correct for the pack it was generated from, and a map that
is off by one shape turns every row of the report into a statement about a different preset.

---

## 2 · The four artefacts

| File | Application | What it asks |
|---|---|---|
| `01-presets-at-their-defaults.pptx` | PowerPoint | all 187 preset shapes, each at its own default adjustments — eight slides, twenty-four plates to a slide |
| `02-presets-at-their-extremes.pptx` | PowerPoint | the same 187, with **every handle pushed to an end of its own domain** |
| `03-type-specimens-and-hatches.pptx` | PowerPoint | advance rulers for five font families, their line pitch, and all fifty-four preset hatches |
| `04-hanging-punctuation.docx` | Word | whether Word hangs ASCII `,` and `.` past the measure in a Japanese paragraph, and in a Latin one |

The fourth is a `.docx` and not a deck for a reason worth knowing: **`w:overflowPunct` is a
WordprocessingML setting.** There is no way to ask a `.pptx` the hanging question at all.

---

## 3 · What to do, for each file

1. **Open it in the real Microsoft application.** Not LibreOffice, not a viewer, not the web version.
2. **Do not edit it.** Do not click anything. If Office offers to *repair* the file, **stop and
   report that** — a file this library wrote that Office will not open is a more valuable finding
   than the whole rest of the pass, and repairing it would destroy the evidence.
3. **File → Export → Create PDF/XPS**, or **Save a Copy** and choose PDF as the type.
4. Save it beside the original, keeping the name: `01-presets-at-their-defaults.pdf`, and so on.
5. Copy the four PDFs into `tests/office-exports/` in the repository and commit them.

That is the whole job. Four exports.

### What NOT to do

* **Do not print to a PDF printer.** A print driver rasterises the page; the export keeps the vectors
  and the glyph positions, and those are the entire measurement.
* **Do not open the files in LibreOffice and export from there.** That comparison already runs on
  every machine in this project and is explicitly not evidence — see §6.
* **Do not change the theme, the fonts, the slide size or the page size.**
* **Do not "clean up" a slide that looks wrong.** A slide that looks wrong is a result.
* **Do not put anything else in `tests/office-authored/`.** Its value is entirely its provenance, and
  the same rule that governs the corpus of Office-authored originals governs this directory: no agent
  may fill it, and neither may a file this project wrote.

### Two things worth writing down while you are there

Neither is required and both are cheap:

* **Which font Word substituted** for the Japanese text in `04-hanging-punctuation.docx`. The
  document deliberately names no East Asian face, so *what Word chose* is itself part of the answer.
* **The Office version and build**, from *File → Account → About*. Every number in the pack is a
  statement about one build of one application.

---

## 4 · The seven items, and which file answers each

These are the keys the harness reports under. They are declared in
`crates/mjx-reference-pack/src/lib.rs` as `THE_SITTING`, and
`crates/mjx-reference-pack/tests/the_instructions_are_complete.rs` fails if an item gains a key there
and not a paragraph here.

### `preset-geometry-defaults` — `01-presets-at-their-defaults.pptx`

Does every one of the 187 preset shapes draw, at its default adjustments, the outline our generated
path table resolves? Read by cropping one window per plate out of both rasters, with `pdftoppm`
rasterising **both** sides at one DPI so antialiasing cancels.

### `preset-geometry-extremes` — `02-presets-at-their-extremes.pptx`

And at an end of every one of its handles — which is the case a default is most likely to be right by
accident at. Twenty-two shapes (every callout and every bent or curved connector) carry ECMA-376's
`i32::MAX` *"this handle has no stop"* sentinel; their plates are captioned `— N clamped`, because a
handle dragged to the literal sentinel puts the shape 3 435 973 pixels off the slide. Three plates
are captioned `— no value here`: `circularArrow`, `leftCircularArrow` and `leftRightCircularArrow`
have no finite geometry at one of their own stops, which is the shape's own formula and not a defect
on either side. **PowerPoint will draw something there and we correctly draw nothing; that is not a
finding.**

### `up-arrow` — `01-presets-at-their-defaults.pptx`

What does `upArrow` draw? `presetShapeDefinitions.xml` publishes no geometry for it — the file simply
omits the shape — so this is **the one plate in the whole pack for which an Office export is the only
possible source of truth**. It cannot be checked structurally, differentially, or against
LibreOffice. Its plate is captioned `— no published geometry`, and the crop of your export *is* the
answer.

### `cambria-metrics` — `03-type-specimens-and-hatches.pptx`

What are Cambria's advance widths and line pitch? Microsoft publishes no width table for it and no
metric collection surveyed for `mjx-text` carries one, so its entry is `Unverified` and holds no
numbers at all. Read from two word boxes per character — `HcH` and `HH` — whose spans differ by
exactly one advance, and from six lines at 60 pt for the pitch.

### `clone-transcribed-advances` — `03-type-specimens-and-hatches.pptx`

Are Arial's, Times New Roman's and Courier New's advances the ones Microsoft's faces actually carry?
The table's numbers are transcribed from the **(URW)++ Nimbus clones** and labelled `Published`,
which is a stronger word than the source supports. Same advance ruler, same 92 characters.

### `japanese-hanging-set` — `04-hanging-punctuation.docx`

Does Word hang ASCII `,` and `.` in a Japanese paragraph with `w:overflowPunct` on — and does it hang
them in a Latin paragraph in the same document? `mjx-text`'s
`KinsokuRules::japanese_standard` puts eight characters in the hangable set, of which the last two
are ASCII, and its own module documentation leaves the question open by name. The document has three
paragraphs: Japanese with the setting on, the same Japanese with it off, and Latin with it on. Read
from the last word box of each line against that paragraph's own measure.

### `pictorial-hatches` — `03-type-specimens-and-hatches.pptx`

What do the ten pictorial preset hatches actually look like? ECMA-376 names fifty-four preset
patterns and gives a bitmap for none of them; `mjx_paint::PATTERN_MASKS` derives forty-four of them
from what their names state — a coverage, or a direction and a period — and **draws ten by hand**:
`smConfetti`, `lgConfetti`, `plaid`, `sphere`, `weave`, `divot`, `shingle`, `wave`, `trellis`,
`zigZag`. Those ten are the approximate part of the table and the swatch sheet is what replaces them.

---

## 5 · What happens to the PDFs

```sh
cargo run -p mjx-reference-pack -- preliminary target/reference-pack   # LibreOffice, provisional
cargo run -p mjx-reference-pack -- ingest                              # your exports, authoritative
```

Both report one row per plate. **`ingest` takes no directory and no provider argument**, deliberately:
it reads `tests/office-exports/` and stamps every row *Microsoft Office*, and a
command that took either argument would let a LibreOffice PDF be pointed at it and recorded as
Office's. `preliminary` hard-codes LibreOffice for the same reason in the other direction. **Only
after your exports are in that directory** may a row say anything about parity.

Every row carries where its reference came from and how much that is worth
(`mjx_text::ReferenceAuthority`), and a row may be recorded as parity only when **both** halves hold:
the provider is authoritative *and* the verdict is an agreement.

---

## 6 · ⚠ Why the LibreOffice run that already passes is not this

LibreOffice is installed on every machine in this project and converts all four artefacts today. That
run is **a change detector** — *"this used to render and now does not"* — and it is worth having
before you spend a morning on the same shapes. It is not parity, it has never been parity, and the
code refuses to record it as parity: every LibreOffice row is `Provisional`, and the count of rows
that may be called parity is zero by construction rather than by anyone remembering.

Two exclusions ride on the same rule, and they point in opposite directions:

* **LibreOffice's export does not reproduce gradients and shades.** A pixel difference there is the
  *reference's* fault, and nothing in a pixel diff says which way round it is. So that content is
  excluded, the exclusion is named in the output rather than silent, and an excluded result is
  neither a pass nor a failure.
* **The fifty-four hatches are excluded too, for the opposite reason**: they are the thing being
  measured, and comparing our masks against LibreOffice's would ratify *LibreOffice's* drawing as
  ECMA-376's.

Both exclusions are attached to the **provider**, so both lift by themselves the moment your exports
arrive. Nobody has to remember to revisit them.

**The preset sheets are excluded from nothing.** They are authored with solid fills and solid strokes
precisely so that their exclusion list is empty: an exclusion that covered every sheet would prove
nothing, and one carried onto a sheet that did not need it would quietly remove that sheet from the
comparison.

---

## 7 · What the pack will still not tell you

* **Nothing about a document layout engine**, because there is not one yet. The comparison is between
  the geometry PowerPoint drew and the geometry our preset table resolves, on a grid both sides were
  told. It is not *"our renderer opened your deck"*.
* **The `hhea` triple separately.** A word's bounding box is *ink*, so the top of a line is the top of
  an `H` and not the font's ascender. The pack measures line **pitch**, which is the number pagination
  rests on, and says so.
* **A hatch bitmap the export blurred.** The reader recovers a tile only when the render is periodic
  enough to find a period, and reports that it could not otherwise. It always reports the coverage.
