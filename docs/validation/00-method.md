# The Office validation pass — method

Every fixture in this repository was written by this project or by LibreOffice. **No test here has
ever read a file Microsoft Office wrote, and no machine in this repository can answer the question
this pass exists for:** *does real Office render what we intended?*

Three gates already answer everything a machine can. The schema gate (`mjx-schema-gate`) holds every
part to the ECMA-376 XSDs. `Package::validate` holds the container to its OPC invariants. The
child-order audit walks every element of every part against the generated `xsd:sequence` tables. All
three run over every artefact this pass uses, before a person ever opens one — see
`xtask/tests/validation_harness.rs`. What none of them can see is *rendering*: text-style
inheritance, effective colour, table styles, effects, anchoring. LibreOffice diverges from Office in
exactly those areas, and it tolerated an empty `a:overrideClrMapping` and a lightRig-less
`a:scene3d` across 58 releases of this project. So LibreOffice is a **canary, not a verdict**: one
conversion proves a file opens, and proves nothing about what it looks like.

That leaves a person with Office in front of them. This page is how that person records what they
see.

## 1 · Entry ids

```
V-PPTX-03        an area entry — one rendering-risk theme, one row of the index
V-PPTX-03.2      a check inside it — one thing to look at, with one result
```

* `V-` marks a validation entry and nothing else in this repository uses the prefix.
* The middle token is the format: `PPTX`, `DOCX` or `XLSX`. It is the artefact's extension upper-cased,
  so an entry id names its own file type.
* The number is the area's **position within its format**, zero-padded to two digits. Positions are
  never reused: an area that is retired keeps its number and is struck through, because a result
  recorded against `V-XLSX-04` in one version has to still mean the same area in the next.
* A check appends `.` and its position within the area, not zero-padded. Checks may be added to an
  area without renumbering anything, which is the whole reason they are numbered separately.

Every area entry resolves to a generated artefact and every generated artefact is named by an area
entry. That is not a convention; `xtask/tests/validation_index.rs` fails the build when it stops
being true in either direction.

## 2 · Risk levels

The level says **how much of the answer is a judgement Office could disagree with**, not how
important the area is.

| Level | What it means |
|---|---|
| `high` | Inheritance, resolution or style layering — where a *reading* of the ECMA-376 prose, not a schema, decides the rendered answer. Two implementations can both be conformant and disagree. |
| `medium` | Modelled markup whose shape the schema pins down, but whose rendering still has choices in it: geometry, effects, anchoring, wrapping. |
| `low` | Markup with essentially one sensible rendering. Present so the pass covers the area at all, not because it is expected to surprise anyone. |

The level of every area is stated in two places — `docs/validation/01-index.md` and `AREAS` in
`xtask/src/validation/mod.rs` — and the index test compares them, so a level changed in one place and
not the other fails rather than drifting.

## 3 · Recording a result

Each check carries one result line, and it ships **unfilled**:

```
Result: — · — · — · —
        version · date · initials · verdict
```

Filled in, it reads:

```
Result: 0.0.128 · 2026-09-14 · JS · as intended
Result: 0.0.128 · 2026-09-14 · JS · differs — MJXOFF-000
```

* **version** — the workspace version the artefact was generated at, from `Cargo.toml`. A result is
  a statement about a build, not about the project.
* **date** — ISO-8601, the day Office was actually opened.
* **initials** — who looked. One person, not a team.
* **verdict** — exactly one of:

| Verdict | Means |
|---|---|
| `as intended` | Office renders what the check says it should. |
| `differs` | Office renders something else. Must carry the issue id it was filed as. |
| `blocked` | Could not be judged — Office refused the file, or the artefact was not produced. Must say which. |
| *(unfilled)* | Nobody has looked yet. **This is what every entry ships as.** |

Two rules about the vocabulary, and both are deliberate:

* **There is no `pass`.** No agent, no script and no LibreOffice conversion may write a verdict; a
  verdict means a person opened the file in Microsoft Office and looked. A word that reads like a
  test result would invite exactly the inference this page exists to forbid.
* **An unfilled result is not a failure.** It is the honest state of a check nobody has reached, and
  it stays that way until somebody does.

### The inherited table

`docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md` already carries 28 rows of exactly this kind, written by
MJXOFF-108, with its **Excel says** and **Verdict** columns deliberately empty. It is the detail
behind `V-XLSX-02` and it is carried into this pass **unchanged** — not re-marked, not re-worded, not
re-numbered. Those two columns are part of the same pass and are filled the same way.

## 4 · Filing an issue

Only after the check is `differs`, and only after §5.

1. Confirm the behaviour against a second artefact where one exists — the `authored` and `edited`
   variants of the same area are different code paths, and a difference in only one of them is a
   more useful report than a difference in "the library".
2. File in Plane, project **MJXOFF**. Title: `<entry id> — <what Office does>`. Body: the artefact
   file name, the workspace version, the Office version and build, and what was expected against what
   was rendered. Attach the artefact.
3. Write the issue id into the result line. A `differs` with no issue id is a note nobody will act
   on.

## 5 · A documented gap is never a validation failure

> Check the format's gaps page before filing anything.
> `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` keeps two lists apart on purpose: what the
> library **decides** not to do, each with its reason, and what is **built but not yet verified
> against Office**, each with the work that will verify it. **Only the second list is what this pass
> is for.**

The equivalent pages for the other two formats are
`crates/mjx-docx/docs/guide/fidelity_and_gaps.md` and
`crates/mjx-xlsx/docs/guide/deliberate_limitations.md`. A file that does not render a
feature this library deliberately preserves rather than models is behaving exactly as designed, and
an issue filed against it costs a reviewer the time this rule exists to save.

## 6 · Producing the artefacts

```sh
cargo run -p xtask -- validation-artefacts            # every area, every format
cargo run -p xtask -- validation-artefacts --list     # the catalogue, with ids and risk levels
cargo run -p xtask -- validation-artefacts --format xlsx --area 2
```

Artefacts land in `target/validation-artefacts/` (`--out` moves them). Two runs produce
**byte-identical** files; `xtask/tests/validation_harness.rs` asserts it, because without that the
three-language comparison below is meaningless and every future diff is noise.

Each area produces two artefacts, and they are different code paths:

* **`-authored`** — built from `Deck::blank`, `Document::blank` or `Workbook::blank`. Nothing is read
  from disk, so every byte is one this library wrote.
* **`-edited`** — built by *editing* an Office-authored original from `tests/office-authored/`.
  Authoring bugs and editing bugs are different bugs, and only this variant exercises edit isolation
  against markup we did not write.

The corpus is empty, so every edit variant currently **skips by name** — the run prints the area and
the exact path it looked for. `MJX_REQUIRE_OFFICE_CORPUS=1` turns any such skip into a hard failure,
the same arrangement `MJX_REQUIRE_SOFFICE=1` makes for the `office_open` canary, and it should be set
only once there are files.

Filling it is the other direction of the same command, and it is the half of this pass that changes
the repository:

```sh
cargo run -p xtask -- validation-artefacts --ingest <a file saved out of Office> --area 2
```

It reports which entry the file answers, whether it round-trips at the container and through the
facade, whether the package invariants hold, whether its child order matches our generated tables,
whether it validates, and where it would be committed — and it copies nothing, because committing a
file is a decision taken against the redistribution rule in `tests/office-authored/README.md`.
`docs/validation/06-the-office-pass.md` §5 is the whole loop.

The same artefacts are produced a second time by `bindings/mjx-python/tests/test_validation_artefacts.py`
and a third time by `bindings/mjx-wasm/tests/node/validation_artefacts.mjs`, each compared against
the Rust output **part by part, byte for byte**. A binding method wired to the wrong facade method
changes one payload and fails there — before any human opens anything.
