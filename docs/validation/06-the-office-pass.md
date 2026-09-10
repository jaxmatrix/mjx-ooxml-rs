# Running the pass — the hand-off

**This is the last thing the sixty-two-child programme produced, and it is addressed to one person:
you, with Microsoft Office in front of you.** Everything else here can be checked by a machine, and
has been. This cannot. The pages beside this one say what to look at; this one says what to *do*,
in what order, on which morning, and what to do with what you see.

Nothing in this repository has ever read a file Microsoft Office wrote. That is the deepest weakness
the project has, it is recorded as **R2** in `docs/validation/02-risk-order.md`, and it is the one
weakness no agent can retire — because the value of an Office-authored file is entirely its
provenance, and a file this library wrote and called "PowerPoint-authored" would be a permanent lie
in the one place there is no other defence against one.

### Provenance is how a file got here, and `docProps/app.xml` does not say (MJXOFF-249)

The sentence above has an obvious-looking shortcut, and it is wrong. `docProps/app.xml` carries an
`<Application>` element, every Microsoft application writes its own name into it, and a reader
checking this repository for Office provenance would naturally grep for that. **Two committed
fixtures answer, and neither was written by Microsoft:**

| Fixture | Claims | Actually written by |
|---|---|---|
| `tests/fixtures/charts.pptx` | `Microsoft Macintosh PowerPoint` 14.0000 | python-pptx — its own `docProps/core.xml` says `generated using python-pptx` |
| `tests/fixtures/comments_third_party.xlsx` | `Microsoft Excel` 12.0000 | XlsxWriter 3.2.9, which emits that element verbatim as a compatibility measure |

Every element in a file is something a producer *chose* to write, and a producer may choose to write
somebody else's name. So **provenance is a fact about how a file reached this repository, not about
its bytes** — it lives in the commit that added the file and in the person who ran the application,
and there is no element that can be read instead.

The repository had already made this mistake before it noticed it: `crates/mjx-pptx/tests/charts.rs`
stated that `charts.pptx` *was written by PowerPoint*, and
`crates/mjx-ooxml/tests/preservation/deck_cases.rs` drew a conclusion about what PowerPoint writes
from that file's markup. Both were corrected under MJXOFF-249, and
`xtask/tests/fixture_provenance.rs` is what stops the next one: no fixture's `Application` element
may name Microsoft unless a person has written down, in that file's ledger, how the file actually
got here. The gate asserts that negative and classifies nothing — a gate that read the element and
decided who wrote a file would be building the very inference this section forbids, and would have
been wrong about both rows above on its first run.

---

## 1 · Before you sit down

```sh
cargo test --workspace                      # everything a machine can answer, answered
cargo run -p xtask -- validation-artefacts  # the files you will open
cargo run -p xtask -- validation-artefacts --list
```

The artefacts land in `target/validation-artefacts/`, two per area — `…-authored` built from a blank
document, `…-edited` built by editing an Office-authored original. **Every `-edited` one is missing
today**, and the run says so by name, because the corpus at `tests/office-authored/` is empty. Filling
it is §5, and it is the half of this pass that changes the repository.

Note the workspace version from `Cargo.toml` before you start. Every result you record is a statement
about *that build*, not about the project.

You need Office. You do not need anything else: `xmllint`, LibreOffice and the schema trees are for
the machine half, which has already run.

---

## 2 · The loop, in one paragraph

Open an artefact in the real application. Look at the one thing the check names. Write one line into
the check's `Result:` slot in `03-presentations.md`, `04-documents.md` or `05-workbooks.md` — version,
date, initials, verdict. Move on. When something differs, §4 is how to tell a finding from a
documented gap and §10 is how to file one; when the file came out of Office rather than out of
`xtask`, §5 says how to keep it.

The verdict vocabulary is fixed by `00-method.md` §3 and has exactly three words — `as intended`,
`differs`, `blocked` — and deliberately **no word that reads like a test result**. An unfilled result
is not a failure. It is the honest state of a check nobody has reached.

---

## 3 · The order, and where to stop

`docs/validation/02-risk-order.md` is the running order and this page does not repeat it. Two things
about it are worth restating, because they change what you do on the day:

**Start at R1, and stop at the first `differs` there.** R1 is `V-PPTX-01.1` — whether PowerPoint
takes the master's `p:bodyStyle` for a plain text box, as ECMA-376 §19.3.1.35 says, or the hard
default it used before 0.0.58. Everything below R1 was written assuming R1 resolves. If PowerPoint
disagrees, the text ladder underneath it is being validated against a resolver that is about to
change, and the rest of `V-PPTX-01` is wasted effort until you decide. The change is isolated in one
revertible commit precisely so that this decision is cheap.

**R6's canary is worth nothing.** Table styles are where LibreOffice diverges from Office most, so
`V-PPTX-03` cannot be pre-screened by opening the file in anything else first.

After R1–R7, Word's five and Excel's five are each in their own table on that page, in the order
their specifications put them. Excel's last row — *whether Excel is content with a workbook we
authored from nothing* — is the one to try first among them, because the format is the least
forgiving of the three about structural detail and a repair prompt there invalidates everything under
it.

---

## 4 · What a failure looks like, and what a documented gap looks like

This is the distinction the whole pass turns on, and it is worth three examples.

| What you see | What it is | What to do |
|---|---|---|
| Office shows a repair prompt, or renders something the check says it should not | A **finding**. | §10 — confirm against the other variant of the same area, then file it. |
| The feature is simply not there, and the format's gaps page lists it | A **documented gap**. | Nothing. `00-method.md` §5: *a documented gap is never a validation failure.* Read the gaps page before filing. |
| The check has no artefact to open | **Blocked**, and it says so already. | Record `blocked` with the reason, or supply the file that unblocks it — §6. |

The three gaps pages are `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md`,
`crates/mjx-docx/docs/guide/fidelity_and_gaps.md` and
`crates/mjx-xlsx/docs/guide/deliberate_limitations.md`. Each keeps two lists apart on purpose: what
the library **decides** not to do, and what is **built but not yet verified against Office**. Only the
second list is what this pass is for. A file that does not render something the library deliberately
preserves rather than models is behaving exactly as designed.

**One standing instruction sits with these**, from `V-PPTX-02.6`: *if PowerPoint ever repairs a deck
the schema job passed, capture the part and add a case.* Schema validity is necessary and not
sufficient, and that boundary is where this pass's evidence is most valuable.

---

## 5 · Filling the corpus — the part that changes the repository

Every artefact you open came out of `xtask`. Every file you **save back out of Office** is a
candidate for something this project has never had.

### What to save, and from which application

| Save this | From | As | Closes |
|---|---|---|---|
| The chart artefact, re-saved after *Edit Data* | PowerPoint | `v-pptx-04.pptx`, in `tests/office-authored/` | `V-PPTX-04.4` — and it is the file R2 names as the corpus's first |
| A blank presentation with one text box and one shape, typed from *Blank* | PowerPoint | `v-pptx-01.pptx` / `v-pptx-02.pptx` | the `-edited` half of the text and appearance areas |
| A blank document with two paragraphs and a two-column table | Word | `v-docx-01.docx` / `v-docx-03.docx` | the `-edited` half of Word's ladder and tables |
| A blank workbook with a few typed cells and one cell you have formatted by hand | Excel | `v-xlsx-01.xlsx` / `v-xlsx-02.xlsx` | the `-edited` half of the value and format areas |
| Any of the above re-saved *by Office* after this library edited it | any | the same name | the strongest check there is: Office accepted our edit and wrote it back |

**Start from *Blank*, never from a template.** Office's bundled templates are Microsoft's, and a theme
lifted from one is redistributed with the file. The full rule, and the table each committed file gets
a row in, is `tests/office-authored/README.md`.

### The command to run on it

```sh
# Report on the file before it goes anywhere. Nothing is copied and nothing is committed.
cargo run -p xtask -- validation-artefacts --ingest ~/Desktop/Book1.xlsx --area 2

# If the report is clean, put it where the report says, and the walk finds it.
cp ~/Desktop/Book1.xlsx tests/office-authored/v-xlsx-02.xlsx
cargo test -p xtask --test office_corpus

# The edit variant of that area now gets produced instead of skipping by name.
cargo run -p xtask -- validation-artefacts --format xlsx --area 2
```

### Reading the report

Each check reports one of `held`, `reported`, `skipped` or `FAILED`, and the difference is the whole
design. **`FAILED` is about this library** — byte identity across an edit-free save, every XML part
through the fidelity tree, the same round-trip through the facade, the child-order audit against our
generated `xsd:sequence` tables, an address the typed model could not read, and an edit that moved
more than the text leaf it was aimed at. **`reported` is about the file** — a package defect it
arrived with, markup its producer wrote that the ECMA-376 XSDs reject, or a reference the file makes
that resolves to nothing inside it. A third-party file is not necessarily schema-valid: Apache POI
5.5.1 writes an empty `<c:tx/>` for an unnamed chart series, which `dml-chart.xsd` rejects outright,
and reddening a build over that would teach nobody anything.

Two of the rows are not about bytes at all, and they are the only ones that build a typed element
(MJXOFF-278). **`model`** opens the file as a `Deck`, a `Document` or a `Workbook` and walks all of
it — every surface, shape, table cell, paragraph and run, and the `effective_*` inheritance ladders
over each — then prints its counts and asserts that the whole package still saves byte-identically,
because *reading must not dirty a part*. **`edit`** replaces one text leaf through the model (one run
per slide, one run in a document body, one numeric cell per sheet) and asserts that the bytes the
save inserted are exactly the bytes that were set. A count of zero in the `model` row, or a `skipped`
in the `edit` row, is the honest report that there was nothing there to read or to edit — never a
pass.

### The first real report, and what it did and did not retire

**A file Microsoft Office wrote has now been through this command, once, at 0.0.165.** It was a
12-slide PowerPoint deck — `<Application>Microsoft Office PowerPoint</Application>`,
`AppVersion 16.0000`, 3,927,263 bytes, 135 ZIP entries — saved out of PowerPoint by the repository's
owner and handed over in conversation. **It is not committed and R2 stands**: the corpus below is
still empty, `MJX_REQUIRE_OFFICE_CORPUS=1` still turns every area into a skip, and a measurement
taken outside the repository is not a test inside it.

What it reported, verbatim in the verdict vocabulary above: **nine checks `held` and none `FAILED`.**
135 entries re-saved with every payload byte-identical; all 106 XML parts through the fidelity tree
byte-identical; the facade's own open-and-re-save leaving all 135 unchanged; every OPC invariant
holding before and after; the typed model reading the whole deck and editing it (below); 52 of the 52
parts the category tables require audited clean for child order; 102 parts schema-valid against the
ECMA-376 XSDs.

The one `reported` row is worth reading closely, because it is **not** a file that failed a schema.
All 23 of its rows are `UNCATEGORISED` — 19 `image/svg+xml` pictures in `ppt/media/`, two modern
comment parts, an authors part and a revision-info part, in three namespaces the gate's category
tables have no arm for. That is a gap in the instrument rather than a deviation in the markup, and
only a real Office file could ever have shown it: **MJXOFF-277**.

**Two of those nine rows are MJXOFF-278's, and at 0.0.165 they did not exist.** Until then every
check was byte identity or laziness — including `facade`, which never builds a typed element — so the
report established that the container survived and said nothing about whether we can *read* what
Office writes. The two checks now say it, and this is what they said about that deck:

- **45 surfaces** (12 slides, 10 notes slides, 20 layouts, 2 masters, 1 notes master) and **252
  shapes**, 203 of them carrying a text body, reached by descending into **3 groups**; **4 tables**
  of 151 cells.
- **323 paragraphs, 253 runs, 9,891 characters** in the shapes themselves, and a further **142 runs
  of 1,644 characters** inside those table cells.
- **253 `effective_run_properties` and 252 `effective_shape_fill` resolutions** — the R1 and R5
  ladders, run over markup nobody here wrote — with **no address the model refused**, and all 135
  entries still byte-identical afterwards, so **reading dirtied no part**.
- Then one run's text replaced on each of the 12 slides and a `save()`: **exactly 12 of the 135
  entries changed**, and across **377,690 bytes** of PowerPoint-authored slide markup each of them
  differs from Office's own bytes in **exactly one contiguous region holding exactly the bytes that
  were set**. Nothing else in the deck moved.

The throwaway probe MJXOFF-278 was filed with had reported 243 shapes, 310 paragraphs, 240 runs and
9,855 characters over the same file. The difference is entirely **the three groups**: the probe
counted a group as one shape and stopped there, and the shipped walk descends into it, which adds 9
member shapes, 6 more text bodies, 13 paragraphs, 13 runs and 36 characters. The two numbers that had
to agree — 12 of 135 entries changed, over 377,690 bytes — agree exactly.

### What the gate does not tell you about an extension

**An `<ext>` carrying markup in a namespace the file declares `mc:Ignorable` reports nothing at all,
and that is the honest state rather than a pass.** `mc:Ignorable` markup is resolved before
validation, and resolution removes an ignorable element together with its content; `sml.xsd`'s and
`dml-chart.xsd`'s `CT_Extension` declare their whole content model as a bare
`<xsd:any processContents="lax"/>`, whose `minOccurs` defaults to 1, so the emptied `<ext>` used to
be rejected with *Missing child element(s)* on every conformant file Office has written since 2010.
MJXOFF-196 closed that: an extension slot resolution empties is now dropped along with the extension
it held, because such an element exists only to carry it.

What remains is a **residue, not a deviation**: the gate says nothing about the markup *inside* an
ignorable extension. It never could. `CT_Extension`'s wildcard is `processContents="lax"` and no
schema for such a namespace is loaded, so keeping the content would only have had the validator
accept it unread. `crates/mjx-schema-gate/src/wildcard_slots.rs` names the five elements this
applies to, derived from the pinned XSDs rather than listed by hand, and
`xtask/tests/mce_extension_seam.rs` holds all of it.

---

## 6 · What this pass cannot exercise, and why

Six checks are **blocked** — each is a question the pass wants answered and cannot yet ask. They are
not oversights, and they are listed here rather than left for you to discover one at a time at the
desk.

**One came off this list.** `V-PPTX-02.4` — the colour transforms against PowerPoint's eyedropper —
was the seventh, and the only entry in the pass with *no artefact at all*: `ColorSpec` carried a
colour's kind and value and no transform children, so no facade call could author a `comp`, `gray`,
`gamma` or `invGamma`, and no committed fixture has one. `MJXOFF-219` gave `ColorSpec` the whole of
`EG_ColorTransform`, and `v-pptx-02-authored.pptx` now opens with two rows of swatches to point the
eyedropper at. The corpus was never going to unblock it — PowerPoint's own interface exposes none of
the four, so a saved file is unlikely to contain one — which is why it needed a code change rather
than a file. It is R3, the third-highest risk item in the repository, and it is now the pass's to
answer rather than the pass's to skip.

| Check | Why it has no file | Does the corpus unblock it? |
|---|---|---|
| `V-PPTX-07.6` — is `None` right for a rotation-only transform? | `set_shape_transform` writes only the fields its argument names, so no facade call can author a transform naming a rotation and neither `a:off` nor `a:ext` | Only if a real deck happens to carry one. This is also a **design question** — see §7 |
| `V-PPTX-01.8` — a `+mj-sym` reference the theme does not define | `CharacterPropertiesSpec` has no font setter | Yes, given a deck whose theme leaves the symbol slot undefined |
| `V-PPTX-02.13` — `p:oleObj@spid` naming a `v:shape@id` | asserted only against markup we authored | Yes, given a deck with an OLE object and its VML backing |
| `V-PPTX-02.17` — a deck that arrived with a dangling `r:id` | `save` refuses to write one | Only from a third-party producer that wrote one. Not something Office does on purpose |
| `V-PPTX-08.12` — prefixes declared only at the slide root | our writer declares prefixes where it puts them | Yes, given almost any PowerPoint-written deck |
| `V-DOCX-06.5` — the `w:altChunk` import, performed by Word | `add_alt_chunk` is not projected onto the facade | Either the facade method or an original that has one |

Four more entries are about a **machine** rather than a file and cannot be done on the workstation
that generated the artefacts: `V-PPTX-02.7` (the schema suite on Windows and macOS) and
`V-DOCX-01.8` / `V-XLSX-06.5` (a wheel installed into a clean environment on each, and the npm package
in a real browser — not headless).

---

## 7 · The decisions this pass settles, rather than checks

Six checks have **no expected result**, because what they establish is a decision. None of them may
be recorded as `differs`: there is nothing to differ from. Record what happens, and then decide.

| Check | The question | What follows |
|---|---|---|
| `V-PPTX-01.1` | Does PowerPoint take the master's `p:bodyStyle` for a text box, or the previous hard default? | Yours. The change is one revertible commit — this is R1, and it is the first thing to look at |
| `V-PPTX-01.7` | Does PowerPoint honour `a:lstStyle > a:defPPr` at **every** level? | Yours. Either the prose or the renderer wins, in a resolver all three formats share |
| `V-PPTX-01.8` | Is answering `+mj-sym` verbatim the right report for a slot the theme does not define? | Yours |
| `V-PPTX-02.12` | Does PowerPoint drop a plain `p:contentPart`? | Already decided in advance: if it does, `add_ink` switches to the `mc:AlternateContent` form and the MCE skip list grows by one |
| `V-PPTX-07.6` | Is `None` the right answer for a rotation-only transform? | Yours — keep `None`, or resolve position and size from different tiers. **The check asks whether the documented non-goal is right**, which is a different question from whether the code matches it |
| `V-PPTX-08.7` | Does PowerPoint ignore a dangling `c:dPt`, or offer a repair? | Yours — drop dangling anchors on save, or keep reporting them |

One more decision belongs on this list and is not a check at all: **whether `Deck::blank` should ship
one slide.** It ships an empty presentation today, and every artefact this pass opens had a slide
added to it by the generator.

---

## 8 · One thing the programme escalated to this desk

It is not a validation entry. It was judged the user's call by the child that found it, and would
otherwise live only in a merged pull request. (There were two until MJXOFF-196 took the second one
back and fixed it; §5 records what the gate can and cannot say about an extension now.)

**`CellReference::{new, relative, absolute}` take `(column, row)`**, while thirty-odd methods across
`mjx-pptx`, `mjx-docx` and the facade take `(row, column)`. MJXOFF-118 wrote the reasoning onto the
type itself: those three do not index a body of cells, they construct **the address**, whose only
rendering is `A1` — column letters then row number — so taking the row first would make
`CellReference::relative(6, 1)` spell `B7`. The two idioms never meet at a call site, because nothing
on the `Workbook` facade takes a `CellReference` at all. It is defensible and it is still an
asymmetry in a shipped public API, which is why it is here rather than settled.

---

## 9 · Three defects nobody has fixed

Found by MJXOFF-122 and MJXOFF-128, owned by nobody, and deliberately **not** fixed inside a corpus
child. Each is confirmed against the code as it stands.

1. **`PatternFillSpec::solid` can write a ten-character `@rgb`.** It prefixes `FF` unconditionally, so
   a caller who passes an already-eight-digit ARGB gets `FFFFFF0000`. `sml.xsd` types `@rgb` as
   `ST_UnsignedIntHex`, an `xsd:hexBinary` of length 4, and rejects it —
   *`[facet 'length'] The value 'FFFFFF0000' has a length of '5'`* — measured against the pinned
   Transitional schema. Nothing in the workspace calls it that way, so **no gate has ever seen the
   shape**: the schema gate only validates markup some test authored, and the two checks that run on
   every package — `Package::validate` and the child-order audit — are about the graph and the
   sequence, not about an attribute's lexical space.
2. **`Document::effective_run_properties` raises on a numbering instance nothing defines**, and the
   shipped `crates/mjx-ooxml/examples/build_a_document.rs` writes exactly that shape —
   `attach_paragraph_to_list(4, 1, 0)` on a blank document, where no `word/numbering.xml` exists. The
   example's own comment acknowledges it. A caller following the example and then asking for
   effective properties gets an error from a document the example told them to build.
3. **`CellFormatSpec` has three different shapes across the three languages.** The Rust struct exposes
   twelve public fields; the Python class declares four of them as attributes and takes all twelve as
   constructor keywords; the WebAssembly class wraps the value opaquely behind methods. The binding
   rule is that a class wraps exactly one value and adds no behaviour, and three different shapes of
   the same record is the one place that rule is not visibly held.

---

## 10 · When the pass is done

**The version.** Every child of this programme moved the patch digit and nothing else. `v0.1` means
"complete **and validated against real Office**", and that claim rests on a person opening files in
Office — so the minor and major digits are yours to raise, during verification, and no agent has ever
raised one or written a tag.

**What "done" is.** Every check in the three format pages carries a result line, or carries `blocked`
with the reason. The corpus has at least the files §5's table names. The `Unreleased — 0.1.0` section
of `CHANGELOG.md` is a ledger of breaking changes, not a release, until you decide otherwise.

**What to do with a finding.** `00-method.md` §4: confirm it against the other variant of the same
area where one exists — `authored` and `edited` are different code paths and a difference in only one
of them is a far more useful report — then file it in Plane with the artefact name, the workspace
version, the Office version and build, and what was expected against what was rendered. Write the
issue id into the result line. A `differs` with no issue id is a note nobody will act on.
