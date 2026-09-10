# Changelog

All notable changes to **mjx-ooxml-rs** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning

The project is pre-release and uses `v0.0.x`: the patch number is incremented each development
iteration until the first milestone. Milestones then advance the minor version:

- **`v0.1`** — PowerPoint (`.pptx`) complete
- **`v0.2`** — Word (`.docx`) complete
- **`v0.3`** — Excel (`.xlsx`) complete

Further milestones (rendering, bindings, …) are defined as that work is scheduled. The public API is
**not** stable until `v0.1`.

## [Unreleased — 0.1.0]

`v0.1` is where the public API stops being free to change. The milestone ships when the PowerPoint
slice is complete; until then the working versions stay `0.0.x` and this section accumulates every
break made on the way, so the migration note for `0.1.0` is written as the breaks happen rather than
reconstructed afterwards.

### Breaking changes

| Was | Is | Why |
|-----|----|-----|
| `Presentation::cell_span` → `(columns, rows)` | → `(rows, columns)` | `table_dimensions` answers `(rows, columns)`, `merged_cell_anchor` answers `(row, column)`, and every cell method takes `(row, column)`. Two same-typed `usize`s are read as a habit, not as a signature. |
| `mjx_dml::BlipFill`, `BlipFillMode` | `PictureFill`, `PictureFillMode` | `blip` is ECMA's abbreviation for "binary large image or picture" and nothing else's. This crate already expanded `a:buBlip` to `BulletPicture`. |
| `mjx_dml::Fill::Blip`, `FillSpec::Blip` | `Fill::Picture`, `FillSpec::Picture` | Same token, same expansion. Office's own name for it is "Picture fill". |
| `mjx_dml::StyleMatrixReference::idx` | `index` | An abbreviation named after the `@idx` attribute; the docs already called it an index. |
| `mjx_chart::DataLabelSpec::show_*` (7 fields) | `shows_*` | The struct a caller reads (`DataLabelSettings`) already said `shows_*`; the two differed by one letter. `TrendlineSpec` / `ChartTrendlineData` agree on all nine of theirs. |
| `mjx_chart::ErrorBarSpec::plus`, `minus` | `plus_values`, `minus_values` | Matches `ChartErrorBarData` and `ErrorBars::plus_values()`; `plus` alone did not say plus *what*. |
| `mjx_pptx::ChartErrorBarData::has_no_end_cap` | `no_end_cap` | Matches `ErrorBarSpec`, the struct that writes the same `c:noEndCap`. |
| `mjx_pptx::PptxError::PictureHasNoBlipFill` | `PictureHasNoImage` | Drops the token, and says what the caller can act on. |
| `mjx_pptx::Presentation::activex_binary_bytes` | `activex_state_bytes` | Reads exactly what `set_activex_state` writes; the pair named one artefact two ways. |
| `mjx_pptx::PptxError` was `#[non_exhaustive]` | it is not | A `#[non_exhaustive]` enum forces a wildcard arm on every downstream `match`, which is exactly what would let a new failure mode be silently filed under a catch-all. `mjx_ooxml::Error`'s classification is deliberately exhaustive: adding a variant now fails the build until someone decides which of the eleven `ErrorCode`s it belongs to. |
| `delete_chart_data_labels`, `Axis::is_deleted`, `DataLabels::delete_all`, `auto_title_deleted` (12 public identifiers) | `suppress_chart_data_labels`, `is_suppressed`, `suppress_all`, `auto_title_suppressed` | `delete_*` wrote a `c:delete` (*draw nothing here*) and sat beside `remove_*`, which removes the element (*say nothing here*). Two operations, two near-synonyms, no way to tell them apart from the method list. `delete` was the spec element's own name; a public identifier that needs the spec open to be read is the thing the convention forbids. The wire token is unchanged and still named in every item's docs. |
| `mjx_sml::SmlError::SheetDataTooLarge` | `PackedStoreTooLarge` | There are two packed stores in `mjx-sml` now — the cell store and the shared-string table — over one shared byte arena, and the variant either of them raises said "the cell store's byte space" in its message. A name and a message that are true of one of two callers is the kind of small lie that survives into a user's terminal. |
| Twelve `*_part_bytes` accessors → `Option<&[u8]>` | → `Option<Cow<'_, [u8]>>` (and `Document::alt_chunk_payload` → `(Cow<'_, [u8]>, &str)`) | A part that has been edited has no stored bytes, so a borrow could only be offered by answering `None` for it — which is how `from_package` came to report a main part missing the moment a caller edited it (MJXOFF-222). The `Cow` borrows whenever the part is not dirty, so the ordinary path still copies nothing. The facade and both bindings are unaffected: `mjx_ooxml` already owned its `Vec<u8>` at that boundary. |
| `mjx_docx::PageOrientation` (hand-written, MJXOFF-98) | `mjx_docx::PageOrientation` (re-export of `mjx_ooxml_types::wordprocessingml::PageOrientation`) | A duplicate of the generated enum, caught in MJXOFF-109's own pre-dispatch review — "consume, do not re-create" is the generator's whole reason to exist. `PageOrientation::to_wire(self) -> Option<&'static str>` (`None` for `Portrait`, the schema default) is **removed**: the generated type's own `to_wire(self) -> &'static str` always returns a token, and the "omit the attribute for `Portrait`" convenience now lives in `SectionProperties`'s writer (`crate::page::orientation_wire_value`, crate-private), not as a method on the value type. |
| `mjx_docx::TableStyleOverrideContent::TableProperties`/`TableRowProperties`/`TableCellProperties`, and the same three `StyleDefinitionContent` variants | inner type `Unmodeled` → `TableProperties`/`RowProperties`/`CellProperties` | These variants had no public accessor before MJXOFF-119 (a value of either enum was unreachable from outside the crate), so this is breaking only in the formal sense of a public enum's variant shape changing, never in practice. |
| `mjx_sml::ConditionalFormattingFormula` | `mjx_sml::FormulaElement` (module `mjx_sml::formula::element`) | MJXOFF-123. `sml.xsd` hangs three elements off `ST_Formula` — `cfRule/formula`, `dataValidation/formula1` and `dataValidation/formula2` — whose content model, escaping rules and no-evaluation contract are identical, so the type carries its own local name and there is one implementation rather than three. `new` gains a `local: &str` parameter for the same reason. The answer to a second consumer is one helper both can reach, not a copy with a different doc comment. |
| `mjx_docx::{RunPropertyContent, ParagraphMarkRunPropertyContent, ParagraphPropertyContent, StyleParagraphPropertyContent, SectionPropertyContent, NumberingPropertyContent}::Change`/`Inserted`/`Deleted`/`MovedFrom`/`MovedTo`, `FieldCharacterContent::NumberingChange` | inner type `Unmodeled` → the real revision type (`RunPropertiesChange`, `ParagraphMarkPropertiesChange`, `ParagraphPropertiesChange`, `TrackChangeMarker`, `SectionPropertiesChange`, `TrackChangeNumbering`) | MJXOFF-126. `ParagraphProperties::change()` already had a public accessor returning `Option<&Unmodeled>` — this one is a real, consumer-visible signature change, not only a formal one; every other listed variant had no accessor before this child, matching the row above. |
| `mjx_chart::EmbeddedWorkbook` (`new`, `Default`, `push_row`, `sheet_name`, `rows`, `for_chart_data`, `for_chart_space`, `to_package_bytes`), `mjx_chart::WorkbookCell` (`Blank`, `Number`, `Text`, `text`), `mjx_chart::CONTENT_TYPE_WORKBOOK_PACKAGE`, `mjx_chart::DEFAULT_SHEET_NAME` | **removed.** The two layout entry points become the free functions `mjx_chart::embedded_workbook_for_chart_data(&ChartData) -> Result<Vec<u8>, mjx_sml::SmlError>` and `mjx_chart::embedded_workbook_for_chart_space(&ChartSpace) -> Result<Vec<u8>, SmlError>`; the two constants become `mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE` and `mjx_sml::write::DEFAULT_SHEET_NAME`; the grid type has no replacement, because `mjx-chart` no longer holds a spreadsheet model | MJXOFF-99. `mjx-chart` carried a minimal SpreadsheetML writer because a chart embeds a real `.xlsx` and no SpreadsheetML crate existed — the workspace's one sanctioned duplicate, with a note in its own header naming this child as its executioner. `mjx-sml` (rank 2.1) now writes it and `mjx-chart` (2.2) reaches down to it; `mjx-chart → mjx-xlsx`, which the old note proposed, would have been an upward edge the layering forbids. What a chart's workbook *contains* did not change by a byte. |
| `mjx_pptx::PptxError` gains `Sml(mjx_sml::SmlError)` | — | The same removal: a chart's embedded workbook is now written by `mjx-sml`, so its failures reach a PresentationML caller as themselves rather than being flattened into `Opc`. `PptxError` is deliberately not `#[non_exhaustive]`, so this is a breaking addition; `mjx_ooxml::Error` classifies it through the same `sml_code` that `mjx-xlsx`'s errors go through, and no `ErrorCode` was added — nothing changes for either binding. |
| `mjx_chart::ChartLabelScope::Plot { plot_idx: usize }`, `Series { series_idx: usize }`, `Point { series_idx: usize, point_idx: u32 }` | `Plot { plot_index: u32 }`, `Series { series_index: u32 }`, `Point { series_index: u32, point_index: u32 }` | MJXOFF-118. These were the **last three public fields in the workspace spelled `*_idx`** — an abbreviation named after `c:idx`, which is exactly the case 0.0.69 already settled for `mjx_dml::StyleMatrixReference::idx`. The width goes with the name: this type crosses the facade to both bindings, and **both already published these three as `u32` and cast on the way in and out**, so the rename and the narrowing change nothing in Python or TypeScript and delete five casts (two of them `usize as u32`, which truncate rather than fail). |
| `mjx_pptx::ShapeInfo::index`, `mjx_pptx::LayoutInfo::index`, `mjx_pptx::LayoutInfo::master_index` — `usize` | `u32` | MJXOFF-118, finishing A9's own recorded loose end (*"better normalised once at v0.1"*). All three structs are re-exported **verbatim** by `mjx-ooxml` and by both bindings, which means they bypass `crates/mjx-ooxml/src/index.rs` — the one place the facade's `u32`/model `usize` width difference is meant to be crossed — and carried a host-dependent width into a foreign-function-facing type. Both bindings already read all three as `u32`; those casts are gone. A `mjx-pptx` caller feeding one of these back into a `Presentation` method converts once (`usize::try_from`), which `crates/mjx-pptx/src/index.rs` documents; a `mjx-ooxml` caller can now pass `ShapeInfo::index` straight to a `Deck` method, which was not possible before. |
| `mjx_dml::ColorSpec` — three variants, `#[derive(Eq)]` | a fourth variant `Transformed { base: Box<ColorSpec>, transforms: Vec<ColorTransform> }`; **no `Eq`** | MJXOFF-219. `ColorSpec` is what every authoring caller hands in, and it carried a colour's kind and value and **no transform children**, so nothing in this workspace could author a colour transform and `Color::spec()` silently dropped a producer's. The three existing variants and every construction site are untouched — the alternative shape (`ColorSpec { kind, value, transforms }`, the ticket's option 1) is faithful to the schema and rewrites 393 call sites; this one costs an arm in the seven places that `match` on the enum. `Eq` goes because a transform's value is a `Fraction`/`Angle` (both `f64`, both `PartialEq` only); nothing in the workspace required it, and every type that embeds a `ColorSpec` — `FillSpec`, `LineSpec`, `EffectListSpec`, `CharacterPropertiesSpec` — was already `PartialEq` alone. |

Nothing else in the public surface changed name or shape. The sweep read all 1,561 public
identifiers of the eleven merged PowerPoint children; everything else either already followed the
convention or is a spec-sourced proper noun (`Srgb`, `ScRgb`, `OleObject`, the preset-shape names
whose digits are part of their identity).

The one candidate the sweep declined to settle on its own — whether `delete_chart_data_labels`
should become `suppress_*`, given that `delete` is the spec element's own name and runs through a
dozen coherent `mjx-chart` identifiers — was decided in favour of the rename and taken in 0.0.69,
whole rather than in part: renaming only the `mjx-pptx` method would have traded one inconsistency
for another. It is the row above. A grep in CI now keeps the spelling from drifting back.

## [0.0.163] - 2026-09-10

### The two arms of the gate now look at the same markup (H19)

#### The child-order audit walked the raw tree while the schema arm walked the resolved one (MJXOFF-272)

`mjx-schema-gate` has two arms over every part. `inspect.rs` resolves markup compatibility first —
the winning `mc:Choice` selected, ignorable markup dropped — and validates *that*. `order.rs` did
not: it took `package.part_tree(&part)` and handed it straight to `child_order::audit_tree`, and the
generated tables name no `mc:AlternateContent` slot, so the walk stepped over every such element
without entering it.

What that costs depends entirely on **where the element sits**, and the two shapes are not equally
visible:

* as a root's **only** child, the walk recognises nothing, visits one element, and
  `MINIMUM_ELEMENTS_VISITED` fires. That is a red — a working alarm. `legacy_form_control.xlsx` was
  tripping it, and `mjx-xlsx`'s `ORDER_SWEEP_EXCLUSIONS` register carried the row;
* as one root child **among several**, the walk descends into the siblings and reports a count that
  looks exactly like a healthy one, while the whole `mc:` subtree goes unaudited. **No floor can see
  this**, because the number a floor reads is the number a healthy audit of the siblings produces.

The second is the one that mattered, and it is the shape this programme keeps finding: a surface
exercised at one point and reported as covered. Both are closed the same way — the ordering arm
resolves through `markup_compatibility_resolved_tree`, the one function that produces the view, so
the two arms cannot drift apart again. A part whose markup compatibility will not resolve is now a
reported defect of the ordering arm rather than a silent fall-back to the raw tree.

`crates/mjx-schema-gate/tests/ordering_under_markup_compatibility.rs` is what holds it, and neither
of its two cases asserts a total. It authors an `xdr:wsDr` with a plain `xdr:twoCellAnchor` beside an
`mc:AlternateContent` whose `mc:Fallback` holds a **copy of that anchor**, so the relation between
the two walks is a property of the markup rather than a quoted number: the resolved walk must visit
exactly twice the structure below the root that the raw walk does. The second case puts the
fallback's `xdr:clientData` ahead of its `xdr:from` and requires the audit to redden naming
`CT_TwoCellAnchor` — while the raw walk still reports the part clean and past the floor, which is the
silence being closed.

A third case covers the branch the fix *opened* rather than the one it closed: a part whose markup
compatibility will not resolve is reported by name, because falling back to the raw tree there would
put the arm straight back to auditing markup the schema arm never sees — silently, and for exactly
the parts most likely to be hiding something.

`ORDER_SWEEP_EXCLUSIONS` and `the_order_sweep_exclusion_is_still_necessary` are **deleted**. The
register asserted its own row was still needed, so fixing the defect turned it red and the row could
not outlive it; every committed `.xlsx` now takes every step of the sweep with nothing held back.

## [0.0.162] - 2026-09-10

### The last residues, and the one the sweep found on its way (H18)

Three named remainders, and a defect that only appeared because one of them was closed properly.

#### The program that writes the committed stub is now inside the type gate (MJXOFF-270)

`bindings/mjx-python/pyproject.toml` set `[tool.mypy] files = ["tests"]`, and CI runs
`python -m mypy --strict` with no path — so `tools/stub_docs.py`, the program that *generates* the
committed `.pyi` three languages read, was the one Python file in the binding nothing type-checked.
It had a real error in it: `ast.stmt.end_lineno` is `int | None` and `_docstring_span` returned it
as `int`. The scope now names both directories and says which is which, and CI's own invocation
covers 34 files rather than 33. `bindings/mjx-wasm` was checked for the same shape and has none:
every tracked `.mjs` under it is either matched by CI's `node --test` glob or imported by the
`guide_examples/` runner, which derives its list from the directory.

#### ECMA's published markup, swept for what it can and cannot settle (MJXOFF-250)

`xtask/tests/published_markup.rs` is the written list item 2 asked for, and every verdict in it that
names an artefact this project can read is *checked* rather than stated.

The one new derivation: every preset geometry ECMA publishes is a token
`mjx_ooxml_types::drawingml::PresetShapeType` round-trips, and the only token with no published
geometry is `upArrow` — registered, in both directions, so neither a new gap nor a healed one can
pass unremarked. (The artefact writes `<upDownArrow>` twice, so it publishes 186 distinct
geometries against the schema's 187 tokens.)

The more useful half is the **negatives**, because item 1's result — the standard's data confirming
a hand-maintained table exactly — invites the reflex that every artefact is an authority.
`presetCellStyles.xml`, in the same directory, disagrees with Annex G.2's `builtinId` table in three
places, and in all three the prose is right: `<heading1>` carries `builtinId="17"`, which
`<heading2>` also carries and which §18.8.7 forbids two styles from sharing; `<accent3>` carries no
`builtinId` at all; and `<normal builtinId="0">` holds the `Percent` stylesheet. Those three are a
register, held in both directions.

Item 3's list — every place in this workspace that resolved an ambiguity by following a spec
*cross-reference* rather than a spec *statement* — is in the same file's header, with the shape of
each link, because they are not equally strong. The weakest is not a cross-reference at all:
`mjx-sml`'s `x:start` / `x:end` take their meaning from WordprocessingML clauses that nothing in
Part 1 connects to §18.8. Neither published artefact writes either element even once, which is now
measured rather than assumed.

#### The roster sweep's own population list (MJXOFF-252)

`BasePopulation` in `xtask/tests/derived_rosters.rs` is hand-written, and H8 filed that against its
own gate. Deciding *what counts as an enumerable thing* is judgement and cannot be derived, so the
judgement is made — the candidates are worked through in a table, each with its verdict — and the
mechanical halves are checked: `BasePopulation::ALL` is held against the enum's own declaration, so
the array cannot silently lose a variant while the exhaustive matches still compile.

The committed fixture corpus, the child-order schema stems, the guide examples and the validation
area ids joined. The guide pages were tried and **withdrawn on the evidence**: that walk answers
with every `.md` in the repository, so it read `entry_points.rs`'s three landing pages as a roster.
A population that broad does not find rosters, it manufactures them.

Adding the rest found three sites:

* `crates/mjx-pptx/tests/schema_validity.rs` named the two fixtures carrying markup compatibility,
  under a comment reading *"both fixtures that carry markup compatibility"*. It finds them now.
* `crates/mjx-xlsx/tests/comments.rs`'s `every_producer_workbook_…` named three. It now finds every
  `.xlsx` carrying a comment part, which is what its name promises.
* `crates/mjx-xlsx/tests/schema_gate.rs` edited two named fixtures. It edits the whole corpus.

#### The child-order audit and the schema arm were reading different markup (MJXOFF-272)

Widening that last one surfaced it. `mjx-schema-gate`'s schema arm resolves markup compatibility
before validating — `inspect.rs`'s header says so — and its child-order arm does not: `order.rs`
hands `part_tree` straight to the walk. So a part whose only root child is an `mc:AlternateContent`
is audited over nothing, and its own vacuity guard fires. One committed fixture does it,
`legacy_form_control.xlsx`, whose `xdr:wsDr` LibreOffice wrapped entirely in an `mc:Choice`. Nothing
this library writes is out of order and the schema arm validates the part cleanly.

The ticket is open; the interim state is one register row that must *still be needed*, so fixing it
turns the register red rather than leaving a row behind.

## [0.0.161] - 2026-09-10

### The small residues, drained (H17)

Seven work items filed by the units that could have hidden them. Two of them turned out to be about
something other than what they said, and the correction is the finding.

#### The guide's `python` and `js` blocks are documents too (MJXOFF-256, MJXOFF-263)

`xtask/tests/doc_gate.rs` dropped every fenced block before it looked for a claim, on a reason that
is right for a Rust doctest and was never right for anything else: a `python` or `js` block is
*run* by its binding's harness, which exercises its calls and says nothing about the paths its
comments name. A fence is now dropped exactly when rustdoc compiles it.

The corpus also grows two languages. `.py` and `.mjs` files carry comments; those comments are
documents; every half of every guide example opens with a header naming its guide page, its Rust
sibling and its harness **by path**, and none of it resolved against anything. Since a committed
block is a byte-copy of a region of one of those files, checking the file checks the block.
`.pyi` stays out: since MJXOFF-234 its docstrings are generated from `///` comments this gate
already reads as Rust.

Found on its first run: `bindings/mjx-python/tests/test_build_a_deck.py` named the JavaScript
walkthrough as `tests/node/build_a_deck.mjs`, which resolves under `bindings/mjx-python/`.

`xtask/tests/guide_examples.rs` gains the rule H12 suggested, stated over all three languages
rather than over "not Rust": every block the facade guide shows in Rust, Python or JavaScript is a
copy of a file a runner executes. It holds today with no edits; what it stops is the twentieth block
written straight into a page.

#### An example declares the packages it offers, and both are compared (MJXOFF-262, MJXOFF-260)

Two facts about a guide example were inferred, and both inferences were wrong in a way nothing could
see. *Whether* it produces a package was decided by looking for a binding named `saved`, so an
example whose binding had been deleted was indistinguishable from one of the seven that genuinely
produce none — and more than a third of the corpus took a skip path nobody read. *How many* it
offers was one, and two examples author two, so the second package's bytes were compared by nothing
and the choice of which to offer was explained in prose.

Both are one statement now: every Rust half carries a `guide-example:packages` line naming the
bindings it offers, or the word `none`, and the gate holds all three halves to the statement rather
than to each other. `the_same_chart_on_all_three` offers the deck and the Word document;
`one_authoring_vocabulary` offers the workbook and the deck. The Node harness has no skip left in it
at all.

#### `docProps/app.xml` is not provenance, and two fixtures prove it (MJXOFF-249)

The ticket names `comments_third_party.xlsx` as the only fixture claiming Microsoft authorship.
`charts.pptx` is a second: it carries `Microsoft Macintosh PowerPoint` and is python-pptx's template
deck, which its own `docProps/core.xml` says in words. **The trap had already sprung** —
`crates/mjx-pptx/tests/charts.rs` stated in a live doc comment that that deck was written by
PowerPoint, and `crates/mjx-ooxml/tests/preservation/deck_cases.rs` drew a conclusion about what
PowerPoint writes from its markup. Both are corrected.

`xtask/tests/fixture_provenance.rs` asserts a negative against a ledger and classifies nothing: no
fixture's `Application` may name Microsoft unless a person has written down how the file reached
this repository. A gate that read the element and decided who wrote a file would build the inference
the rule forbids and would have been wrong about both rows on its first run.

#### The preset table style families come off ECMA's own markup (MJXOFF-250)

`BuiltInTableStyleFamily`'s six prefixes and six bounds are compared against
`presetTableStyles.xml`'s 144 published names in three directions — every name parses and writes
back unchanged, every bound is the largest number its family reaches with no gap from 1, and one
past a bound is refused and absent from the artefact. A seventh family variant fails to compile
against the exhaustive match that enumerates them.

#### A VML wrapper's child order, measured rather than assumed (MJXOFF-264)

The ticket's premise — that per-child validation is "blind to sequence by construction" — is false.
Handing a child to `xmllint` as a standalone document applies its content model rather than removing
it, and a `v:shapetype` that writes `o:complex` before its shape elements fails validation today.
The order of everything *inside* a wrapper's children has been audited since MJXOFF-245. What is
left is the order of the wrapper's own children, and `<xml>` is a Microsoft convention that no
schema in either pinned tree declares — it has no content model to be out of. Both halves are
checked in `crates/mjx-schema-gate/tests/wrapper_child_order.rs`; the five pages that cited this
ticket as an open gap now say what was measured.

#### `SURFACES` is held to the facade handle population (MJXOFF-252, item 1)

The known instance of the roster shape `derived_rosters.rs` cannot see, closed the way the ticket
preferred: derived in place, through `xtask::facade_surface`, which both test binaries call rather
than walking `crates/mjx-ooxml/src` twice. The ticket's second item — a `BasePopulation` for the
enumerable things nobody has named — stays open.

### Left open, with the measurement

**MJXOFF-242** (`Package::part_bytes`). Both of its candidate closures are sized very differently
from what it believes. It records fifty-three sites converted and "the only callers left are
`mjx-opc`'s own round-trip suites, five `#[cfg(test)]` assertions and three guide pages"; the
measurement today is **314 call sites in 83 files** outside `mjx-opc`, of which **10** are under a
`src/` directory and four of those are `#[cfg(test)]`. A CI grep would need an exemption list of
three hundred rows; the rename is a breaking change across the same three hundred and belongs in the
`0.1.0` table. What *is* narrow is the misreading itself — **15** sites combine `part_bytes` with
`is_none()`/`is_some()`, every one of them in a test — but the shape that would actually reintroduce
the defect is a library site branching on `None`, and no lexical rule separates that from a site
asking for the bytes. Recorded on the ticket rather than closed by a rule that would not catch it.

## [0.0.160] - 2026-09-10

### Two gaps that were classes rather than instances (MJXOFF-265, MJXOFF-266, H16)

#### The four serialization ledgers ask what a pair loses; none asked what it moves (MJXOFF-265)

`crates/mjx-dml`, `crates/mjx-sml`, `crates/mjx-docx` and `xtask/tests/upper_markup_ledger.rs` hold
every hand-written `FromXml`/`ToXml` pair in the workspace to an idiom, and every idiom is a
*conservation* claim. Child order is outside all four by construction, which is how MJXOFF-251
shipped: six `mjx-docx` types re-ordered a child they never dropped, and all four stayed green.
H13 fixed the six and said in as many words that its search for a second instance was *"a spot check,
not a census"*.

`xtask/tests/child_order_census.rs` is the census. It scans **every function in every workspace
member's `src/`** rather than impl bodies — MJXOFF-251's defect lived in two free functions both
halves called, which is exactly what a ledger of impls cannot see — and looks for the two shapes a
child sequence loses its order by: a `Vec` both pushed into and extended from (MJXOFF-251's writer),
and an `Option` set from inside a loop over a child sequence (MJXOFF-251's reader). Both detectors
are deliberately over-inclusive; every site either finds is on a ledger with a written reason, and a
site that really moves a child also names the markup that proves the position travels.

**The answer to the ticket's question: besides MJXOFF-251's six, nothing.** Two further sites move a
child for reasons of their own — `mjx-sml`'s packed cell store, which remembers the payload's
position as two byte spans, and `mjx-mce`'s alternate-content resolution, which is a read-only
projection nothing is written back from — one builds fresh markup from owned arguments, and two are
the over-inclusion. The counts are printed by the test rather than written down anywhere.

The derive reads the bulk of the workspace and is the census's one deferral, so
`crates/mjx-derive/tests/derive.rs` gained the cases that make it a claim: two typed children
presented in the reverse of the order the type declares, a foreign child *between* two typed ones,
and a pretty-printed container — **indentation is made of text nodes and a text node is a child**,
which is the half MJXOFF-251's own report did not reach.

#### The two bindings' doc comments were written independently (MJXOFF-266)

Since MJXOFF-234 the `.pyi`'s docstrings are generated from the PyO3 crate's `///` comments and the
`.d.ts`'s from the wasm crate's, so neither can drift from its own source — and nothing held the two
sources to each other. `xtask/tests/binding_doc_parity.rs` now does.

Equality could never have been the gate: a sentence naming a sibling spells it `chart_series` in
Python and `chartSeries` in TypeScript, `str` against `string`, `None` against `undefined`. So the
comparison normalises first — identifiers inside backtick spans written `snake_case`, a closed table
of language words, articles dropped — and two members are a pair only when they take the **same
number of arguments**, which is what disposes of the trap the ticket warned about (`BorderEdgeSpec.new`
takes a style and a colour in Python and nothing at all in JavaScript, and pairing them by name
would compare two different constructors).

**Building it found twenty-three members where a TypeScript reader was told strictly less than a
Python one**, and they were fixed rather than recorded: six `CellFormatSpec.applies*` getters that
never said `undefined` writes no attribute, `Document.open` with no error paragraph at all where
`Workbook.open` beside it had one, `FontProperties.scheme` documented as the single word `scheme`,
and the schema attributes (`w:vanish`, `w:jc`, `w:outlineLvl`, `w:val="auto"`, `ST_HpsMeasure`) the
Word effective-properties getters name in Python and named nowhere in TypeScript. What remains is a
ledger with a reason per row, held to the measurement in both directions.

One row on it is not a prose difference at all: `ChartWrap.kind` returns `"top_and_bottom"` in Python
and `"topAndBottom"` in JavaScript, which contradicts the wasm binding's own written rule that data
tokens stay `snake_case`. MJXOFF-268 owns it.

## [0.0.159] - 2026-09-09

### Three places the API was not truthful about itself (MJXOFF-241, MJXOFF-248, MJXOFF-238, H15)

Each is small; together they are one theme — the library saying something about its own state that
is not so.

#### A refusal that named a part that was there (MJXOFF-241)

`Workbook::add_comment` aimed at a chartsheet or a dialogsheet answered
`MissingWorkbookPart("sheet 1")` — *"workbook part sheet 1 is missing from the package"* — about a
part that is present, correct and exactly what its `x:sheet` entry says it is. So did every other
edit that needs `x:worksheet` markup: the message came out of whichever helper reached for it first,
in eight places across seven modules, because `Workbook::worksheet_markup` answers one `Ok(None)`
for two different facts.

* `XlsxError::SheetIsNotAWorksheet { index, kind }` says what the tab is instead, with a phrase for
  each of the four shapes — a chartsheet, a dialogsheet, a part typed as a worksheet whose root is
  not `x:worksheet`, and a tab reaching a part of no sheet content type at all.
* `Workbook::require_worksheet_markup` is the one place that decides between the two answers. A tab
  reaching no part at all is still `MissingWorkbookPart`, which is what that is.
* It lands on the existing `mjx_ooxml::ErrorCode::WrongKind`, the code
  `PptxError::PartIsNotVmlDrawing` already answers, carrying the tab index. **The vocabulary does
  not grow and neither binding changes.** MJXOFF-213 kept the message byte-identical when it fixed
  the *ordering* of that refusal precisely because a variant reaches this classification.
* The facade's *cell* surface keeps its own `NothingToRead`: `worksheet_or_refuse` never claimed a
  missing part and reads that tab as "there are no cells there", the same answer a read gets. That
  is a decision, and a test now pins it as one.

#### A claim about the package that was only sometimes true (MJXOFF-248)

`Presentation::set_inline_table_style` promised *"no shared part, relationship or referenced GUID is
involved"*. Since MJXOFF-232 that stopped being a statement about the package for a table
`add_table` made — that call authors a `tableStyles.xml` for the emphasis flags it turns on.

The branch is **not** dead, and a test says so rather than a reader having to work it out: a deck
that *arrives* with a table and no shared part is the route, and every clause of the sentence holds
against it. The doc comment now separates what *this call* adds from what the package holds, and
names the two tests that hold the halves apart. The garbage-collection option is not taken, for the
reason the ticket records — it would delete a part a caller may be about to point another table at.

### Fixed

- **`Package::validate` could not see a reference our own edit broke (MJXOFF-238).** Its
  markup-reference check walks `authored_xml_parts`, which answers *"was this markup ours?"* and not
  *"did our edit break this markup?"*. The two come apart for one shape: `remove_relationship` on a
  part whose body is never touched. `check_relationships` finds no missing target — there is no
  relationship left to have one — the markup check skips the part, and `save()` wrote out a file
  naming a relationship nothing declares.

  The scope is widened by **exactly one set and only for one question**: a part whose `.rels` this
  library edited is checked too, and only for the ids it removed
  (`Package::unwired_relationships`). Checking such a part *whole* would have been the simpler
  widening and would have regressed the promise the scoping exists for — a file that opened and
  saved a moment ago would stop saving because a relationship it never named was dropped somewhere
  else in the package. `a_dangling_reference_the_file_arrived_with_survives_a_removal_it_has_nothing_to_do_with`
  is the case that separates the two, and MJXOFF-212's own bound
  (`a_part_that_holds_the_removed_relationship_but_names_it_nowhere_is_left_alone`) is held in the
  same file.

  **It was not only latent.** The ticket recorded the shape as unreachable through any shipped
  method, and that holds — but this repository's own suites constructed it three times and asserted
  the broken save was clean: two cases in `crates/mjx-opc/tests/edit_surface.rs` dropped the
  presentation's relationship to a slide while `p:sldId r:id` still named it, and
  `dropping_the_printer_settings_relationship_is_caught_by_the_packaging_check` had to write the
  broken container out and reopen it to launder the worksheet's provenance before the check could
  reach the markup at all. All three now assert the refusal; where the rest of the case still needs
  the bytes, it writes them with `save_unchecked`, which is what exists for a package a caller knows
  to be inconsistent.

  `validate`'s doc comment says what it now parses, and the module documentation says why the second
  scope is one question rather than a whole part. `Package::is_checkable_xml_part` is the one
  spelling of "a part whose markup can be walked", shared by both scopes — two would be two things
  to keep in step, and `XML_CONTENT_TYPES_WITHOUT_SUFFIX` has already cost this project one silently
  empty scope.

## [0.0.158] - 2026-09-09

### The stub stopped writing its own prose (MJXOFF-234, H14)

`bindings/mjx-python/python/mjx_ooxml/__init__.pyi` is a committed contract and
`bindings/mjx-python/tests/test_stub_parity.py` holds it to the compiled module in both directions —
over **names**. Beside each name sat a sentence, hand-copied from the `///` comment it restates, and
nothing compared those. MJXOFF-226 found the two disagreeing about the same item and differently
wrong in each, which is what two hand-maintained copies of one fact always eventually do.

**Generated, not compared, and the measurement is what decided it.** The two were never independent
prose: PyO3 compiles each `///` doc comment verbatim into the member's `__doc__`, so the compiled
module already carries the Rust sentence and the stub can be written from it with no parser in
between. Before any edit, **1,861 of the 1,924 docstrings the stub shared with the Rust were already
that comment's first paragraph, character for character** — so generation changes about three per
cent of the file rather than overwriting editorial work, which is the empirical question the ticket
said to answer before choosing. The ticket's own argument for comparison — *"equality would fail on
every one of 1,637 members"* — was false in the direction that mattered.

* `bindings/mjx-python/tools/stub_docs.py` restates the stub's docstrings from the module;
  `--check` writes nothing and reports. Committed output, never a `build.rs`.
* `bindings/mjx-python/tests/test_stub_docs.py` is the drift check over **1,937 governed
  docstrings** — 300 classes and 1,637 members, the ticket's figure exactly — with the floor phrased
  as *the scanner has stopped matching* and each half of the walk floored separately, because a scan
  that matches nothing rewrites the file to itself and passes.
* The stub's header says so, and editing a docstring there is now a test failure.

### Fixed

- **Six doc comments described `regenerate_chart_workbook` and were attached to
  `refresh_chart_workbook`** — three per binding, so `help(mjx_ooxml.Deck.refresh_chart_workbook)`
  and `mjx_ooxml.d.ts` both said the call "rewrites the embedded workbook so its cells hold exactly
  what the chart now draws". The facade says the opposite of the important half: it writes *the
  cells its own `c:f` formulas name, and nothing else*, and every other sheet, format and name the
  workbook carried survives. **The committed `.pyi` had it right and the Rust comment beside it had
  it wrong**, which is the drift class above caught in the act.
- **Eight `///` comments in the bindings were prose about Rust.** Five carried a rustdoc intra-doc
  link — `[`save_unchecked`](Deck::save_unchecked)` renders as broken markup in `help()` and in a
  `.d.ts` — and three said "see this module's own doc comment", naming a module the reader of the
  projected surface cannot open. The binding's `///` **is** the Python docstring, so these were
  already wrong for their real audience before the stub was generated from them.
- **`OoxmlError` and `IndexOutOfRangeError` said less at run time than the stub said about them.**
  The stub's fuller wording is now the C string literal in `bindings/mjx-python/src/errors.rs`, so
  `help()` gains it too. The other ten exception classes already agreed.
- **CI's `rustdoc` job was red on `epic/phase-h`** (MJXOFF-267). `crates/mjx-schema-gate` names
  `crate::harness` in an intra-doc link, and the crate has both a `harness` module and a `harness`
  function, so `cargo doc --workspace` under CI's `RUSTDOCFLAGS` refused to document it. Introduced
  by the previous unit, and invisible to `fmt`, `clippy` and `test --workspace` — which is the part
  worth carrying: broken links are denied by rustdoc alone, so the two `cargo doc` runs belong in
  every gate set, not only in the units that touch documentation.

### Recorded, not fixed

- **The two bindings' `///` comments are written independently**, so a TypeScript reader and a
  Python reader can be told different things about the same method — the second half of MJXOFF-234,
  which it raised and did not resolve. Of the 1,610 members both bindings document under the same
  class and Rust name, **1,440 carry the same sentence character for character** and 170 differ; a
  large part of the 170 is forced (`chart_series` against `chartSeries`, `str` against `string`), so
  equality cannot be the gate and the instrument needs designing. Filed as **MJXOFF-266**.

## [0.0.157] - 2026-09-09

### A named child comes back where the file put it, and a `.vml` part is validated child by child (MJXOFF-251, MJXOFF-245, H13)

Two holes in things that looked like they were watching. One silently rewrote a user's file; the
other meant a whole crate's markup was validated by nothing.

#### `w:customXmlPr`, `w:smartTagPr` and `w:docPart` keep their position (MJXOFF-251)

`crates/mjx-docx/src/document/structured_content.rs` names one child of six elements and passes the
rest through — the four `CustomXml*` wrappers and `SmartTagRun` share a worker, `Placeholder` had
the shape written inline. The reader took the first name match **wherever it sat** and the writer put
it back **first**, so `<w:customXml><w:p/><w:customXmlPr/></w:customXml>` came back with its two
children swapped. The function was called `split_leading_properties` and its doc comment said *"if
present as the first child"*; the code checked no such thing.

`wml.xsd` does put these children first, so the ticket recorded this as affecting non-conforming
files only. **It affects conforming ones too, and that is what settled the design.** Indentation is
made of text nodes and a text node is a child, so a pretty-printed wrapper whose `w:customXmlPr` was
the first *element* was still not the first *node*: hoisting it to index 0 stepped it over the
producer's own newline. A file whose element order was exactly what the schema asks for came back
with different bytes.

So the fix is positional rather than a rule about what counts as leading: the index the named child
sat at is read with it and restored on write, through one shared `split_positioned_child` /
`join_positioned_child` pair all six now use. Nothing is normalised, `the_round_trip_contract.md`
gains no exception, and the accessors keep working for a file that put its properties second — the
alternative (take it only when it is literally `children[0]`) would have byte-preserved just as well
while reporting `properties() == None` for every pretty-printed conforming wrapper.

#### A `.vml` part is validated, one child of its wrapper at a time (MJXOFF-245)

`crates/mjx-schema-gate/src/categories.rs` skipped every `.vml` part for two stated reasons, and one
of them had been false since MJXOFF-134: `vml-main.xsd` compiles perfectly well through the driver
schema the harness builds for **every** schema on one code path. Measured again here — without a
driver it is *WXS schema … failed to compile*; through one, the only error left is
*Element 'xml': No matching global declaration available for the validation root*, which is reached
only after compilation succeeds.

That residual obstacle is real but narrow: the wrapper root is a bare `<xml>` in no namespace, while
`v:shape`, `v:shapetype`, `o:shapelayout`, `x:ClientData` and the rest are global elements, and
`vml-main.xsd` imports its four sibling schemas. So a `.vml` part is now category **1b** — a
`WrapperRoot`, validated child by child — and `mjx-vml`'s authored markup meets a schema for the
first time. Every `.vml` part in the committed corpus gets a verdict instead of a skip line, and a
`v:shape` carrying an attribute VML does not admit is reported invalid naming the part, the child and
the attribute.

Two things follow. The category-2 allowlist is keyed on a namespace again and **only** on a
namespace: its VML entry matched the *absence* of one, so a worksheet that lost its `xmlns` was
reported as "a VML drawing part" and skipped — a false green `mjx-xlsx` had to carry a path rule to
close from the outside, and which is now a hard failure at the part. And MJXOFF-196's fifth wildcard
slot, `{o}equationxml`, can fire: it is declared in a VML schema and no VML part had ever been
resolved, so the slot had sat unreachable since the day it was derived.

What is still unchecked is **child order** — no `vml-*` schema is in the child-order generator's
`CHILD_ORDER_SCHEMAS`, so an authored drawing's sequence meets nothing. MJXOFF-264 owns that, and the
five places that state VML's guarantee now say so by number rather than repeating a reason that had
gone stale.

## [0.0.156] - 2026-09-09

### The guide is finished: every block in three languages, or Rust-only with a reason a test checks (MJXOFF-254, MJXOFF-261, MJXOFF-257, H12)

Nine of the facade guide's code blocks were three real files a runner executes; ten were still prose
that looks like code. All ten are done, and the page set is now closed: **every fenced code block
under `crates/mjx-ooxml/docs/guide/` is either shown in Rust, Python and JavaScript or marked
Rust-only with a stated reason**, and the only unmarked fence left in the set is the `sh` one
listing three walkthrough commands.

#### A block with a Rust half and no binding half has a spelling, and it is a claim rather than a suppression

MJXOFF-261 and MJXOFF-257 filed the same hole twice: a little of this facade is Rust-only by
decision, and the only way to say so was to write no marker at all — indistinguishable from having
forgotten, which is exactly what the gate exists to catch. The spelling is a fourth marker form that
names the Rust symbols making the claim true — `<!-- guide-example: the_escape_hatches rust-only
workbook_mut -->` — and it renders as one Rust block. What it changes is what `xtask/tests/guide_examples.rs` then demands,
and the demand is **stronger** rather than weaker: the example must have a Rust half and **no**
binding half and be shown by no marker in either language; every declared name must occur in the
region the block shows, so the reason is about *this block* rather than about the language; and
every declared name must be reachable from **neither** binding, read out of the committed `.pyi` and
the committed `#[wasm_bindgen]` declarations. The day a binding projects one of them, the claim
reddens. `rust-only` with no names is refused where it is parsed.

Reading the two binding surfaces moves out of `xtask/tests/binding_projection.rs` into
`xtask/src/binding_surface.rs`, so both gates reach one implementation: two parsers of the same two
surfaces would disagree with no way to say which was wrong. Its wasm type scan derives from
`#[wasm_bindgen] impl` targets rather than from `pub struct` lines, because most of that crate's
classes are declared by a macro and a `pub struct` walk sees sixteen of a hundred and eighty-seven.

#### Two blocks are Rust-only. The third was not, and that was checked rather than assumed

The backlog named three. `the_escape_hatches` and `downcasting_to_the_typed_cause` are genuinely
Rust-only — `presentation_mut`/`document_mut`/`workbook_mut`, `PptxError` and `downcast_ref` are
declared by neither binding.

**`the_round_trip_contract` was not.** What made it look Rust-only was its own choice of surface: it
edited a chart on a deck and compared the two packages through `mjx_opc::Package` — one layer below
the facade the page is about, and the layer the guide seals deliberately. `Workbook::part_names` and
`part_bytes` are the only general part door on this facade and the only one present in all three
languages, so the project's central claim is now stated in all three rather than exempted from two,
and a facade guide no longer opens the sealed package in one of its blocks.

#### Where the three differ in shape, the page says so — once, in one form

Six of the ten are expressible everywhere but do not *read* the same, because an `ErrorCode`, an
`ErrorDetail`, a `CellInput` and a `Format` accessor each project differently. Three blocks that
differ structurally with no sentence explaining why read as a typo, so the guide now has one device:
a line beginning **"The three differ in shape here"**, always *above* the blocks — a note underneath
arrives after the reader has already concluded one of them is wrong — and a table on the guide index
collecting the differences in one place.

#### The last block that never ran

`README.md` § *Bytes in, bytes out* was the guide's only `no_run` doctest: it read `in.pptx`, a file
that does not exist. The hidden prelude fixes it, and every code block a reader sees in this guide
is now executed by something.

## [0.0.155] - 2026-09-09

### Eight more guide examples in three languages each, and the one that finally exercises preservation (MJXOFF-254, H11)

0.0.154 built the mechanism and carried one example through it. This one drains the mechanical part
of its backlog: **eight** guide blocks stop being prose that looks like code and become three real
files a runner executes — `the_round_trip`, `authoring_from_nothing`, `addressing_a_deck`,
`addressing_a_document`, `the_calls_that_take_the_column_first`, `preserved_rather_than_modelled`,
`the_same_chart_on_all_three` and `one_authoring_vocabulary`.

`the_round_trip` was taken first, and not by alphabet. Every example the mechanism carried until now
authored every part it compared, so the whole arrangement said **nothing** about copy-on-write or
verbatim re-emission — the contract this library exists for. `the_round_trip` opens
`tests/fixtures/sample.xlsx`, which this project did not write, and each of its three halves asserts
preservation *inside the block a reader sees*: the same part names before and after, and
byte-identical payloads for every one of them. That placement is the point. Three languages agreeing
with each other cannot establish preservation; each half is checked against the input file instead,
and the harness comparison on top is a second, different fact — that all three preserved it the same
way. One `rename_sheet` inserted into the Python half fails the example's own assertion with
`/xl/workbook.xml changed` before the harness comparison is reached.

### The extractor grows a hidden Rust prelude

Reading a file is the caller's job — every guide page says so, and `build_a_deck.rs` keeps its own
`std::fs::read` outside the code the guide shows. Python and JavaScript get that for free, because
whatever they do above their sentinel is not in the block; a Rust half cannot, because its block is
*also* a compiled doctest and one that names `original` without binding it does not compile. So a
Rust half may carry an earlier region emitted as rustdoc's `#` lines: compiled, run, never shown.
`a_prelude_is_a_rust_only_device_and_every_one_of_them_extracts` reports one in another language
rather than ignoring it, because there it would be a no-op that reads like a feature.

### Two of the ten turned out not to be mechanical, and were not forced

The backlog called all ten name-for-name projections. Two are not, and neither can have a binding
half at all:

* **`the_escape_hatches`** — `Deck::presentation_mut`, `Document::document_mut` and
  `Workbook::workbook_mut` are Rust-only *by decision*, which is the whole point of the page the
  block sits on. Neither binding exposes one, and `bindings/mjx-wasm/src/deck.rs` says so in a
  comment.
* **`the_round_trip_contract`** — the block compares a deck edit through `mjx_opc::Package`, and the
  package is sealed at this facade deliberately. A `Deck` has no general part door in any language,
  which `fidelity_and_gaps.md`'s own gaps table already states.

Both move to the unit that decides how the guide *says* a shape differs, and both need something the
mechanism does not have: a spelling for a block with a Rust half and no other.

## [0.0.154] - 2026-09-09

### The guide holds markers, not code: one example in Rust, Python and JavaScript, kept equal by copying (MJXOFF-254, H10)

Phase H §4 asks for every example in every language the API ships in, and its own warning is why this
child built a mechanism rather than blocks: *three code blocks that drift apart are worse than one,
because two of them become confidently wrong.* A transcription is held together by whoever last
remembered to update all three, and Phase G's lesson is that prose is not checked — a code block in a
`.md` is prose that looks like code.

So a guide example is now **three real files a real toolchain runs**, each carrying a
`guide-example:start` / `guide-example:end` region:

| Language | File | Runner |
|---|---|---|
| Rust | `crates/mjx-ooxml/examples/guide_<name>.rs` | `cargo run --example`, and the same region again as a doctest under `cargo test --doc -p mjx-ooxml` |
| Python | `bindings/mjx-python/tests/guide_examples/<name>.py` | `pytest`, through `bindings/mjx-python/tests/test_guide_examples.py` |
| JavaScript | `bindings/mjx-wasm/tests/node/guide_examples/<name>.mjs` | `node --test`, through `bindings/mjx-wasm/tests/node/guide_examples.mjs` |

**`cargo run -p xtask -- guide-examples`** copies each region verbatim into the block that marks it;
`--check` writes nothing and reports whether the committed blocks are current. The output is
committed, never a `build.rs` — the rule `CLAUDE.md` already states for `mjx-ooxml-types`. A block a
reader sees cannot differ from a file a harness ran, *because it is a copy of one*.

Generation was considered and rejected. Emitting Python from Rust needs a model of the binding
projection, and the projection is not mechanical — a `CellInput` becomes a `CellWrite` constructor,
an `ErrorCode` becomes a string on `.code`, a range argument becomes two numbers, a `Format` accessor
becomes a free function. `xtask/tests/facade_curation.rs`'s own verdict applies: a translator that is
subtly wrong in that layer is worse than none. Copying has no model to be wrong about.

**`xtask/tests/guide_examples.rs` is the gate.** It holds four populations equal in both directions —
the markers in every `.md` in the repository, and the three source directories, all derived from the
filesystem rather than listed — and it also checks that every committed block byte-equals its
source's region today, that every half really has a region, that the three halves agree about whether
the example produces a package, and that each binding harness runs the Rust example and reads both
packages through that binding's one shared payload reader. Neither harness may name an individual
example: the population comes from the directory. Five mutations were run and each reddens a named
test; the verbatim output is in the pull request.

It inherits `walkthrough_triples.rs`'s limit **exactly, and says so in its module comment**: it
cannot verify that the two payload maps a harness reads are then asserted equal, because that is an
assertion in a language `xtask` cannot execute. What establishes that is the same discipline —
`SlideSize::widescreen` swapped for `SlideSize::standard` in one language at a time reddens that
binding's comparison and names `ppt/presentation.xml` and `ppt/slideMasters/slideMaster1.xml`.

**One example is carried end to end: `saving_validates`**, the blank/validate/save/detect block in
`crates/mjx-ooxml/docs/guide/opening_and_saving.md`. It produces a package, so the output comparison
is real rather than vacuous — but it starts from `Deck::blank`, so **every part it compares was
authored by this library and none was preserved from an input file**. It is the same blind spot
`crates/mjx-ooxml/examples/build_a_document.rs` has, and an example that opens a committed fixture is
the first item of the backlog for exactly that reason. The remaining eighteen blocks are mechanical.

**`@mjx/ooxml` now resolves inside the Node test suite.** `build-npm.sh` links `npm/` into
`tests/node/node_modules/@mjx/ooxml`, the way `npm link` would, so the specifier the guide shows is
the specifier a consumer writes and the one the example really imports. `node_modules` is git-ignored;
nothing is committed.

**One latent defect fixed on the way.** `xtask/tests/entry_points.rs` decided whether a crate hosts a
guide with `source.contains("pub mod guide")` — a prefix match, so `xtask`'s new `pub mod
guide_examples` classified the one crate that deliberately has no guide as hosting one. It now
matches the declaration `pub mod guide;` as a whole line.

## [0.0.153] - 2026-09-09

### The Word walkthrough was claimed to be compared byte for byte in both bindings, and was compared by nothing (MJXOFF-239, H9)

`CLAUDE.md` said of the acceptance triples that *"every one compares its output against the Rust one
part by part, byte for byte. A method wired to the wrong `Deck` method changes one payload and fails
there."* That was **false for Word in both bindings**.
`bindings/mjx-python/tests/test_build_a_document.py` and
`bindings/mjx-wasm/tests/node/build_a_document.mjs` transcribed
`crates/mjx-ooxml/examples/build_a_document.rs` call for call and each wrote its *own* `.docx` into
`target/examples/`. Nothing ran the Rust example; nothing compared. Both files passed, which is why
nobody saw it: a gate phrased *"X is covered and green"* is green precisely when X is skipped.

Both Word walkthroughs now run the Rust example as a subprocess and assert the part-name sets are
equal and no payload differs, exactly as PowerPoint's and Excel's have. Each was proved able to fail
before it was believed: one table cell changed from `+12%` to `+13%` reddens both and names
`word/document.xml`.

**`xtask/tests/walkthrough_triples.rs` closes the class.** It derives the walkthroughs from
`crates/mjx-ooxml/examples/` rather than listing them, and fails when a walkthrough has no copy in a
binding, when a binding holds a copy of a walkthrough that does not exist, or when a copy does not
both run the Rust example of its own name and read two packages through the shared payload reader.
Checked out against the pre-fix files it names all four defects; against a fourth walkthrough with no
copies it names both absences. What it deliberately does not claim is that a comparison it can see
actually asserts anything — that is an assertion in a language this crate cannot execute, and a
textual gate claiming otherwise would be the same nominal check the file exists to prevent.

**The Python OPC helper exists once.** `_part_payloads` had three copies — one each in
`test_build_a_deck.py`, `test_build_a_workbook.py` and `test_validation_artefacts.py` — plus an
open-coded fourth reading in `test_surface_coverage.py`. All four now go through
`bindings/mjx-python/tests/opc.py`, which is the Python half of what
`bindings/mjx-wasm/tests/node/zip.mjs` already was. A helper copied per file is a comparison that can
go missing from a file unnoticed, so the gate also fails any binding test that names archive
machinery (`zipfile`, `node:zlib`) outside the one reader.

**The rest of the set was swept, and Word was the only hole.** The validation-catalogue triple is
whole in both bindings: `test_the_generator_set_is_the_whole_catalogue` compares the produced set
against `GENERATORS` in both directions, and its Node sibling makes the same both-directions
`deepEqual` inside its one comparison test. The prose that asserted the false claim — `CLAUDE.md`,
`README.md`, `crates/mjx-ooxml/docs/guide/README.md`,
`bindings/mjx-python/docs/guide/how_much_is_exercised.md`, `xtask/tests/binding_projection.rs`'s own
header and the Rust example's — now says what is checked and names the gate that checks it.

### Also: the derived-roster gate failed on its own calibration sample (MJXOFF-253)

`epic/phase-h` was red at `ffa5a25`. `xtask/tests/derived_rosters.rs`'s scanner calibration holds a
`SAMPLE` of three deliberately *partial* rosters, so that a scanner which has stopped matching fails
there — and `workspace_rosters()` swept every tracked `.rs` file including that one, so the fixture
was in the corpus it calibrates and was exactly the shape the corpus sweep rejects. No `ROSTERS` row
could truthfully register it: a row asserts *"this list is the whole of that population"*. The sweep
now skips that one file, with the reason where the skip is and an assertion that the excluded path
is still in `git ls-files`, so a rename cannot make the exclusion a silent no-op. The calibration is
untouched — it calls `rosters_in` on `SAMPLE` directly under a synthetic path.

## [0.0.152] - 2026-09-09

### Blind sweeps: no test enumerates by hand what the repository enumerates itself (MJXOFF-225, H8)

MJXOFF-224 found `crates/mjx-ooxml-types/src/child_order.rs`'s four safety suites sweeping three of
nine generated tables — a literal that was the whole population when it was written and had been a
fraction of it for six schemas since, while every count it printed stayed plausible. That
instance is fixed. This closes the **class**.

`xtask/tests/derived_rosters.rs` sweeps every tracked `.rs` file for a *roster*: a literal `[…]`
list, in test code, whose elements all name members of a population this repository derives — the
workspace crates, the generated child-order tables, the generated simple-type modules, the facade
handle types, the validation catalogue pages, the validation artefact formats. Matching is on the
first string of each element, so it sees a roster written as bare strings, as tuples or as struct
literals, which is why a grep for `for … in [` finds barely any of them. Every roster it finds must
be registered as the **whole of a named derived population**, and the gate re-derives that
population and compares both ways. Its own scanner is calibrated against a sample held in the file,
so a scanner that has stopped matching fails before it can report a clean bill.

What the sweep found, beyond the instance MJXOFF-224 had already closed:

- **`crates/mjx-sml/tests/package_writer.rs` was wrong, not merely blind.** Its forbidden-dependency
  list named five crates and read as though it named all of them; `mjx-omml` and `mjx-vml` sit at
  rank 2.2 beside `mjx-chart` and were missing, as were both bindings. With the old list in place,
  `mjx-sml` could declare `mjx-omml` and that test passed.
- **`xtask/tests/codegen_drift.rs`'s curated re-export sweep** iterated the two `pub(crate)` modules
  by name. It now filters `SIMPLE_TYPE_MODULES` on `visibility`, so a third joins it the day one
  exists.
- **`CLAUDE.md`'s rank table was mirrored by convention and checked by nothing.** It is now the
  source the crate populations above are derived from, so `xtask/tests/layering.rs` compares it
  against `TIERS` crate by crate — rank and label, both directions — and `derived_rosters.rs` holds
  it against `Cargo.toml`'s `members`.
- **`layering.rs`'s own tier-exercise check** listed eight tier labels; it now derives them from
  `TIERS` as *every ranked tier except the floor*.
- Three further rosters — the catalogue pages in `validation_calls.rs`, the format tokens in
  `validation_index.rs`, the artefact extensions in `validation_calls.rs` — are now read off
  `ArtefactFormat`, which grew a `page()` accessor so that "which three of the seven pages" is
  answered beside `extension()` rather than retyped.

Every other roster the sweep found was already complete. Those are **registered rather than
rewritten**: the gate re-derives what each one claims and fails by name the moment the repository
moves under it, which is a stronger guarantee than copying a derivation into each of seven files
would have been.

**A partial sweep is expressible only by naming the subset as a population of its own.** There is no
register variant for a subset chosen by hand and justified in prose: every roster in this workspace
turned out to be the whole of some derivable population once the population was named precisely
enough, including the two that looked most arbitrary.

The gate states what it cannot see, and MJXOFF-252 owns it: a roster whose elements are not string
literals — `facade_curation.rs`'s `&[&DECK, &DOCUMENT, &WORKBOOK]`, complete today and blind by
construction — and a population nobody has named, `BasePopulation` being hand-written and therefore
this file's own instance of the defect it closes.

## [0.0.151] - 2026-09-09

### `mjx-docx`'s 158 hand-written serialization pairs, and the three that were losing content (MJXOFF-218, H7)

MJXOFF-216 found four `mjx-dml` types destroying foreign attributes, foreign children and `xmlns`
declarations, and MJXOFF-217 answered it with a gate that reads that crate's own sources: every
hand-written `FromXml`/`ToXml` must be on a ledger with an **idiom checked against the impl body**.
MJXOFF-220 wrote `mjx-sml`'s. This closes the last of the three.

`mjx-docx` writes serialization out by hand for **158 types** and every one of them is a pair, so the
shape of the risk is different from either of the other two crates. 146 of the 158 are *the same body
typed out again* — a reader storing the element's `name`, `attributes`, unknown bucket and
self-closing flag, and a writer rebuilding from exactly those four — with nothing shared, no macro
and no helper. The question is therefore not *"did somebody design this type's preservation wrongly"*
but **"did somebody copy the body wrongly"**, 146 times over, and a ledger with 146 rows would be the
longest list in the workspace and would say nothing about the one character that matters.

`crates/mjx-docx/tests/serialization_ledger.rs` compares each body against the canonical text
**character for character**, in both directions, requiring the bucket field to agree across the pair.
The twelve pairs that are genuinely different carry a row with one of three idioms, each checked
against its own body. It found three losses on its first run, all in `document/drawing.rs`:

- **`Control`** — MJXOFF-216's shape exactly. Its reader stored **no children at all** (the struct
  had no field for them) and its writer handed `RawElement::rebuilt` a fresh `Vec::new()` with the
  self-closing flag hard-coded `true`. A foreign child, a comment or an `o:` extension inside a
  `w:control` was destroyed, and `<w:control></w:control>` came back `<w:control/>`. `CT_Control`
  declares no content model, which is what made it look safe — the fidelity rule has no "the schema
  says this cannot happen" clause.
- **`WordprocessingShape`** and **`TextboxInfo`** — both read `element.empty` into a field their
  writers ignored in favour of a literal `false`, so `<wp:wsp/>` and `<wp:txbx/>` came back as
  open/close pairs. That is the loss MJXOFF-217 found twice in `mjx-dml`, twice more here.

Four cases in `crates/mjx-docx/tests/drawing_placement.rs` prove all three against markup rather than
against a source shape, and each fails against 0.0.150. `ObjectEmbed` and `ObjectLink` were correct
but spelled differently and were folded onto the canonical text, so the family really is one body and
the gate can compare rather than list.

### Three ledger files, one scanner, and a hole closed in the oldest of them

The gate lives in three per-crate files rather than one shared crate, and that is now recorded with
its third data point: the files share a source scanner — sixty lines of brace matching — and share
nothing else, because the idioms are the finding and they differ. `mjx-dml` checks three idioms over
eight bespoke pairs; `mjx-sml` follows a one-line delegation into the `as_raw_element` behind it; this
one compares 146 copies of one body to a canonical string, an arm that would be meaningless in
`mjx-dml`, where no two bodies are alike.

What the duplication costs is that a scanner improvement has to be made three times, and it had
already happened once. MJXOFF-218's own census reported 5 `FromXml` and 57 `ToXml` in `mjx-sml` where
there are 6 and 58, because a scanner keyed on a bare `impl ToXml for` cannot see
`impl mjx_ooxml_core::ToXml for ColorElement`. MJXOFF-220 fixed that in its own file;
`crates/mjx-dml/tests/serialization_ledger.rs` still carried the bare needle and now carries the
fixed scanner too, because a known hole in a gate is not a thing to leave for a ticket.

## [0.0.150] - 2026-09-09

### SpreadsheetML `@theme`: the writer and the resolver meant different colours by the same number (MJXOFF-246, H6)

`mjx-sml`'s stylesheet writer authored font 0 — the font every cell that names no font of its own
draws with — as `<color theme="1"/>`, meaning the theme's first **text** colour, which is what
Excel's own font 0 says. `mjx-sml`'s resolver read that same `1` as `lt1`, the theme's **background**.
So this library read the default font colour of every workbook it authored as *white*, and a renderer
built on `effective_cell_format` would have painted white text on a white sheet. Nothing saw it: both
candidate slots are defined in every theme, so the reference gate resolved either way, schema validity
passed and the round trip passed.

The resolver was following an *inference*, and that is the substance of this change. §20.1.6.2 is a
DrawingML clause about `clrScheme`, and the table it prints is headed **"Sequence Index"** — it states
the document order of `CT_ColorScheme`'s children. SpreadsheetML's four colour elements say only *"a
zero-based index into the `<clrScheme>` collection (§20.1.6.2)"*. That the sequence order is also the
SpreadsheetML lookup table does not appear in either clause.

**It is not, and ECMA's own markup says so.** The Part 1 5th-edition package ships
`OfficeOpenXML-SpreadsheetMLStyles/`, whose `presetCellStyles.xml` and `presetTableStyles.xml` are the
only SpreadsheetML *markup* the standard publishes — the built-in cell styles and table styles, in
`@theme` positions. Every one of the sixty-three `Normal` fonts is `<color theme="1"/>` on a bare
sheet; `Accent1`…`Accent6` and `Check Cell` are `<color theme="0"/>` over a mid-tone fill; a table
style paints `theme="0"` text onto a `theme="0"` fill darkened 35%; `Title` and `Heading 1`…`4` are
`<color theme="3"/>` with no fill at all. Under the sequence reading, all four are a colour on itself.

So the two dark/light pairs are swapped and nothing else moves:

```text
0 lt1   1 dk1   2 lt2   3 dk2   4..9 accent1..accent6   10 hlink   11 folHlink
```

This is **not** *Office over the specification*: it is the standard's own normative sample data over
an inference from a cross-reference in the same edition of the same standard. That it also agrees
with what Excel writes, and with what all three third-party producers in `tests/fixtures/` write, is
the outcome the fidelity rule wants rather than the argument for it.

**The structural half matters as much as the table.** The writer no longer spells the number: it names
`ColorSchemeSlot::Dark1` and lets `theme_color_position` supply the position. For as long as the two
halves each stated the mapping in their own words they could drift, and they did. There is one table
now and both ends read out of it — which is why MJXOFF-235's five `*_from_theme` constructors needed
no change at all.

Two gates, each red before this change and each failing for its own reason:

* `crates/mjx-sml/tests/theme_index.rs` **derives** the mapping rather than restating it. It resolves
  every font colour in ECMA's two preset files against the ground beside it and requires the two to be
  tellable apart. ECMA's worst pair separates by 39.7 of 255; the sequence reading produces 144 pairs
  under the floor, the worst of them exactly 0.0. It follows the `References/` convention — a notice
  and a pass without the tree, a failure under `MJX_REQUIRE_SCHEMA=1` — so CI now fetches Part 1 for
  its 84 KB styles member and runs `-p mjx-sml` in the schema job.
* The same suite's **agreement** gate authors a package, reads font 0's colour back out of the
  `xl/styles.xml` the writer produced, and resolves it against the `xl/theme/theme1.xml` the same
  writer produced. Nothing in it re-types the number, so a writer and a resolver that disagree cannot
  both be satisfied. This is the one-line assertion the ticket said did not exist.

`docs/validation/05-workbooks.md` gains `V-XLSX-02.7`, the Excel-side tie-break: the derivation is from
markup, and the reference implementation is what closes it.

## [0.0.149] - 2026-09-09

Four API defects that were all the same shape: one half of a pair shipped and the other did not.

### The adjustment writer reaches all three languages (MJXOFF-223, H5)

`mjx_ooxml::Deck` had `shape_adjustments` — a reader that answers *what may this handle be set to* —
and no writer for the same thing. That reader is asked in order to set, so the pair belongs on the
facade: `Deck::set_shape_adjustments`, `Deck.set_shape_adjustments` and `Deck.setShapeAdjustments`.

The ticket said the format-tier half already existed. It did not on this line —
`epic/phase-g-geometry` has never been merged into `main` or `epic/phase-h` — so
`Presentation::set_shape_adjustments` arrives **verbatim** from that branch, byte-identical to its
state there, rather than as a second implementation that would conflict when the two lines meet.

The WebAssembly argument shape was the substantive design question. A list of `(name, value)` pairs
cannot cross wasm-bindgen without `serde`, which the shipped `mjx-dml` may not grow derives for, so
the pairs arrive as **two parallel arrays** — the shape a range already takes when it becomes two
numbers — and a length mismatch is refused with `InvalidArgument`.

### A theme-following sibling for every colour convenience Excel authors with (MJXOFF-235, H5)

`Color::from_theme` was the only theme-following constructor in the Excel authoring vocabulary, and
every convenience beside it took a hex literal — so the shortest path pinned a colour into a document
whose owner may have rebranded it, and the theme-following path cost four lines. That is the standing
rule *let it inherit* inverted at the API level.

Each of the four now has a sibling — `PatternFillSpec::solid_from_theme`,
`ColorScaleSpec::two_color_from_theme`, `DataBarSpec::spanning_the_range_from_theme`,
`DifferentialFormatSpec::highlight_from_theme` — over the primitive underneath them,
`Color::from_theme_slot`. They take a **`ColorSchemeSlot`**, not a `@theme` position, because `4`
means `accent1` only to a reader with §20.1.6.2 open. `theme_color_position` is the inverse of the
existing `theme_color_slot`, and both directions of the pair are asserted rather than left to a
second copy of the table.

`crates/mjx-ooxml/examples/build_a_workbook.rs` and its two twins now state the heading **fill** as
`accent1` and keep a literal for the text on it: contrast is a constraint between two colours and a
slot states one. Following the theme is the default; it does not outrank being readable.

### A new table names a style, and the deck's own theme colours it (MJXOFF-232, H5)

`Presentation::add_table` wrote `firstRow="1" bandRow="1"` and no `a:tableStyleId`. Those flags name
*parts of a table style to emphasise*, so a table born with them and no style asked for two parts of
nothing and rendered unstyled. PowerPoint writes both halves.

A style id is always written now, and whose style it is depends on the deck: one whose
`tableStyles.xml` already names a default it really defines gets **that** id and is not touched at
all; any other gets a default authored on first use with **not one literal colour in it** — the
header row is `<a:schemeClr val="accent1"/>` under `lt1` text and the band is `accent1` at
`lumMod="20000" lumOff="80000"`. A table in a deck branded green comes out green.

**One of the twenty validation artefacts changes**: `v-pptx-03-authored.pptx`, 6478 → 6603 bytes, and
inside it exactly one part — `ppt/tableStyles.xml` gains the themed style. Its slide is
byte-identical, and the other nineteen artefacts are unchanged.

`mjx-schema-gate`'s reference resolver grows the rule that sees the class rather than the instance:
**an `a:tblPr` with an emphasis flag on and no style is an unresolvable deferral**, the twin of the
chart-series rule and the shape MJXOFF-200 named as invisible.

### A producer for every exported class (MJXOFF-228, H5)

`ResolvedColor` and `TableStyleFlags` were exported by the facade and by both bindings and returned,
taken and constructed by nothing, so a caller in three languages could name a type and never obtain a
value. Each now has exactly one producer — `Deck::resolved_scheme_color`, `Deck::table_style_flags` —
and so does `Backdrop`, via `Deck::shape_backdrop`.

**`Backdrop` was not on the ticket.** `every_exported_class_is_obtainable_from_some_other_call` found
it, which is the argument for a sweep over two assertions. It asks the type-level form of the
reachability rule and answers `binding_projection.rs`'s objection — *a signature parser that is
subtly wrong is worse than none* — by not writing one: the committed, parity-checked `.pyi` is a
declaration of the whole projected surface, and the gate reads it rather than reconstructing one from
two hand-written crates.

## [0.0.148] - 2026-09-09

The gate stopped reporting a defect of its own as a defect of everybody's files.

### An extension slot markup-compatibility resolution empties goes with the extension (MJXOFF-196, H4)

`mjx-schema-gate` validates the markup-compatibility-**resolved** view of a part, because
`mc:Ignorable` names attributes the base schemas have no declaration for. Resolution removes an
ignorable element together with its content, which is what ECMA-376 Part 3 says. Composed with the
base schemas it left a hole: `sml.xsd`'s and `dml-chart.xsd`'s `CT_Extension` declare their whole
content model as a bare `<xsd:any processContents="lax"/>`, whose `minOccurs` defaults to **1**, so
an `<ext>` whose only child was ignorable was rejected — *Missing child element(s)*. That fired on
every conformant file Office has written since 2010, in all three formats, because Office 2016 writes
a `c16:uniqueId` extension under `mc:Ignorable` on chart series.

**The shape that was refused.** The ticket's first option was to resolve fully and, on failure, retry
with the ignorable content kept. That is a try-then-fall-back arrangement: with two views a deviation
must appear in **both** to be reported, which is a gate that goes quiet, and MJXOFF-88 §7 names the
signature. Exactly one view is validated now and it is always the same one.

**What ships instead.** The rule is content-dependent and consults the schema.
`crates/mjx-schema-gate/src/wildcard_slots.rs` derives, from the pinned XSDs, every element whose
content model is `xsd:any` particles and nothing else and cannot match the empty sequence — a
*wildcard slot*, an element that exists only to carry one foreign child — and `inspect.rs` drops such
an element when resolution emptied it. Ignoring the extension without ignoring the slot that held it
is half a resolution.

The derivation finds **five**: `{…/drawingml/2006/chart}ext`, `{…/spreadsheetml/2006/main}ext`,
`{…/spreadsheetml/2006/main}Schema`, `{…/spreadsheetml/2006/main}DataBinding` and
`{…office:office}equationxml`. The ticket named two. `xtask/src/validation/ingest.rs` had found a
third by hand, with the note *"so that a fix cannot stop at two"* — a derivation is the general form
of that note, and `every_wildcard_slot_in_the_reference_schemas_is_listed` recomputes the committed
table from the XSDs on every run with `References/`, so a new schema or a different edition of the
reference tree reddens rather than passing silently.

**Why not keep the content instead.** Every wildcard that admits an ignorable extension is
`processContents="lax"`, and no schema for such a namespace is loaded, so a validator handed the
content accepts it **unread** — which is exactly what the ticket's own middle view measured and
called "validates". Keeping the content and dropping the slot are validation-equivalent; dropping
needs one bit of schema knowledge instead of a content-model matcher. `mjx-mce` is untouched: its
resolution is correct MCE, and it was the gate's *use* of it that was over-broad.

**Three properties keep the rule from quieting anything.** It fires only on the derived table; a
slot's content model is wildcards and nothing else, so dropping it can never hide a missing *named*
child; and it fires only when the source element had element children, so an `<ext/>` this library
authored empty is still a failure.

**The gate.** `xtask/tests/mce_extension_seam.rs` replaces the reproduction that lived in
`xtask/tests/office_corpus.rs`, which is deleted — it was written against the defect, so its own red
was the signal the defect was gone. The new suite runs a worksheet **and** a chart series carrying an
ignorable extension through `assert_authored_deck_is_schema_valid`, which tolerates nothing, and adds
both discriminations: an author's empty `<ext/>` still fails, and a `w14:` element inside a `w:rPr` —
the counterexample that ruled out "keep ignorable elements always" — is still removed. CI names the
suite beside the other two `xtask` suites that need `References/`.

**The residue, named.** The gate says nothing about the markup *inside* an ignorable extension, and
it never could. Six prose sites carried the old limitation — two more than the ticket listed — and
each now says that instead: `crates/mjx-chart/docs/guide/fidelity_and_gaps.md`,
`crates/mjx-xlsx/docs/guide/deliberate_limitations.md`, `docs/validation/06-the-office-pass.md` §5
and §8, `tests/office-authored/README.md` and `xtask/src/validation/ingest.rs`.

## [0.0.147] - 2026-09-09

Two refusals that had already changed something. One refused too late; the other refused when it
should not have.

### A cell comment refused by a dialogsheet no longer leaves the comments part behind (MJXOFF-213, H3)

`Workbook::add_cell_comment` aimed at a tab that cannot carry one — the dialogsheet in
`tests/fixtures/print_and_sheet_kinds.xlsx` — refused correctly and *late*. By the time the refusal
came, `xl/comments1.xml` had been inserted, `[Content_Types].xml` amended and the sheet's `.rels`
grown, so a caller who handled the error and saved anyway shipped a comments part for a comment that
does not exist. The MJXOFF-210 preservation gate found it and carried it in `KNOWN_DEFECTS`.

**What changed.** The tab's kind is checked first, before the first `insert_part` — from the sheet
list this crate already resolved at open, so the answer costs no parse and does not depend on whether
the tab happens already to have a VML drawing. The legacy VML drawing is then created before the
comments part, because it is the half that reads the sheet's markup and so the last step that can
turn a tab away. This is MJXOFF-210's own shape (*"the theme is written last, after everything that
can refuse"*), applied to the other half of the same rule.

`KNOWN_DEFECTS` is now **empty**, and the gate compares it against the sweep in both directions, so
it cannot be quietly refilled. `crates/mjx-xlsx/tests/comments.rs` pins the refusal directly: it
asserts the premise (the tab really is a dialogsheet, the call really was refused) and then compares
**every entry** of the container, name and payload, rather than the three the defect happened to
touch.

### A part that has been edited is present, and it has content (MJXOFF-222, H3)

`Package::part_bytes` answers `None` for two different reasons — the part is absent, or the part is
dirty — because an `Edited` body has no stored bytes left. Call sites across six crates read that as
one answer. The visible failure was `Document::from_package` and `Presentation::from_package`
reporting `MissingDocumentPart` / `MissingPresentationPart` **naming a part that was present and
correct**, which is exactly the "authored part by part" case both constructors' doc comments
advertise.

Nothing had caught it because `PartBody::Parsed` still answers `Some`: merely *reading* a part as a
tree is safe, and only a part someone has mutated trips it.

**What changed.** `mjx-opc` grows the two questions it was missing, each with one meaning:

| The question | The call |
|---|---|
| is this part in the package? | `Package::contains_part` |
| what does this part contain? | `Package::part_payload` |
| does it still carry the bytes it arrived with? | `Package::part_bytes` |

`part_payload` borrows when the part still holds its bytes — every part of a file nobody has edited,
so the ordinary path still costs no copy — and serialises the tree when it does not, through the same
writer `save` uses, so what it hands back is byte for byte what saving would write. The third row is
a *fidelity* question and its doc comment now says so.

Seven presence probes moved to `contains_part` and forty-six content reads to `part_payload`, across
`mjx-chart`, `mjx-docx`, `mjx-pptx`, `mjx-xlsx`, `mjx-ooxml` and `mjx-schema-gate` — every site in
the workspace that asked the storage question while meaning one of the other two. Only one of them
was proved live by measurement (`Presentation::chart_part_bytes`, which answered `None` for a chart
the caller had just retitled — a `mjx-ooxml` test comment had been excusing it); the rest are on
parts nothing currently reaches with `part_tree_mut`, and they were fixed anyway because the fix is
one call and the next edit that dirties such a part would reopen the defect silently.

**How the sites were counted.** A temporary probe in the `Edited` arm of the package's byte accessor
recorded a backtrace for every call that met a dirty part, and the whole workspace was run under it
with `--all-features`, plus the three walkthrough examples and the validation-artefact catalogue.
Eight hits: one library call site and seven test helpers that ask the storage question on purpose.

**One workaround is not retired here, because it is on another branch.** The reference pack
MJXOFF-207 added — the child that reported this defect — saves and reopens the package instead of
calling `from_package`, and says in its own doc comment why. Its crate is not in this tree; the extra
round trip can go when the branches meet, which is MJXOFF-240.

### Breaking

Twelve accessors that hand over a preserved part's bytes now answer `Option<Cow<'_, [u8]>>` rather
than `Option<&[u8]>` — `chart_part_bytes` on all three formats, and `mjx-pptx`'s
`picture_image_bytes`, `ole_object_part_bytes`, `ole_snapshot_image_bytes`, `activex_part_bytes`,
`activex_state_bytes`, `activex_snapshot_image_bytes`, `vml_part_bytes`, `ink_part_bytes` and
`diagram_part_bytes` — plus `mjx_docx::Document::alt_chunk_payload`, which answers
`(Cow<'_, [u8]>, &str)`. The borrow is still a borrow whenever the part is not dirty; `.as_deref()`
recovers the old comparison. **The facade and both bindings are unchanged**: `mjx_ooxml` already
copied into a `Vec<u8>` at that boundary.

## [0.0.146] - 2026-09-08

### Removing a slide another slide links to no longer leaves a file that can never be saved (MJXOFF-212, H2)

`Presentation::remove_slide` — and `Deck::remove_slide` with it — unwired the slide from
`p:sldIdLst`, dropped the presentation's relationship to it and deleted the part, and left every
*other* part's reference to that slide exactly where it was. On `tests/fixtures/hyperlinks.pptx`,
where slide 0's rectangle jumps to slide 1:

```rust
deck.remove_slide(1)?;   // succeeded
deck.save();             // Err(RelationshipTargetMissing { … "/ppt/slides/slide2.xml" })
```

`Package::validate` — which `save` runs — refuses a relationship whose internal target is not in the
package, so **the edit succeeded and the document could never be written back**. Found by the
MJXOFF-210 preservation gate on its first sweep, and registered in its `KNOWN_DEFECTS` until now.

**What changed.** A slide is named from three places, and all three now go with it: the `p:sldId`
that lists it (as before), an `a:hlinkClick` / `a:hlinkHover` on a run or on a shape's `p:cNvPr` in
any part, and a custom show's `p:custShowLst > p:custShow > p:sldLst > p:sld`. Each is a
relationship **plus** the element naming it, and both are removed — the element first, then the
relationship.

**The decision, and the two options rejected.** Dropping only the relationship and leaving the
markup is not a fix at all: it trades `RelationshipTargetMissing` for
`UndeclaredRelationshipReference`, still unwritable, and on a part this library never authored it
would slip past `validate` altogether and hand PowerPoint a file to repair. Retargeting the link at
whichever slide takes the removed one's place would put a destination the caller never chose in
place of the one they deleted — the inverse of the standing rule that a default is supplied only in
the absence of the user's own. Refusing the removal while another slide links to it damages nothing,
but it makes a slide undeletable, and a deck can be *made* undeletable by adding a link to it. So a
run that used to be a link **keeps its text and loses its link**, which is what PowerPoint does, and
a custom show loses its entry while the show itself stays.

**The rule is keyed on the reference, not on a list of element names.** Any element naming a
relationship that resolved to the removed slide is removed, so markup outside the schema — a vendor
extension, an `mc:AlternateContent` alternative — cannot quietly leave behind a reference `validate`
would refuse. The sweep runs after the cascade, over parts that survive, and reads a part before it
writes one: a part that holds a relationship it never names in markup keeps its original bytes.

**A referring slide is rewritten**, and stops being re-emitted byte for byte. That is the price, it
is stated on the method, in the facade, in both bindings and in the pptx guide, and it is bounded —
only the parts that actually named the removed slide are touched. The preservation gate's
declaration for `remove_slide` says so: `changed(SLIDE, Any)` and `changed(RELATIONSHIPS, Any)`, with
the presentation part still pinned at exactly one so the wildcard cannot hide a sweep over the deck.

**And the bound now has a test, which it did not when this was first written.** The sweep visits
every part holding a relationship to the removed slide, and a part can hold one it names nowhere in
markup — an unreferenced relationship is valid OOXML, and `remove_shape` leaves them behind on
purpose. Such a part is read and not rewritten. That was documented, exercised on every removal, and
guarded by nothing: forcing a rewrite of every visited part left `mjx-pptx` and the preservation gate
entirely green.

Two things had to be understood to close it. The first is that the case, though it fires on every
single removal, only ever fired on `presentation.xml` — which `remove_sld_id` has dirtied an instant
earlier, so the guard had no observable effect anywhere any test reached. The second is that
**byte equality cannot express the property at all**: the fidelity serializer reproduces an unmutated
tree byte for byte, so "kept its original bytes" and "re-serialized without changing anything" are
the same bytes. What differs is *provenance*, and provenance decides **scope** — `validate` walks the
parts this library authored and spares the ones it did not. So the assertion that bites is a
user-visible one: a deck carrying something it *arrived* with that this library would refuse to
author (two shapes sharing a `p:cNvPr@id`) opens, saves, has a slide removed, and **still saves**.
Dirty the untouched slide and it stops saving, faulted for markup nobody asked us to touch.

### `validate`'s markup-reference check is narrower than its name (MJXOFF-238)

Recorded, not fixed here. `Package::validate`'s content-type and relationship checks are
package-wide, but `check_relationship_references` — the one that catches markup naming a relationship
nothing declares — walks `authored_xml_parts()` alone, and so does `mjx-pptx`'s `validate::check`.
The scoping is deliberate and mostly right: a deck opened and left alone must never be faulted for
what it arrived with, and the bound above depends on exactly that rule.

It answers *"was this markup ours?"*, though, and not *"did our edit break this markup?"* — and those
come apart for one shape: an edit that changes a part's **relationships** without touching its
**body**, because a `.rels` edit does not mark its owning part authored. No shipped method does that
today (this one rewrites the part in the same breath; `remove_hyperlink_rel_if_unreferenced` reads
the markup first), but `Package::remove_relationship` is public and the next such edit would reopen
it. It is why the rejected "drop the relationship, keep the markup" option was worse than MJXOFF-212
itself said: on the referring slide of a real file it would not have been reported at all.

### The same defect does not exist elsewhere, and one nearby gap is now ticketed

MJXOFF-212 asked whether the shape generalises to the other `remove_*` methods. It does not, and the
reason is structural rather than lucky: **every other part-deleting edit either unwires the inbound
relationship before deleting, or refuses to delete a part anything still references.**
`Package::remove_part_if_unreferenced` is what `mjx-docx` uses and cannot dangle by construction;
`mjx-xlsx`'s two direct `remove_part` calls remove the sheet's relationships first (comments) or
guard on `is_reachable` (a chart's embedded workbook); `clear_notes` is safe for a different reason
again, in that nothing but its own slide ever names a notes slide. `remove_shape` leaves
relationships alone on purpose, and that stays correct: an *unreferenced* relationship is valid
OOXML, and only a *dangling* one is not.

One nearby gap is now **MJXOFF-237**, recorded rather than fixed. PowerPoint's sections live in
`presentation.xml`'s `extLst` and list slides by the slide's **id number, not by `r:id`**, so they
are not relationship references, this sweep does not see them, and a section keeps an entry for a
slide that has gone. That cannot make a package invalid and `validate` is right not to complain. The
markup is a Microsoft extension outside ECMA-376 — it is not in `References/`, and no committed
fixture carries one — so the ticket's first task is to confirm it against a file Office wrote rather
than to change anything.

## [0.0.145] - 2026-09-08

### Run coalescing stops merging two runs a resolved colour cannot tell apart (MJXOFF-233, H1)

0.0.144 documented this as a live defect. It is now fixed, and the pptx guide's **Known defects**
table is empty.

`Presentation::coalesce_paragraph_runs` merged two adjacent runs when their *effective* character
properties compared equal. Effective means **resolved**, and resolution loses information in two
directions, so "equal" was not the same as "the same":

- **A transparency disappeared.** `resolve_fill` bakes a colour to `RRGGBB` and says in its own doc
  comment that a resolved `a:alpha` is not represented. Two runs differing only by an `a:alpha`
  compared equal and **one run's element was deleted**, taking its transparency with it.
- **A theme link became a literal.** An `a:schemeClr` and the `a:srgbClr` it resolves to against the
  deck's theme compared equal, so a merge could leave a hard-coded colour where a theme link had
  been — the exact inverse of the standing rule that where OOXML lets a value inherit, it must
  inherit. Which of the two survived was positional: `coalesce_adjacent_runs` merges the later run
  into the earlier.
- **The same held for fonts, which the ticket did not name.** `Presentation::resolve_theme_fonts`
  replaces a `+mj-lt` / `+mn-lt` typeface with the font the theme's scheme names for it, so a run
  that follows the theme and a run that hard-codes today's answer were indistinguishable once
  resolved. Fixed with the same condition.

`unmodeled_state_eq` caught none of it: `a:solidFill` and `a:latin` are modelled, so they are
filtered out of the residual it compares and both runs' residuals were empty.

**The fix narrows the merge rather than changing what resolution answers.** A third condition now
holds before two runs join: their own, *unresolved* colours and typefaces must agree, via a new
`mjx_dml::CharacterPropertiesSpec::resolution_sensitive_eq`. It lives in `mjx-dml` because that crate
owns both the spec and the resolver, so the knowledge of what resolution discards sits beside the
code that discards it; `mjx-pptx` consumes it downward. The ticket's other candidate — teaching
`resolve_fill` to carry the alpha, now representable via 0.0.143's `ColorSpec::Transformed` — was
rejected: it changes what every `effective_*` reader answers across three formats, the facade, both
bindings and three `effective_properties.md` pages, and it does not address the theme-link half at
all. Among candidate fixes, the one that cannot break a caller already working wins, and a condition
that can only ever *refuse* a merge cannot.

**What it costs, stated in the method's docs, the facade's, and the guide:** a run that names a
colour or a typeface **explicitly** no longer merges with a neighbour that **inherits** the same one.
Every other property still compares as meaning rather than as markup — a run stating `b="1"` still
merges with a neighbour that inherits bold — and the method's own purpose, undoing
`set_text_range_properties`' splitting, is untouched, because those runs all carry identical explicit
properties. The comparison is confined to the seven resolution-sensitive fields and skips the ten
resolution copies verbatim; it is written as a full destructuring with no `..`, so a property added
to `CharacterPropertiesSpec` fails to compile there until someone has classified it.

**Both methods leave the preservation gate's `NEVER_EXERCISED` register.** The register said a
*fixture* holding two adjacent runs would retire them. It did not need one — the preparation makes
the state, in two edits whose order is the point: formatting the first character alone splits the
paragraph's opening run in two, and restyling the shape then gives the halves identical `a:rPr`.
The old preparation only restyled, which is why it left one run per paragraph and nothing to merge.

`mjx-docx` was checked and has no run coalescing at all — nothing there merges runs, and nothing else
in the workspace compares resolved character properties for equality.

## [0.0.144] - 2026-09-08

### The landing: one entry point, and Phase G's register closed (MJXOFF-230, G13)

**Ten guide sets were written in this phase and a person arriving at this repository still landed on
a `README.md` that predated all of them.** Its "Guides" section listed the PowerPoint, Word and Excel
pages one by one and named none of the other seven sets — not the facade's, not the packaging tier's,
not DrawingML's, SpreadsheetML's, the upper markup's, the generated vocabulary's or the bindings'.
This is the difference between *documentation exists* and *documentation is found*.

- **`README.md` reaches every guide set in one hop**, one row per set linking that set's own index,
  with `docs/api/README.md` named at the top as the one link that reaches everything. The
  page-by-page tables are gone: they duplicated the index, and the duplicate is where the rot was.
- **`PLAN.md` says what shipped.** Phases 4, 5 and 6 are marked done and a new section at the head of
  the phase list says plainly that the list is now a record of how the library was built — naming the
  two things that are *not* done, the human Office pass and rendering.
- **Every crate root sends a reader somewhere.** Eleven of the twenty-one members said nothing about
  where their prose lives, including `mjx-opc`, which *hosts* the packaging tier's whole guide set. A
  crate with a guide now links it; the seven without one name the page that covers them, as a path
  rather than an intra-doc link, because nothing may point upward.

### A count on an entry-point document is derived, or it is absent

`xtask/tests/entry_points.rs`, seven checks. **A landing page is where counts go to die** — the
most-read and least-tested document in a repository, and exactly where a figure is typed once and
quoted for a year. What the derivations found:

| Claim | Was | Is |
|---|---|---|
| Runnable programs on the front page | twenty-six, listing twenty-five commands | **28**, and the list omitted `mjx-ooxml`'s `build_a_workbook`, `mjx-xlsx`'s `chart_range_cost` and `mjx-xml`'s `mjx248_measure` |
| Excel guide pages (`PLAN.md`) | thirteen | **17** beside their index |
| Excel examples (`PLAN.md`) | six | **7** |
| Generated lines in `mjx-ooxml-types` | 84,107 | **84,128** |
| That crate's own lines | 85,296 | **85,399** |
| The child-order table | 59,512 | **59,529** |
| Value classes (`docs/api/README.md`) | 185, where the guide it indexes said 186 | **181** |
| Enumerations (`docs/api/README.md`) | 100 | **102** |
| Enumerations (`test_enums.py`'s docstring) | seventy-four | **removed** — its own `MEMBER_COUNTS` pins 61 and the module projects 104, so the number matched neither |

The three `mjx-ooxml-types` figures were wrong *when they were written* and had been copied into
seven documents by the time anyone counted; no measure reproduces them, and the three deltas differ
from each other, so they were not one alternative definition either. The value-class figure is the
more interesting one: the guide's own sentence explains it — "beside the handles sit 186 value
classes" is what you get by subtracting the enumerations and the exceptions from the class total
*without* also subtracting the three handles and `Format`/`FormatFamily`.

The sweep is what stops the next one. A number of five or more anywhere in `README.md`, `PLAN.md` or
`docs/api/README.md` must lie inside a claim's sentence or inside a `NOT_A_COUNT` entry with its
reason, and the exemption table cannot rot. Five is the line because **every count this phase found
stale was at or above it**, and sweeping below five would need an exemption beside every "the two
bindings" in the repository — a table of fifty exemptions is where a reviewer stops reading, which is
the failure the file is about.

### Known defect: run coalescing compares a resolved colour (MJXOFF-233)

MJXOFF-219 found `theme_model.rs` asserting a colour loss as expected behaviour. The same loss has a
second consumer, and this one **deletes content**. `coalesce_paragraph_runs` merges two adjacent runs
whose *effective* properties compare equal; effective means resolved, and `resolve_fill` says in its
own doc comment that a resolved `a:alpha` is not represented in the result. So two runs differing
only by transparency compare equal and one is deleted — and so do a run carrying `a:schemeClr` and a
run carrying the literal that scheme resolves to, which can leave a hard-coded colour where a theme
link was. `unmodeled_state_eq` does not save either: `a:solidFill` is modelled, so both residuals are
empty. Both methods sit in the preservation gate's `NEVER_EXERCISED` register, so nothing was going
to find this by running.

Documented on both methods, on both facade counterparts, in the guide beside the sentence that
recommends the call, and in a new **Known defects** table at the head of the pptx fidelity page —
which until now opened by saying that nothing on it was an oversight. Fixing it changes either a
documented promise or the public output of every `effective_*` call across three formats and both
bindings, so it is MJXOFF-233 rather than a change made in a documentation unit.

### MJXOFF-198 §6 is closed

Every item fixed, ticketed, or recorded as deliberate with its reason; the closing register is a
comment on that epic. Newly ticketed here: **MJXOFF-232** (F4 — `add_table` writes `firstRow` and
`bandRow` into a deck with no table style for them to resolve against, which is MJXOFF-200's shape
with the half a resolution gate cannot see), **MJXOFF-233** above, and **MJXOFF-234** (nothing
compares the `.pyi`'s docstrings against the Rust docs they restate — symbols are compared in both
directions, the sentences beside them are not).

## [0.0.143] - 2026-09-08

### A colour-transform surface on `ColorSpec` (MJXOFF-219, G14)

**`V-PPTX-02.4` is R3 — the third-highest risk item in this repository — and it was the only entry in
the human Office pass with no artefact at all.** Not because nobody had written one, but because
nobody *could*: `ColorSpec` is the interner-free description every authoring caller hands in, from
`Color::from_spec` up through `FillSpec::solid`, `CharacterPropertiesSpec::with_color` and every
facade call above them, and it carried a colour's kind and its value and **no transform children**.
Both directions of the corpus were closed at once — nothing could generate a file with a colour
transform in it, and no committed fixture has one either.

The measurement that set the scope, taken across every committed `.pptx`/`.docx`/`.xlsx` before the
decision:

| `a:tint` | `a:satMod` | `a:shade` | `a:alpha` | `a:comp` | `a:gray` | `a:gamma` | `a:invGamma` |
|---|---|---|---|---|---|---|---|
| 35 | 23 | 13 | 7 | 0 | 0 | 0 | 0 |

`V-PPTX-02.4` names the four that occur **zero** times; `tint`/`satMod`/`shade` are what Office
writes constantly for theme-colour variants. Scoping this to the risk item alone would have shipped
a surface nothing exercises while leaving the common cases unauthorable — so the scope is the whole
of `EG_ColorTransform`, all twenty-eight members. (And all thirty-five occurrences sit in a
`theme1.xml`, a `slideMaster` or a `slideLayout`. **Not one is on a slide**, because every fixture
here was written by this project or by LibreOffice rather than by Office.)

- **`mjx_dml::ColorTransform`** — the twenty-eight members with their values, plus an `Other` bucket
  for an element the group does not name, or one it does whose `@val` is absent or unparseable, so a
  transform this model cannot read still round-trips rather than being deleted.
  **`mjx_dml::ColorTransformKind`** names the same twenty-nine without their values, and
  **`ColorTransformValue`** says what each carries; that is the shape `ColorKind`/`ColorSpec` already
  had one layer up, and it is what lets both bindings project the group as an *enumeration* their own
  suites check member by member.
- **Builders on `ColorSpec`** — one generic `with_transform`, plus six named conveniences for the
  four transforms that occur in the corpus and the `lumMod`/`lumOff` pair PowerPoint writes for every
  *"Accent 1, Lighter 40 %"*. **Every builder appends.** Order is part of the markup: the group is an
  unbounded `xsd:choice` applied left to right, so the same transforms in another order are another
  colour, and a builder that merged into a set would quietly write a different file.
- **The read side closes with it.** `Color::spec()` used to drop transform children, so a
  `spec()` → `from_spec()` round trip lost a producer's. It no longer does. An audit of every shipped
  `.spec(` and `from_spec` call site found the loss was **latent**: every mutating API in the
  workspace replaces a colour from the caller's own spec rather than reading one back, so no shipped
  path performed that round trip on an opened file. The preservation gate would not have caught it
  either — it is part-granular, its arguments are fresh literals, and no fixture carries a transform
  on an editable surface.
- **`xtask`'s validation catalogue gains the artefact.** `v-pptx-02-authored.pptx` grows two rows of
  swatches: the four transforms `V-PPTX-02.4` names plus `a:inv` over a fixed `4472C4`, and
  `tint`/`shade`/`satMod`/`lumMod`+`lumOff`/`alpha` over the theme's accent 1, each row led by an
  untransformed baseline. Both bindings write the same twelve swatches, and the three artefacts are
  still compared part by part, byte for byte.

`crates/mjx-dml/src/resolve.rs`'s caveat stands and is meant to: `comp`/`gray`/`gamma`/`invGamma`
follow *a documented interpretation* and are **not** guaranteed pixel-identical to Office, unlike
`lumMod`/`shade`/`tint`/`alpha`. Being able to author one is not evidence that resolving it is right
— it is what finally gives the person with PowerPoint open something to point the eyedropper at.

## [0.0.142] - 2026-09-08

### The projection across three languages, audited then documented (MJXOFF-226, G12)

**Each of the three walkthroughs exists in Rust, Python and TypeScript, and every one compares its
output part by part, byte for byte.** That proves the projection is *wired*. It does not prove it is
right *across the surface*, and the difference is the whole of this release:

> Our gates reliably ask whether a value **reaches** somebody; they do not ask **at how many
> distinct points** the surface was ever exercised.

`xtask/tests/binding_projection.rs` asks. **966 of 1,619 declared Python members (59.7%) and 900 of
1,743 exported WebAssembly functions (51.6%) are named by any test at all**; the rest — 653 and 843
— are exercised by nothing. The walkthrough framing turns out to be the smaller half of the story:
only 18 Python members and 33 WebAssembly ones are reached by a walkthrough *and nothing else*. The
three parity pairs do almost all of the work.

Both totals are asserted **exactly** rather than as floors, so a method added without a test fails
the build with its own name in the message. The same file holds all 1,743 exported functions to the
camelCase rule, with a ledger of the seven names JavaScript itself forces (`toString`).

### Fixed

- **`CellFormatSpec` had three shapes in three languages, and the WebAssembly one could not say what
  the format says.** Rust carries twelve public `x:xf` attributes; Python declared four readable
  ones against twelve constructor keywords, and TypeScript had four getters, six builders, and no
  way to reach `@applyAlignment` or `@applyProtection` at all. §18.8.9 makes the six `apply*` flags
  three-valued — absent *participates*, `"0"` *suppresses* — and the four builders that existed
  could only ever write `"1"`. **All twelve attributes are now readable in both bindings and
  writable in both**, through eight new Python getters and, in TypeScript, eight getters and six
  `withApplies…` builders taking `boolean | undefined`. Purely additive: no existing call changes.
- **Six doc comments said Word and Excel were "detected, not yet editable".** Both surfaces have
  existed since MJXOFF-139 and MJXOFF-137, and `Format::is_editable` returns true for every format
  except `WorkbookBinary`. Three sites in each binding, plus a *seventh* variant of the same claim
  in the committed `.pyi` that disagreed with the Rust comment it is generated from — which is how
  it survived: `bindings/mjx-python/tests/test_stub_parity.py` compares names, and checks only that
  a docstring exists.
- **Both binding READMEs claimed 257 `Deck` methods.** It is 255, and it is 255 in both languages.

### Documented

- **A guide set for the bindings**, six pages, reachable from `docs/api/README.md`: an index, then
  Installing, The mapping rules, What is not projected and How much is exercised under
  `bindings/mjx-python/docs/guide/`, and The TypeScript surface under `bindings/mjx-wasm/docs/guide/`
  — hosted by the crate it is about, because the two bindings are siblings and neither may see the
  other. `mjx_python::guide` and `mjx_wasm::guide` wire them into rustdoc.
- **The three surfaces are method-for-method identical across the two languages**: 255 on `Deck`,
  123 on `Document`, 138 on `Workbook`, read off the committed `.pyi` and the generated `.d.ts`
  rather than off either binding's source. So are the 185 value classes and the 100 enumerations.
  Only three exported names differ, and each is forced: Python's eleven `OoxmlError` subclasses,
  TypeScript's `CellExtent` and `CellAddress`.

### Recorded, not fixed

- **`ResolvedColor` and `TableStyleFlags` are dead exports in all three languages.** Both are
  re-exported by `crates/mjx-ooxml/src/lib.rs` and projected by both bindings, and no facade method
  returns, takes or constructs either. `xtask/tests/facade_curation.rs` cannot see it: that ledger
  is about methods, and a type with no producer is a shape it was never asked to look for.
- **`blank_with_properties` reaches neither binding**, so no Python or TypeScript caller can set a
  document's title or author. A Rust caller who reaches past the facade still can.
- **The six `(u32, u32)` returns** — `table_dimensions`, `cell_span`, `merged_cell_anchor` on `Deck`
  and `Document` — and **the twenty-five chart methods spelling `series_idx` where their `Workbook`
  siblings say `series`**. Both are renames in three languages at once.

## [0.0.141] - 2026-09-08

### The crate nothing re-derived, audited then documented (MJXOFF-224, G11)

**`mjx-ooxml-types` is 85,296 lines of which 84,107 are generated, and until this release nothing
anywhere asked whether the committed output was still what the generator would produce.**
`CLAUDE.md` decides that generated source is committed rather than built by a `build.rs`, and that
decision is right — a `build.rs` would put a 5,000-page specification and a `rustfmt` run of 84,107
lines on every consumer's critical path. Its consequence had never been written down:

> Nothing re-derived the committed output, so a generator defect was frozen into the repository
> rather than failing on the next build — and the committed file is the only artefact anyone reads,
> which makes a defect indistinguishable from a deliberate choice.

The compiler catches the structural half of that and no more. Rename a generated enum by hand and
the crate stops compiling; change a wire token, a rank in a child-order table, a doc comment
recording an `ST_*` symbol or a row of `COVERAGE.md`, and nothing notices. Those are precisely the
parts a reader trusts and no build touches.

**The answer to the question, asked for the first time: the committed output is exactly what the
generator produces.** All thirteen artefacts, 2,626,921 bytes, byte for byte.

### Added

- **`xtask/tests/codegen_drift.rs`** — six tests in two tiers, because only one of them can run
  everywhere.

  `the_committed_output_is_what_the_generator_produces_today` regenerates every artefact in memory
  and compares it byte for byte with what is committed. It is the whole answer, and it is
  local-or-gated: it **skips** when `References/` is incomplete, and `MJX_REQUIRE_CODEGEN=1` turns
  that absence into a failure. **No workflow can set it today**, and the obstacle is not a missing
  switch: `.github/scripts/fetch-ecma-schemas.sh`'s `ARCHIVES` holds ECMA-376 Part 4 (Transitional
  schemas) and Part 2 (OPC schemas), and this generator needs **Part 1** for two of its three inputs
  — the Strict schema set and `presetShapeDefinitions.xml`. Growing that list is MJXOFF-197's, which
  owns the same download for `crates/mjx-dml/tests/guide_formula.rs`'s preset-geometry sweep; one
  archive unlocks both.

  The five beside it re-derive everything in the committed output that needs **no schema at all**,
  and run on every push: the module set and its visibilities against `SIMPLE_TYPE_MODULES`; the
  file set and the `@generated` banner on each; `COVERAGE.md`'s nine simple-type counts against the
  committed module files and its child-order rows against `CHILD_ORDER_SCHEMAS`; and the two
  hand-written curation lists (`src/drawingml.rs`, `src/presentationml.rs`) against the generated
  items they re-export, in both directions — nothing failed before when the generator emitted an
  item that never reached them.

  Every one was made to fail with a **reachable** mutation, and the first attempt was not: renaming
  a generated enum broke the build instead, which proves the compiler catches that case and not
  this one. The mutation that does prove it is a one-word edit to a doc comment.
- **`cargo run -p xtask -- codegen --check`** — the same comparison as a command. `codegen::run` and
  `codegen::check` are now two consumers of one `codegen::artefacts`, which renders every artefact
  without writing anything. `codegen` moved into `xtask`'s library target for the same reason
  `validation` did: an integration test cannot see a binary's modules, and this suite is written
  against the generator's tables rather than a text rendering of them.
- **A liveness check on `UNCOVERED_SCHEMAS`, closing MJXOFF-88 §9 B10.** That table writes prose
  straight into `COVERAGE.md`, a shipped document, and the only things checked about a row were that
  its stem exists and that no stem appears twice — nothing failed when a row's claim stopped being
  true. `check_uncovered_schemas_are_live` enforces the rule the table's own doc comment already
  stated and nothing tested: a schema covered in both tables has no row, a note is written only for
  the column that needs one, and a live note never opens with `generated`. It found **three dead
  rows** (`pml`, `wml`, `dml-main`, all covered in both tables since Phases C and D) and **one dead
  note** — `dml-chart`'s child-order note said `generated — every complex type` about a column
  computed elsewhere, and would have been printed verbatim into `COVERAGE.md` the moment `dml-chart`
  left `CHILD_ORDER_SCHEMAS`. `COVERAGE.md` is byte-identical after the removals, which is what a
  dead row means.
- **`ALL_TABLES` in the generated child-order module**, and with it the end of a structurally blind
  sweep. The four suites in `crates/mjx-ooxml-types/src/child_order.rs` that check rank order, the
  unordered-type safety property and the content-model census each opened with a literal
  `[&DML_MAIN_TYPES[..], &PML_TYPES[..], &DML_CHART_TYPES[..]]`, written when those were the only
  three tables. Six schemas joined afterwards and none joined that list, so **819 of the 1,335
  complex types went unchecked while every test stayed green** — including
  `no_unordered_type_is_given_a_false_order`, whose own comment says a table that ranked a choice's
  branches would *"fault conforming markup"*. The census's message said *no `xsd:all`*, which was
  true of its three tables and false of the corpus: `CT_DocPartPr` in `wml.xsd` is one. The roster
  is generated, so the next schema to join `CHILD_ORDER_SCHEMAS` joins the sweep with it, and the
  census is now **806 sequences, 58 choices, one `xsd:all`, 470 empty**. The safety property holds
  across all 1,335 — the exposure was real, the outcome is clean.

  **The wider class is MJXOFF-225, filed rather than absorbed here**: other tests that sweep a
  hand-written list of a population a generator produces more of. This unit closed the one instance
  it stood on and did not become a coverage programme.
- **A gate on the naming convention's audit trail.**
  `every_curated_enumeration_cites_the_spec_section_its_names_came_from` requires each of the 110
  enumerations whose variant names are curated to have at least one row sitting under a comment
  naming the ECMA-376 section the names were read out of. `CLAUDE.md` requires a name that is not
  inferable from its token to be *sourced from the prose, never guessed*, and a name with no
  citation is indistinguishable from a guess. Six enumerations had none.
- **Six guide pages** under `crates/mjx-ooxml-types/docs/guide/`, reachable from
  `docs/api/README.md`, plus `mjx_ooxml_types::guide`.

### Fixed

- **The adjustment-bound closure is 334 guides, not 335.** `xtask/src/codegen/geometry.rs`'s header
  had said 335 since it was written, and the figure was repeated into a ticket from there. The size
  is now **derived into the generated table's own doc comment** rather than restated in a header
  that cannot fail. The file's own total, 3,923, was right, and
  `crates/mjx-dml/tests/guide_formula.rs` asserts it by walking the addendum.
- **Six wrong ECMA-376 section citations in the DrawingML naming block**, found by checking every
  `§` in `xtask/src/codegen/spec.rs` against the section titles of ECMA-376 Part 1:
  `ST_PenAlignment` §20.1.10.40→.39, `ST_PresetLineDashVal` §20.1.10.48→.49, `ST_PresetShadowVal`
  §20.1.10.50→.52, `ST_TextHorzOverflowType` §20.1.10.62→.69, `ST_LightRigDirection`
  §20.1.10.31→.29, `ST_LightRigType` §20.1.10.32→.30. Every citation outside that block checked out;
  the seven apparent SmartArt mismatches were an artefact of the audit script reading multi-citation
  lines, not defects.
- **`ST_PresetShadowVal`'s justification was factually wrong.** The comment beside `Shadow1` …
  `Shadow20` said the tokens have *"no semantic name"*; §20.1.10.52 names all twenty (`shdw1` is
  *Top Left Drop Shadow*, `shdw11` *Back Left Long Perspective Shadow*). The names are unchanged —
  renaming twenty generated variants is an API break — and the divergence is now recorded where a
  reader meets it.

### Recorded, not fixed

- **`ST_SchemeColorVal`'s `phClr` is `PlaceholderColor`; §20.1.10.54 titles it *Style Color*** and
  describes it as *"a color used in theme definitions which means to use the color of the style"*.
  `PlaceholderColor` reads the `ph` as *placeholder*, which is a guess the section does not support.
  An audit of all 743 variant overrides against the Part 1 prose found no third divergence that is
  not either deliberate and documented (`MYD`, `axisPage` — where the published friendly-name column
  is itself wrong) or a plain expansion of the published name.
- Both of the above, and the four things nothing here checks at all, are in
  `crates/mjx-ooxml-types/docs/guide/what_to_distrust.md`.

## [0.0.140] - 2026-09-08

### The upper shared markup, audited then documented (MJXOFF-221, G10)

**Three crates, one rank, one guide set — and a content-type predicate that had been re-making
MJXOFF-114's defect one crate further up.** `mjx-chart`, `mjx-omml` and `mjx-vml` are the whole of
layering rank 2.2: the markup that sits *on top of* DrawingML and SpreadsheetML rather than beside
it. None of them had a guide, and the audit that preceded one found two live instances of this
project's signature failure — a guard written as a string literal that quietly matches nothing.

### Fixed

- **`mjx_vml::is_vml_content_type` now folds case and trims media-type parameters.** It was
  `content_type == CONTENT_TYPE_VML` — an exact comparison against Office's own capitalisation —
  while ECMA-376 Part 2 §10.1.2.3 compares a media type case-insensitively. This is MJXOFF-114's
  defect one crate up: there, `mjx-opc`'s exception list of suffix-less XML content types carried
  `…vmlDrawing` in Office's spelling while `is_xml_content_type` folded its argument, so the entry
  matched nothing and every authored `.vml` part sat outside `Package::validate` from the day the
  list was written. That fix folded `mjx-opc` and not this, so the two halves have disagreed since:
  a lower-cased spelling counted as XML down there and as *not VML* up here. On a file this library
  never wrote, `Presentation::vml_part_names` enumerated nothing, and `check_is_vml` and
  `read_vml_document` refused a part that is a VML drawing with `PartIsNotVmlDrawing`.
- **`mjx-schema-gate` had two copies of `is_xml_content_type`**, in `inspect.rs` and `order.rs`, and
  both matched `ends_with("vmlDrawing")` case-sensitively. That predicate decides whether the gate
  looks at a part *at all*, so an unrecognised spelling is a part nobody validates and a gate that
  stays green — MJXOFF-88 §7's shape reached through a string literal rather than a missing table
  row. `order.rs` now calls the one rule instead of restating it, and the rule folds.

Both fixes have unit tests that fail against the old bodies.

### Added

- **`xtask/tests/upper_markup_ledger.rs`** — MJXOFF-218's third and last instalment, and the
  question is neither of the two already answered. `mjx-dml`'s ledger asks what eight hand-written
  `FromXml`/`ToXml` pairs lose; `mjx-sml`'s found that did not transfer and followed the risk into
  the rebuilder behind a delegation. Here the reason is arithmetic: **the three crates hold zero
  hand-written impls and zero rebuilders between them.** All **62 element declarations** reach XML
  through one of two generic mechanisms — `#[derive(FromXml, ToXml)]`, or the crate's own
  `fidelity_*!` macro, one body each. So there is no body to audit, and the live risk is *which
  mechanism a type is on*: nothing before this file would have noticed a type going on neither,
  which is the hole `mjx_dml::Picture::to_xml` came through (MJXOFF-216).

  Five checks, and the fifth earned its place. A zero cannot carry an anti-vacuity floor, so **the
  impl scanner is calibrated against `mjx-dml` (13) and `mjx-sml` (64)**. With the scanner
  deliberately mistyped, the rank-2.2 check still reported *0 hand-written impls, 0 on the ledger*
  and passed; only the calibration noticed. Every arm was made to fail with a reachable mutation and
  the register is in the file header.

  One file for three crates, hosted by `xtask`: unlike `mjx-dml` and `mjx-sml`, whose idioms differ
  per crate, the expensive half here is shared, and `xtask` is where every cross-crate structural
  gate already lives and is outside the layering graph.
- **Eight guide pages**, reachable from `docs/api/README.md`: six under
  `crates/mjx-chart/docs/guide/`, plus `crates/mjx-omml/docs/office_math.md` and
  `crates/mjx-vml/docs/legacy_vml.md`, hosted by their own crates because a same-rank crate cannot be
  depended on and so cannot be linked into — the same reason `mjx_opc::guide`'s MCE page lives in
  `mjx-mce`. They give `doc_gate` 35 path mentions and 51 crate-qualified symbol references, and
  carry five compiled doctests.

  The through-line is more specific than the rank table: **only `mjx-chart` uses the height.** It
  reaches `mjx-sml` (2.1) for the workbook a chart embeds and `mjx-dml` (2.0) for everything a chart
  draws with, while `mjx-omml` and `mjx-vml` declare no dependency on either and sit at 2.2 because
  a rank is a ceiling on what a crate *may* reach, not a claim about what it does. And none of the
  three may see the other two, which decides what they can model.
- **VML's weaker guarantee is stated where a caller meets it** — in the index, in the fidelity page,
  in `crates/mjx-vml/docs/legacy_vml.md`, and now at `mjx-vml`'s own crate root. `vml-main.xsd`
  cannot compile without an `xml.xsd` ECMA omits and a `.vml` part's root is a bare `<xml>` in no
  namespace, so `mjx-schema-gate` files it under `ForeignMarkupKey::NoNamespace` in
  `PRESERVED_FOREIGN_MARKUP` and **the round trip is the only real check there is.** Reading and
  re-emitting is as safe here as anywhere; authoring or editing carries a risk the other two crates
  do not, because a wrongly ordered shape would reach Office before it reached CI.
- `xtask/src/validation/ingest.rs` records the **third instance** of MJXOFF-196's mandatory-wildcard
  shape: `vml-officeDrawing.xsd:175` declares `CT_EquationXml` as
  `<xsd:sequence><xsd:any namespace="##any"/></xsd:sequence>`, again with no `minOccurs`. It cannot
  fire — nothing models the type, and no VML part is validated at all — so it is not a fourth defect
  but the third address a fix has to visit.

### Two counts this ticket had wrong

- **`ReferenceProblem` has eight variants, not nine.** MJXOFF-221's own description says the embedded
  workbook patcher "refuses nine reference shapes by name"; `CHANGELOG.md`'s MJXOFF-208 entry lists
  six of them in prose. The declaration has eight, and
  `crates/mjx-chart/docs/guide/the_embedded_workbook.md` tables them row for row.
- **`mjx-sml` holds 6 hand-written `FromXml` impls and 58 `ToXml`, not 5 and 57.** MJXOFF-218's
  census missed `crates/mjx-sml/src/font/color.rs`'s `ColorElement`, whose impl is written with a
  qualified trait path. The new ledger's scanner admits the qualified spelling, which is what puts
  its calibration floor above 50.

## [0.0.139] - 2026-09-08

### SpreadsheetML's guide, and the count that had been wrong four times (MJXOFF-220, G9)

**The largest crate in the workspace had no guide, two design notes sitting beside it, and the one
figure this programme has got wrong most often.** `CT_Worksheet` is the widest content model in
`sml.xsd` at 39 slots, and its modelled/held split had been written down as 25/14, then 31/8, then
34/5, then 35/4 across four children — **three of the four wrong when they were written**. MJXOFF-88
§9 B2 named the structural cause: `styles/stylesheet.rs` asserted that its modelled and held slots
add up to the generated table's length and `worksheet/frame.rs` asserted nothing of the kind.

**The split is now derived from the read path, and the prose is held to it.** Three tests in
`crates/mjx-sml/src/worksheet/frame.rs` read a worksheet holding one of every slot the generated
table names, ask `read_slot` which of them it typed — **39 slots, 35 modelled, 4 held**
(`phoneticPr`, `legacyDrawingHF`, `drawingHF`, `extLst`) — hold the module's rank table to that
answer row by row, and hold the sentences around it to the same answer, which is where the last
stale figure actually was: the heading said *thirty-four modelled, five held* over a table listing
thirty-five and four.

The same class was everywhere it could be. `sheets/frame.rs` gains the derivation over the three
sheet kinds and found `dialogsheet.rs` claiming *eleven* of its sixteen slots were modelled where the
reader types ten, and naming *five held verbatim* directly above a list of six — chartsheet is
14/10/4, dialogsheet 16/10/6, macrosheet 27/20/7. `workbook/mod.rs`'s test walked ranks 0..18 and
checked each was *rankable*, a property of the generated table that would have passed unchanged had
the reader stopped modelling one; `stylesheet.rs`'s compared two hand-written lists. Both now read a
part and ask the reader. `crates/mjx-sml/tests/worksheet_spine.rs` held a test named *the thirty-nine
slots are accounted for* whose documentation claimed the modelled set "is exactly the thirteen this
workspace claims" and whose body only checked that each of thirteen frozen names is *a* slot of
`CT_Worksheet` — a test that read as proof of the thing that had gone wrong. **Four shipped artefacts
stated the split and three were stale**: `mjx-xlsx`'s `fidelity_and_the_part_graph.md` said 34/5,
`reading_and_editing_cells.md` 18/21 and `the_sheet_grid.md` 31/8.

**`mjx-sml`'s half of MJXOFF-218**, and the answer is not `mjx-dml`'s. The census in that ticket
counted `^impl (From|To)Xml for <Type>` and so missed `impl mjx_ooxml_core::ToXml for ColorElement`:
the figures are **6 `FromXml`, 58 `ToXml` over 58 distinct types, 52 `ToXml`-only** — the last being
the one number the ticket had right, because both of its inputs were one too low. Fifty-seven of the
fifty-eight writers are byte-identical (`{ self.as_raw_element() }`), so each would be a "dispatcher"
under `mjx-dml`'s vocabulary and each would pass a dispatcher's check trivially and forever. The risk
is one hop away, in the inherent rebuilder — which exists because a worksheet's writer takes `&self`
and has no `&mut Interner` to lend, the same property that lets `sheetData` be a packed store — so
`crates/mjx-sml/tests/serialization_ledger.rs` follows the delegation: every hand-written writer must
*be* it or carry a row, all **60 rebuilders** must rebuild from `self.name`, `self.attributes` and
`self.empty`, the 4 content-enum dispatchers must construct no element, and all 6 hand-written
readers are on a ledger with reasons. **Nothing here loses content**; what is new is that the
sixty-first cannot arrive unnoticed. Replacing `&self.attributes` with a fresh vector in one
rebuilder leaves every test in the crate green while destroying `@count` and every foreign attribute,
because a rebuilder is only reached once a slot has given up its verbatim bytes.

**`Color::from_opaque_rgb` prefixed `FF` unconditionally** (MJXOFF-88 §9 A5 defect 1, MJXOFF-198 §6
F6), so `"FFFF0000"` — the spelling `Color::rgb`'s own documentation gives — became a ten-character
`@rgb` that `sml.xsd` rejects, reached from `PatternFillSpec::solid` and three more convenience
constructors and projected onto both bindings. Decided as a **normalisation, not validation**: the
signature cannot become fallible without breaking every caller that already works, and `Color::rgb`
is a public field that could not carry the invariant anyway. Six digits behave exactly as before,
eight are taken as they stand, a leading `#` is dropped, and anything else stays the caller's
contract and is now documented as such. No gate could see it because the schema gate validates the
markup a test authored and every test handed it six digits — so the new gate is over the *authoring
vocabulary*: `every_authored_colour_is_a_valid_unsigned_int_hex` builds 35 colours through the five
entry points and holds each `@rgb` to eight hexadecimal digits.

**Then the guide**: six pages under `crates/mjx-sml/docs/guide/`, reachable from `docs/api/README.md`
and from `mjx_sml::guide`. `docs/CELL_STORE.md` and `docs/SHARED_STRINGS.md` **moved into the set**
rather than being left beside it, so MJXOFF-95's and MJXOFF-97's records are pages three and four
rather than orphans. The pages give the documentation gate 141 crate-qualified symbol references and
47 repository-path mentions over 9 paths it had not been shown before.

Two things recorded rather than changed. **`threadedComments` and `persons` appear nowhere in this
repository** — a 2018 Microsoft extension absent from ECMA-376, so nothing generated from the schema
names them — and every workbook a modern Excel saves with a comment carries both; they round-trip as
ordinary parts, and `Workbook::sheet_comments` reports the legacy shadow copy, which is the text
without the thread. That reasoning existed only in MJXOFF-88 §9 B12 and is now on the fidelity page.
And F6's second half — every colour convenience takes a hex literal while `Color::from_theme` has
none beside it, so **the theme-following path is the one nobody takes** — is answered without new
API: the specs' fields are public, the one-line theme literal is now on `from_theme` itself and in
the guide, and a Rust-only `solid_theme` would be a surface two of the three languages could not use.

## [0.0.138] - 2026-09-08

### `mjx-dml`'s guide, and the six hand-written pairs the audit for it found (MJXOFF-217, G8)

**The largest crate in the workspace had no guide at all, and one countable question outranked
writing one.** MJXOFF-216 found `mjx_dml::Picture` and `PictureNonVisual` destroying every attribute,
every unmodelled child and the element's own prefix, and asked how many of this crate's hand-written
`FromXml`/`ToXml` pairs did the same. **Of the eight pairs `mjx-dml` held at 0.0.137, four lost
content and two more lost the self-closing flag.**

The four that lost content are `Picture`, `PictureNonVisual`, `Graphic` (any child beside the
`a:graphicData`) and `GraphicData` (its own name and prefix — so a producer that bound
DrawingML-main to any prefix but `a:` had it rewritten — plus any node beside a typed `pic:pic`
payload). All four move **onto `mjx-derive`**, which is the point rather than a convenience: the
derive's `#[xml(children, child(..))]` arm emits the `Raw` fallthrough unconditionally, so they are
now inside the same codegen guarantee that one test file backs for every derived type at once, and an
unmodelled child keeps its *position* among its modelled siblings. The two that lost the self-closing
flag — `wordprocessing_drawing::Inline` and its `Anchor` — take the formula
`fidelity_element_impls!` already used. Every one of the six is pinned by a case in
`crates/mjx-dml/tests/in_context_roundtrip.rs` that fails against 0.0.137.

All six were **latent**: no shipped write path reaches a parsed value of any of these types, because
every `mjx_dml::Graphic` this workspace writes is freshly built for a chart or a picture. Latent is
not fixed — MJXOFF-216 states plainly that it goes live the day anyone adds a picture-editing method.

**The class, not the instance.** `crates/mjx-dml/tests/serialization_ledger.rs` reads the crate's own
sources and requires every hand-written `FromXml`/`ToXml` to be on a ledger with an idiom and a
reason — **and checks the idiom against the impl body**, so a row claiming to preserve everything
while handing `RawElement::rebuilt` a fresh `Vec::new()` fails, which is exactly the shape
`Picture::to_xml` had. It reports 41 derived types, 57 via the shared macro and 13 hand-written impls
over 9 types. `mjx-docx`'s 158 hand-written pairs and `mjx-sml`'s 57 types are outside it, raised as
MJXOFF-218.

**Then the guide**: six pages under `crates/mjx-dml/docs/guide/`, reachable from `docs/api/README.md`
and from `mjx_dml::guide`. Written for a caller who has a shape and wants it filled, outlined,
positioned or coloured — not a tour of a thousand items — around the four facts that explain the
crate: every type is a view over one element, an interner-bound value has an interner-free `*Spec`
twin, `spec()` reads while `to_*()` builds fresh and `apply()` merges, and the measures name their
own units. They give the documentation gate 150 crate-qualified symbol references over 114 distinct
symbols and 37 repository-path mentions.

Three things the audit found and did **not** change, each recorded with its reason:
`mjx_dml::ColorSpec` still carries no colour transform, so nothing can author a `comp`/`gray`/
`gamma`/`invGamma` and validation entry `V-PPTX-02.4` still has no artefact — a write-path gap whose
fix is a code change; `teardrop` and `sun` stay `ShapeGeometry::Unmodeled`, the Phase A deferral for
spec-ambiguity; and `mjx-pptx` keeps navigating `p:spPr` by hand rather than through
`mjx_dml::ShapeProperties`.

## [0.0.137] - 2026-09-08

### The packaging tier's guide, and the four fidelity claims answered (MJXOFF-215, G7)

**`mjx-opc`, `mjx-mce`, `mjx-xml` and `mjx-ooxml-core` are where this project's promise is actually
implemented, and none of the four had a narrative guide.** Six pages now, over the four crates rather
than one apiece, because the mechanism is spread across all of them and no one of them can be read
alone. Five are hosted by `mjx-opc` — the only crate in the tier that can see two of the other three
— and the sixth by `mjx-mce`, which is the same layering rank and therefore unreachable from it. That
is the layering rule showing through the documentation rather than a gap in it.

**The page that had to be written is `removing_a_part.md`.** Four methods on `Package` remove a
part, they have genuinely different blast radii, and until now the difference lived only in prose
MJXOFF-209 had to write after the fact. A caller choosing wrongly deletes a producer's content: that
is exactly what MJXOFF-209 *was*, three `Document` edits finishing with the package-wide sweep and an
edit about a header deleting an unrelated image.

**`CLAUDE.md`'s four fidelity rules were claims nobody had checked as claims.** Each now has an
answer, and two of the four needed correcting.

*"Part-level laziness"* is **parse** laziness. `Package::open` inflates every ZIP entry eagerly with
`read_to_end`; what is deferred is the XML parse. The distinction is why a small archive can expand
without bound (MJXOFF-154, still open), and it was written down nowhere.

*"Every modeled complex type carries `extra: Vec<RawNode>`"* names the rarest of three idioms. The
guarantee is stronger than the sentence — `mjx-derive`'s codegen *generates* the `Raw` fallthrough, so
one test failure reaches every derived type at once — but a reader who grepped for `extra` would
conclude `mjx-dml` had no bucket at all, when what it has preserves strictly more. "Every" also has
exceptions, and one of them is a defect: `mjx_dml::Picture` and `mjx_dml::PictureNonVisual`
hand-write `FromXml`/`ToXml` with no raw remainder and an empty attribute vector, and were **proved**
to destroy a producer's attribute and a foreign child. Latent rather than live — no shipped write
path reaches them — and filed as **MJXOFF-216**, with the ledger question the class raises.

*"MCE is handled in `mjx-mce`"* is true of resolution. Two format crates walk `mc:AlternateContent`
by hand instead, defensibly, and `mjx_mce::resolve` has exactly one shipped call site.

*The round-trip contract itself* is the best-enforced of the four, at three granularities — container,
tree and edit — and the page says which test holds each and what none of them can see.

**Also: seven stale claims repaired.** The worst was on `Package::remove_part`, which said the graph
operation was "left to a later phase" while its three graph-aware siblings sat below it in the same
file. The rest were `mjx-ooxml-core` describing a typed model, a derive and an attribute-typing
variant that had all shipped, and promising an arena that was deliberately never built — the reasoning
for which is in `crates/mjx-sml/docs/CELL_STORE.md` and is now recorded as a decision rather than a
gap. And `CLAUDE.md` gained the `#[xml(text)]` escaping gap that two `mjx-sml` source comments have
cited it for since MJXOFF-114 without it ever being there.

## [0.0.136] - 2026-09-07

### The facade's guide, and the audit that had to come first (MJXOFF-214, G6)

**Phase G's first documentation unit, and the audit still led.** Five units preceded it and none
wrote documentation, because auditing first kept finding defects the planned guides would have
described as working. This one found no defect that destroys content — it found five claims the
facade makes about itself that were false, and one class of claim that cannot stay true.

**Four counts had rotted, all in the same way.** `crates/mjx-ooxml/src/deck.rs` said *sixteen*
`Presentation` methods were deliberately absent when the difference was **eighteen** — the two it had
never named being `blank_with_properties` and `from_package`. `crates/mjx-ooxml/src/workbook.rs` said
`mjx_xlsx::Workbook` carried *roughly seventy* public methods when it carried **165**, and filed
**nine** entries under *the closure-taking markup doors* that take no closure at all
(`worksheet_markup`, `write_worksheet_markup`, `sheet_formatting` and six siblings hand back an
owned but interner-bound model, which is a different reason with a different consequence). Both
module docs quoted an error-variant count that had grown — 65 → **67** for `PptxError`, 35 → **41**
for `DocxError`. And `mjx_sml::CellReference`'s own doc comment was headed *"the constructors take
`(column, row)`, and everything else in the workspace takes `(row, column)`"*, which is false: five
other public sites take the column first, four of them on this facade
(`Workbook::add_chart`, `add_range_chart`, `add_one_cell_anchored_picture`,
`add_two_cell_anchored_picture`), every one for the same good reason — an `xdr` marker is
`<xdr:col><xdr:colOff><xdr:row><xdr:rowOff>`, so its offsets interleave with its indices — and none
of them said so anywhere.

**The cure is not a fresher number.** `xtask/tests/facade_curation.rs` is new: it walks the inherent
`pub fn` items of `Deck`/`Document`/`Workbook` and of the three types below them, and requires the
difference to equal a written ledger **in both directions**, with a stated reason on every entry
from a closed set of eight. A method added to `mjx_pptx::Presentation` and not projected fails there
until somebody decides which it is — projected, renamed, or deliberately left behind. The counts came
out of the prose; the list went into the gate. *A number in prose can only be right on the day it is
written; a list can be compared.*

**One behaviour was undocumented and is now on the method itself.** `Workbook::write_cells` into a
cell that carries a formula keeps the `<f>` and replaces only the cached `<v>` — so the written value
does not survive Excel's next recalculation. Verified by running it, not by reading the code. The
decision is right (dropping the `<f>` would destroy a formula the caller did not name, in a file they
opened to change a number) and it was written down nowhere.

**Then the guide: seven pages under `crates/mjx-ooxml/docs/guide/`**, matching the shape of the three
existing sets and deliberately not repeating any of them. Those three describe one format each; this
one describes the surface all three are reached through — [Opening and saving], [Addressing], [One
vocabulary, three surfaces], [Errors], [The curated surface], [Fidelity and the known gaps]. Every
snippet is a compiled doctest that asserts on a value it computed, and the set gives `doc_gate`
**+86 crate-qualified symbol references and +30 path mentions over 7 new distinct paths** to check —
which is the point of writing a page that names things: *a guide that names no symbol cannot go
stale, and cannot be checked.*

**Recorded, not fixed** — each is a judgement call rather than a small correction, and each is in
the ticket: twenty-five chart methods spell the same parameter `series_idx`/`point_idx` on `Deck`
and `Document` and `series`/`point` on `Workbook` (a keyword-visible difference in Python);
twenty-three `Document` methods still take `impl Into<BlockPath>` against the facade's own stated
rule; `table_dimensions`/`cell_span`/`merged_cell_anchor` return an anonymous `(u32, u32)` that
Python gets as a tuple and TypeScript as a `CellExtent` class; `blank_with_properties` exists on all
three model types and on none of the three facade surfaces, so no caller in any language can set a
document's title or author.

[Opening and saving]: crates/mjx-ooxml/docs/guide/opening_and_saving.md
[Addressing]: crates/mjx-ooxml/docs/guide/addressing.md
[One vocabulary, three surfaces]: crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md
[Errors]: crates/mjx-ooxml/docs/guide/errors.md
[The curated surface]: crates/mjx-ooxml/docs/guide/the_curated_surface.md
[Fidelity and the known gaps]: crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md

## [0.0.135] - 2026-09-07

### The preservation gate: every fixture × every mutating API, asserting what changed and that nothing else did (MJXOFF-210, G5)

**The rule this enforces:** *an edit changes what the caller asked to change, and nothing else* —
the sharp form of Phase G's standing design rule, *supply a default only in the absence of the
user's own, never in place of it*.

Four tests in this workspace already stated that property, each for one method on one fixture. Every
one of them was hand-written and per-feature, so **a method added later sat outside all of them by
default** — which is how both destructive defects of Phase G got in. `crates/mjx-ooxml/tests/preservation/`
now states it once, over the whole surface: **8,941 fixture × method pairs**, every one of the
committed corpus's 54 packages against every public `&mut self` method of `Deck`, `Document` and
`Workbook`.

**The method list is derived from the facade's own source, not written down.** A hand-maintained
list of methods is the `const FIXTURES` failure one level up, and it fails the same way: silently,
on the next method added. `enumeration.rs` reads `crates/mjx-ooxml/src/{deck,document,workbook}` and
the registry is compared against it **in both directions** — a method the facade grows and the suite
does not register fails, and so does a registered case naming a method the facade no longer has.
The predicate is `&mut self` rather than "looks like a mutator", which over-selects heavily and
deliberately: more than half of the 452 are *readers* that need `&mut self` only because parts are
parsed lazily, and a reader that left the part it read dirty would rewrite a part the caller merely
looked at. They declare `NOTHING`, which is the strongest assertion in the file.

**Three more both-directions comparisons carry the anti-vacuity weight**, because a floor over a
total says the extractor is alive rather than complete: the corpus against the fixtures the sweep
visited, `mjx_fixtures::PACKAGE_EXTENSIONS` against the surfaces that have a driver, and the
`NEVER_EXERCISED` register — eleven methods the committed corpus cannot make do their job, each
naming the fixture content that would retire it — against what the sweep actually saw.

**A case may prepare the fixture first.** A `clear_*` on a shape with nothing to clear, or a
`remove_chart_trendlines` on a chart with no trendline, succeeds and changes nothing, and a
declaration is only checked in both directions when something happened. So a case may name an edit
made *before* the snapshot, saved and reopened, whose effect lands in the `before` bytes and never in
the diff. That took the methods proving nothing from 54 to 11 and the applied pairs from 1,832 to
2,492.

**Proved able to fail by re-introducing all three defects the unit locks in**, each pasted red and
restored by re-editing: restoring MJXOFF-208's workbook regeneration reddens on the producer's
`docProps` vanishing from the embedded package (the *removed* direction); removing MJXOFF-209's
percent-decode reddens 46 pairs on `percent_encoded_targets.docx` with a package `save()` will not
accept (the save guard); pointing MJXOFF-200's theme writer at the package unconditionally reddens 22
pairs across 13 fixtures on a changed `theme1.xml` (the *changed* direction).

### Fixed

- **A chart refused by a tab that cannot hold one no longer leaves a theme part behind.**
  `mjx_xlsx`'s `write_chart` authored the theme (MJXOFF-200's fix, three commits old) *before* the
  step that resolves the drawing part, so `add_chart` aimed at a dialogsheet refused and had already
  changed the package. The theme is now written last, after every step that can refuse.
  `mjx_docx::Document::add_chart` had the same latent ordering and was moved with it.
- **`crates/mjx-docx/tests/charts.rs`'s add-a-chart isolation case was structurally blind.** It
  iterated the *before* map alone, so it could not see a part the edit added — MJXOFF-198 §5 named
  it as a test that reads as proof and is not one. It now asserts the added and removed sets too, and
  the general form of the property is the new sweep.

### Found, ticketed, and registered rather than fixed

Two defects the gate found on its first sweep. Both are recorded in `KNOWN_DEFECTS`, which is
compared against the sweep **in both directions**: neither can be forgotten, and neither fix can land
without deleting its entry.

- **MJXOFF-212** — `Deck::remove_slide` on a deck where another slide hyperlinks to the removed one
  leaves that relationship pointing at nothing, and `save()` then refuses: the file can never be
  written back. The fix needs a decision about what becomes of the hyperlink.
- **MJXOFF-213** — `Workbook::add_cell_comment` aimed at a dialogsheet refuses only after the
  comments part and the sheet's relationship have been written.

## [0.0.134] - 2026-09-07

### Charts authored into Word and Excel had no data series: no theme part was ever written (MJXOFF-200, G4)

**Found by a person opening a file.** The human validation pass opened `v-docx-04-authored.docx` and
reported that the chart showed no data. It showed everything else: title, axis titles, category
labels, value-axis tick labels, legend *text* and the plot frame. **No bars, and no colour keys in
the legend.** The value axis auto-scaled correctly from the cached maximum, so the consumer was
reading the series fine — the failure was in painting.

**Cause: this library never wrote a theme part for Word or Excel.** Measured across the twenty
validation artefacts at 0.0.133: 8 of 8 `.pptx` carried one, 0 of 6 `.docx` and 0 of 6 `.xlsx` did. A
chart series this library authors carries **no `c:spPr`**, deliberately, so that the host document's
brand wins — which means its fill comes from the theme's `accent1…accent6`. With no theme part those
resolve to nothing and the series is painted with no colour. PowerPoint escaped by accident: a
`.pptx` always has a theme because every slide master requires one, so the identical chart markup
rendered blue and orange bars there and nothing in Word.

**The fix is a theme, and the constraint on it is the whole difficulty.** Per-series `spPr` literals
were rejected explicitly: they would make our own artefacts look right while overriding the palette
of whoever opens the file. And a writer that emitted `word/theme/theme1.xml` unconditionally would
have destroyed the branding of every real document this library opens and re-saves — an
invisible-chart bug turned into a corrupt-the-customer's-file bug, and every gate here would have
stayed green through it. So a theme is authored **only into a package that carries none**, and "has a
theme" is decided by content type over the whole package rather than by the relationship this crate
happens to classify.

### Added

- **`mjx_dml::default_theme_xml`** — the one `a:theme` this workspace authors, moved down from
  `mjx-pptx`'s `blank` so Word, Excel and PowerPoint share the same bytes rather than writing the
  markup out three times. Every deck is byte-identical to 0.0.133's.
- **`mjx_sml::write::WorkbookPackage` writes `xl/theme/theme1.xml`**, so `Workbook::blank` and every
  chart's embedded workbook carry one.
- **`mjx-schema-gate`'s reference-resolution gate** — for a package we authored, every reference its
  own content makes resolves to something present: relationship ids and targets, DrawingML scheme
  colours and theme fonts, WordprocessingML theme colours, theme fonts, style ids and numbering ids,
  SpreadsheetML theme colour indices, the font scheme and every index into a `styles.xml` table. This
  is the class MJXOFF-198 §5 records as having no gate at all: **every other check in this repository
  asks whether the bytes we wrote are the bytes we meant, and none asks whether a part we did not
  write should have existed.**

  The rule that matters is the one with nothing in the markup to look for: a `c:ser` with **no**
  `c:spPr` states no colour, it *defers* to `accent1…accent6` cycled by series order. A gate that
  searched for `a:schemeClr` would have stayed green through this entire defect.

### Fixed

- **A chart added to a `.docx` or `.xlsx` gains a theme when the package has none**, so its series
  resolve a colour. A document or workbook that arrives with a theme keeps it byte for byte.
- **The authored workbook's font 0 follows the theme** — `<color theme="1"/>` and
  `<scheme val="minor"/>` beside the literal `Calibri`, which is what Excel writes. This was API-audit
  finding F5: inert while there was no theme to reference, live the moment there is one, so it is
  fixed in the same change.

### Documentation

- `crates/mjx-xlsx/docs/guide/deliberate_limitations.md` recorded the missing theme as harmless —
  *"a `theme`-referencing colour in a file you opened resolves against the theme that file carries"*.
  True of a file you opened and **false of the authored case the row was about**. The row is gone and
  the section says why, because a limitations page is a place a rendering defect can hide reading as
  a nicety. Four more pages said the same thing and are corrected with it.

## [0.0.133] - 2026-09-07

### A relationship target's percent-encoding is decoded, and three Word edits stop sweeping the package (MJXOFF-209, G3)

**A legal OOXML file this library could open but refused to write back, and — through
`save_unchecked` — deleted parts from.** ECMA-376 Part 2 §9.1.1 makes a part name an IRI: a character
outside the `pchar` set is written percent-encoded in a relationship `Target` and in a content-type
`Override`'s `PartName`, and the part it names is the *decoded* form. Real producers write these;
LibreOffice 25.8 writes `Target=".../my%20image.png"` for an image whose file name holds a space.
There was no percent-decoding anywhere in `mjx-opc`.

So `../media/image%20one.png` resolved to `/word/media/image%20one.png`, which never matched the ZIP
entry `word/media/image one.png`. `Package::validate` reported `RelationshipTargetMissing` and
`Package::save` refused the file outright; `remove_unreferenced_parts` does not follow an edge it
cannot resolve, so the real part was never marked reachable and was swept as an orphan.

**The difficulty is not the decoding, it is not re-encoding.** Decode-then-encode is not the
identity: `%2520` and `%20`, `%5f` and `%5F`, `%75` and `u` decode alike and re-encode differently. A
library that normalised on write would change the bytes of `.rels` and `[Content_Types].xml` parts in
files nobody asked it to touch — a far wider fidelity regression than the bug it fixed.

### Fixed

- **`PartName::resolve` / `resolve_from_root` decode the target**, and `ContentTypes::parse` decodes
  an `Override`'s `PartName`. Decoding happens on the way *into* a `PartName` and nowhere else:
  `Relationship::target` keeps the producer's exact text, and an unedited control part re-emits
  verbatim, so a producer's own spelling survives a round trip byte for byte.
- **Dot segments are folded before decoding** (RFC 3986 §5.2.4), so `%2E%2E` is an ordinary segment
  named `..` rather than a climb above the package root, and the split into segments happens before
  any escape can become a separator.
- **A malformed escape is passed through, not refused.** `%ZZ`, a truncated `%4` and a trailing `%`
  are all things a non-conforming producer writes — most often a file name that genuinely holds a `%`
  and was never encoded (`100% margin.png`), which still resolves. Nothing in the decoder can panic
  on any byte string, and a decoding that is not valid UTF-8 leaves the segment as written. A segment
  that decodes to text containing `/` *is* refused with `OpcError::TargetResolution`: OPC forbids an
  encoded separator because it would turn one segment into two, and naming a different part silently
  is worse than reporting that it does not resolve.
- **`remove_override_element` matches the decoded attribute**, so a rule spelled
  `/word/my%20header.xml` is found for the part `/word/my header.xml`. Without it the element would
  be left in the stream while the parsed view dropped it. It also matches the *encoded* spelling,
  because the two escaping systems in that attribute do not commute: percent-encoding runs first, so
  a part name holding `&` is written `%26` and the XML escaper never sees it, while the name itself
  still holds a bare `&` whose escaped form is `&amp;`. Matching only the escaped name would leave
  two `Override`s for one part after a second `set_content_type_override`, stale one first.
- **Three `Document` edits no longer run the package-wide sweep.** `remove_header`/`remove_footer`,
  removing the last comment, and `remove_drawing` each finished by calling
  `Package::remove_unreferenced_parts`, which deletes every orphan it can find — including one the
  *producer* left in the file. Removing a header would take an unrelated image with it. They now use
  `Package::remove_part_if_unreferenced`, scoped to the part the edit itself orphaned. `mjx-pptx`
  never had the problem: its sweep is the opt-in `Presentation::remove_unused_parts`, and these three
  were the only automatic callers in the workspace.

### Added

- **`Package::remove_part_if_unreferenced`** — `remove_part_cascading` guarded by the same reference
  check the sweep decides reachability with. The clean-up an edit performs on its own behalf, as
  distinct from the package-wide garbage collection a *caller* asks for.
- **`PartName::relative_target` and the `Override` writer percent-encode**, closing the pair: a part
  the caller named `a picture.png` produces a conforming reference that reads back as itself. Every
  name this library generates is already unreserved, so no authored package changes a byte.
- **`tests/fixtures/percent_encoded_targets.docx`** — five parts addressed through an escape, in four
  spellings (`%20`; `%2520` over a name that really holds `%20`; lowercase `%5f` against an uppercase
  `%5F` in the other control stream; a gratuitous `%75` for `u`). It joins every byte-identity corpus
  and the schema gate by being in the directory, so the no-re-encoding rule is held permanently. It
  is **hand-built** and the suite says so: authored by this library, then post-processed to rename
  five parts and spell their references as a conforming producer must. LibreOffice cannot serve as
  the producer — it renames every embedded picture to `media/imageN.png`, so it never writes an
  *internal* encoded target, though it does encode the external ones.
- Tests: `crates/mjx-opc/tests/percent_encoded_targets.rs` (resolution, `validate`/`save`, the sweep
  reaching all five, byte identity of every spelling across an edit, and the authored direction);
  `crates/mjx-docx/tests/scoped_cleanup.rs` (each of the three edits removes what it orphaned and
  leaves a part planted beforehand alone).

## [0.0.132] - 2026-09-07

### A chart data edit no longer discards the producer's embedded workbook (MJXOFF-208, G2)

**A defect that destroyed content in files a user opened, and it fired automatically.** Opening a
real `.docx`, `.pptx` or `.xlsx`, changing one chart series value and saving discarded every extra
sheet, cell format, defined name, macro and document property the chart's embedded workbook carried.
`set_chart_series_values` and `set_chart_series_categories` called `refresh_chart_workbook` for you,
and that method built a *fresh* one-sheet package with `embedded_workbook_for_chart_space` and wrote
it over the part the producer had written.

It was **documented** — the doc comment said the workbook was *"regenerated, not patched"* and named
what was lost, offering `detach_chart_workbook` as the escape. The disclosure was honest. The default
was inverted: under the project's standing rule — *supply a default only in the absence of the user's
own, never in place of it* — the preserving branch is what must happen when the caller says nothing.

**And the stated justification was wrong, not merely weak.** `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md`
called reconciling a third-party workbook with edited chart data *"a merge problem with no correct
answer"*. It is not a merge problem. The chart already states where its data lives — the `c:f` beside
each cache — so putting the new numbers there is an address lookup.

### Changed

- **A data edit patches the embedded workbook.** `Presentation::set_chart_series_values`,
  `set_chart_series_categories` and `refresh_chart_workbook`, and their `Document` and `Workbook`
  counterparts, now write the chart's data into the cells the series' own `c:f` names and touch
  nothing else. Everything else in the package — every other sheet, the stylesheet, the shared-string
  table, `docProps`, a theme — comes back byte for byte, because the parts holding it are never
  rewritten.
- **A cell that already holds its value is not written**, and a workbook in which nothing changed is
  not written back at all. Re-saving a package rewrites its ZIP container even when every part inside
  is identical, so skipping the write is what keeps a no-op refresh a no-op in the host's bytes. The
  answer stays `true` in that case: it says *this chart has an embedded workbook*, not *bytes moved*.
- **A data edit is all of it or none of it.** The workbook is worked out before the chart part is
  touched and written after it, so a reference this library will not write refuses the whole call and
  leaves both parts as they were.
- **New text is written as an inline string** (`t="inlineStr"`) rather than interned. Interning would
  mean rewriting `xl/sharedStrings.xml` as well — a second part of somebody else's file that the
  caller never named — and because unchanged labels are not written at all, a workbook's existing
  shared strings stay shared.
- `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` loses the *regenerated, not patched* non-goal;
  it is now in that page's list of what used to be there. `docs/validation/03-presentations.md`'s
  `V-PPTX-04` and `V-PPTX-08` rows say the same.

### Added

- **`mjx_chart::plan_workbook_patch` / `apply_workbook_patch`** (`crates/mjx-chart/src/embedding/patch.rs`),
  with `WorkbookPatch`, `WorkbookPatchPlan`, `ChartWorkbookError` and `ReferenceProblem`. One
  implementation for all three hosts: `mjx-chart` is rank 2.2 and reaches `mjx-sml` (2.1) and
  `mjx-opc` (1.0), and the three format crates are rank 3.0. Two functions rather than one because a
  host cannot borrow the chart's part tree and the package's bytes at once — which is also what gives
  a data edit its all-or-nothing shape.
- **`mjx_chart::embedded_workbook_part`** — the *chart part → relationship id → workbook part* walk,
  which `mjx-pptx`, `mjx-docx` and `mjx-xlsx` each carried their own copy of.
- **`regenerate_chart_workbook`** on `Presentation`, `Document`, `Workbook`, on all three facade
  types, and in both bindings (`regenerate_chart_workbook` / `regenerateChartWorkbook`). This is the
  old behaviour, under the name that says what it does: it replaces the workbook wholesale and
  **discards whatever it held**. A caller now has to ask for it.
- **`ChartAccessError::EmbeddedWorkbookNotWritable { reference, problem }`** and
  **`PptxError::ChartEmbeddedWorkbookNotWritable`**. A `c:f` naming another workbook, several sheets,
  whole columns, a rectangle, a sheet the workbook does not have, or fewer cells than the data has
  points is refused **by name** — never a quiet fall back to regenerating, which is the content loss
  the patch exists to prevent. `ReferenceProblem` is the whole list, and every entry of it is a shape
  of *reference*, decided from the text the producer wrote, never a guess about provenance. The
  facade classifies it as `ErrorCode::UnsupportedContent`, beside `ChartFillNotSupported`.
- **A point is written at its `c:pt@idx`, not at its position in the file.** A sparse cache — a
  series with a blank third value writes points `0`, `1`, `3` — would otherwise slide every later
  value one cell up somebody else's column. The old regenerator had the same flaw in its own grid.
- Tests: `crates/mjx-docx/tests/charts.rs` opens `tests/fixtures/chart_in_word.docx`, edits one
  series value and asserts the embedded workbook's part list, its styles, its string table, its
  document properties, the *other* series' column, its sheet view and its page margins all survive;
  `crates/mjx-pptx/tests/charts.rs` does the same over `charts.pptx`, whose workbook carries a theme;
  `crates/mjx-xlsx/tests/charts.rs` is the third host. A companion case shows a refusal changes
  neither part, and another shows `regenerate_chart_workbook` still replaces the package — which is
  what keeps the first case from being a claim about a method nobody calls.

### Why no gate caught it

`crates/mjx-docx/tests/charts.rs` asserted the workbook part *did* change, starting from a blank
document — so it locked the behaviour in without ever seeing what was lost. No test opened a
producer-written file and asked whether its embedded workbook's content survived a data edit. Three
now do.

## [0.0.131] - 2026-09-07

### The documentation gate and the index — a doc page can now fail (MJXOFF-199, G1)

**Phase G's first child, and the harness the eight that follow write into.** Until this release
nothing in the workspace read a single prose document, and MJXOFF-88 §9 B5/B6 records what that
cost: five documents directing a reader at a `presentation.rs` that has been the directory
`crates/mjx-pptx/src/presentation/` since Phase A, and a **live test** whose own doc comment cited a
file MJXOFF-99 had deleted and described that deletion in the future tense. Every one was found by a
person reading carefully.

### Added

- **`xtask/tests/doc_gate.rs`** — the gate. Its corpus is `git ls-files`, never a list: every
  tracked markdown page and the comments of every tracked Rust file, so a new page is inside it the
  moment it is committed. Four checks, each reporting its counts on success as well as on failure:
  - **Paths.** Every repository path a document names in a code span or a file-shaped markdown link
    exists — resolved crate-relative, then from the root, then by crate name, with `{a,b}` groups
    expanded and a `file.rs::symbol` citation checked against the named file. At this release:
    **1,111 mentions of 369 distinct paths across 293 documents**, plus 5 `file::symbol` citations.
  - **Symbols.** Every crate-qualified reference (`mjx_sml::CellFormula`, and `crate::…` inside a
    crate's own sources) still names something that crate has — **1,356 references across 318
    documents, over 21 crates holding 14,302 declared item names.** The subset and its boundary are
    stated on the test: a bare `Type::member` is not checked, because the same word is a type in
    several crates and a word in every sentence.
  - **The index, both directions.** `docs/api/README.md`'s row set must equal `git ls-files '*.md'`
    exactly. Committing a page without indexing it fails; indexing a page that does not exist fails.
  - **Anti-vacuity floors on all of it**, stated as *the extractor is still matching* rather than as
    *the corpus is this size*, so a floor cannot fire before the assertion it guards. Neutralising
    the code-span scanner turns three checks red with "the extractor has stopped matching" instead
    of a silent green — which is the failure this gate exists to prevent, applied to itself.
  - **The crate set, derived twice and compared in both directions.** Three of the four checks are
    keyed by crate, each key set built by a walk, and a total cannot see one crate leave: dropping
    `mjx-sml` took 1,793 item names and every `mjx_sml::…` reference out of the symbol comparison
    and left all four tests green, while dropping `mjx-pptx` from the resolver's crate-name table
    took five path mentions out of 1,120 and did the same. A floor sized to catch the extractor
    dying altogether cannot catch it losing one crate — and losing one crate, to a rename or a
    parse tweak, is the failure this gate will actually meet. So every walk's crate set is held to
    `Cargo.toml`'s own `members` list in both directions, a symbol whose head is a declared crate
    missing from the map is a named failure rather than a skip, and every skipping arm is counted
    and printed.
- **`docs/api/README.md`** — one entry point, 69 rows, one per markdown page in the repository, with
  what it covers and which crate owns it. It is prose a person writes whose *row set* is derived and
  enforced: an index generated from the same walk a test compares it against would prove nothing and
  would carry no descriptions.
- **A single escape hatch, and a liveness check on it.** A document may name a file or item that no
  longer exists **only inside a block that also names the ticket that removed it**. That one rule
  separates honest history from a stale live claim, and `RETIRED_PATHS` / `RETIRED_SYMBOLS` are
  themselves failed when nothing names them any more.

### Fixed

- **Six documents pointed at `crates/mjx-pptx/src/presentation.rs`**, a directory since Phase A —
  the five §9 B6 names plus `docs/DRAWINGML_FILL_HANDOFF.md`, which it does not.
- **Eight live sites named symbols MJXOFF-99 deleted**, a class §9 B5 does not list at all: the
  `mjx-chart` workbook writer named from `mjx-sml`'s crate root, its package writer, its address
  module, its constants and its package-writer suite, and from `mjx-xlsx`'s parts module; plus
  `crates/mjx-sml/docs/SHARED_STRINGS.md`, which said the duplicate "is still there" and that
  MJXOFF-99 "performs the deletion", in the future tense.
- **`crates/mjx-sml/src/strings/table.rs`** claimed in the present tense that a deleted parity gate
  *compares* two writers, naming no ticket — §9 B5's first site.
- **`crates/mjx-sml/tests/shared_strings_fidelity.rs`** — §9 B5's second. The live test's doc cited
  a deleted file as "the other side" and said MJXOFF-99 "then deletes" the writer. Rewritten to say
  what is true, and the test renamed from `an_authored_table_matches_the_chart_writers_bytes_exactly`
  to `an_authored_table_writes_exactly_these_bytes`: there is no chart writer to match.
- **`CLAUDE.md`** was still prospective about that deletion (§9 B13), and addressed
  `bindings/mjx-python/tests/test_stub_parity.py` from the wrong root.
- **Nine more stale citations the gate found on its first run** — `docs/BENCHMARKS.md` pointing at
  an `xtask/src/fuzz/allocation.rs` that MJXOFF-95 moved to `crates/mjx-allocation-counter`;
  `xtask/src/corpus/memory.rs` naming the same moved module; `mjx-sml` and `mjx-xlsx` citing a
  `docs/fidelity_and_gaps.md` Excel has never had; the two bindings citing test files that do not
  exist (`tests/node/enums.mjs`, `tests/node/format.mjs`, `tests/test_format.py`);
  `crates/mjx-omml/src/support.rs` naming a `crate::geometry::Transform2D` that is `mjx-dml`'s; two
  `xtask` codegen modules naming a `crate::support` that is `mjx-ooxml-types`'; and a broken sibling
  link in `crates/mjx-xlsx/docs/guide/worksheet_tables.md`.

### Changed

- **The eight July-2026 hand-off documents are dated, not retired** (§9 B6). Each carries a banner
  at its head saying it describes the repository as it stood on a given day, before the Phase A
  module split, and that its paths, status markers and counts are not maintained. They are kept
  because the design reasoning they record — 1,477 lines of why each decision went the way it did —
  is written down nowhere else; only their description of the layout has expired. Retiring them
  would have lost the reasoning to save a banner.
- **CI's `lint-test` job names the gate as its own step.** `cargo test --workspace` already runs it
  in both feature modes — verified with `--no-run`, which lists `Executable tests/doc_gate.rs` in
  each — but MJXOFF-130 found `xtask/tests/` reachable by one job and auditor pass 4 found
  `mjx-dml`'s preset-geometry sweep reachable by none. A gate whose job is only implied is the gate
  that turns out not to run.

## [0.0.130] - 2026-09-07

### The Office-authored corpus — the ingestion path, and the weakness it retires (MJXOFF-130, F3)

**Phase F's third child, and the last of the sixty-two-child programme. It builds the road; it
cannot supply the traffic.** No test in this repository has ever read a file Microsoft Office wrote —
the deepest weakness the project has, recorded as `R2` — and the one an agent may not close, because
the value of an Office-authored file is entirely its provenance. **The corpus ships empty, nothing is
marked, and nothing is tagged.**

### Added

- **`xtask/tests/office_corpus.rs`** — the corpus suite. It walks `tests/office-authored/` (**the
  corpus is the directory**, the same rule `mjx-fixtures` makes for every byte-identity corpus) and
  holds every file it finds to: per-part decompressed-payload identity and container-structure
  identity across an edit-free save (`mjx-opc`'s `roundtrip` semantics); every XML part through the
  fidelity tree (`tree_roundtrip`'s); **the same round-trip through the facade**, so `Deck`,
  `Document` and `Workbook` are held to markup nobody here wrote; `Package::validate` before and
  after; and A7c's child-order audit over Office's own output, which is the strongest available check
  that the generated `ChildOrder` tables say what Office actually writes.
- **`cargo run -p xtask -- validation-artefacts --ingest <file>`** — the other direction of the
  artefact command. Hand it something saved out of Office and it reports which validation entry the
  file answers, every check above, and **where it would be committed**. It copies nothing: committing
  a file is a decision taken against the redistribution rule, and a command that filed it would be
  taking that decision for the person running it.
- **`docs/validation/06-the-office-pass.md`** — the hand-off. The order to work through (`R1` first,
  and stop there if PowerPoint disagrees), what a failure looks like against a documented gap, what
  to save out of which application and where to put it, the six checks that settle a **decision**
  rather than report a fact, the seven that are **blocked** and whether the corpus unblocks each, and
  the two escalations and three unfixed defects the programme is handing over.
- **`mjx_schema_gate::audit_order_report`** and `OrderAudit` — the child-order walk without the
  panic, so a *reporter* can print the round-trip and package verdicts too.
  `audit_deck_order` and `assert_deck_is_in_schema_order` are now written on top of it: one walk,
  three callers, and the second of those opens the package once instead of twice.

### Changed

- **`tests/office-authored/README.md`** now carries the **redistribution rule** — a committed file
  must be one whose *content* we authored, started from *Blank* rather than from one of Office's
  templates, carrying nothing from anywhere else and no personal data, with rights that need no
  argument. It is checked per file, before committing, and **recorded in a table in that file**; an
  unclear case is left out and *said* to have been left out.
- **Three verification blocks, rewritten conditioned on the corpus rather than ahead of it.** The
  PowerPoint and Word gaps pages say what now exists and that it is empty; the Excel guide grows the
  *Built, not yet verified against Excel* section it never had, with the six rows MJXOFF-79's risk
  list implies and the entry id that checks each.
- **`mjx-schema-gate` is a dependency of `xtask`** rather than a dev-dependency of it. The ingest
  command reports the same schema and child-order verdicts a suite asserts, and it is a command
  rather than a test; the alternative was a second child-order walk inside `xtask`. Nothing shipped
  depends on the gate, and `xtask` is host-only, `publish = false` and outside the ranked graph —
  which `xtask/tests/layering.rs` already distinguishes. The gate's own documentation said it was a
  dev-dependency "of nothing else", which had been untrue since MJXOFF-122; it now says what is true.

### Fixed

- **`two_runs_produce_byte_identical_artefacts` no longer assumes an empty corpus.** It asserted
  `names.len() == AREAS.len()`, which would have started failing the day the first Office-authored
  original landed — a gate that breaks on the work it is waiting for. The expected count is now
  derived from how many areas have an original.
- **`docs/validation/02-risk-order.md` said "five" design questions and listed six.** The 0.0.129
  entry below already said six.
- **Three gates in `xtask/tests/validation_harness.rs` would have gone red the day the first
  Office-authored original landed**, and none of them for a reason that is this library's. Measured,
  not predicted: with a stand-in file in the corpus slot, `every_generated_artefact_is_a_valid_package`
  failed on `21 != 20` (a second hard-coded `AREAS.len()` beside the determinism one) and
  `every_generated_artefact_is_schema_valid_and_in_child_order` failed on `v-xlsx-02-edited.xlsx` for
  a `workbookPr@dateCompatibility` **LibreOffice** wrote — a deviation `tolerances.rs` already
  records for that fixture, reaching the gate through a path that consults no tolerance list. An
  `edited` artefact is mostly somebody else's file, re-emitted verbatim, so it is now held to **no
  *new* defect**: what the original arrived with is subtracted, and anything left is ours. The
  authored artefacts are unchanged — nothing in a file we wrote is excused.
- **The validation harness's schema half was skipping in every CI run.** `xtask/tests/` is reached
  only by `lint-test`, which has no `References/`, so
  `every_generated_artefact_is_schema_valid_and_in_child_order` validated **nothing** on CI and the
  child-order half carried the job alone. The `schema-validity` job now runs
  `cargo test -p xtask --test validation_harness --test office_corpus` under `MJX_REQUIRE_SCHEMA=1`,
  where the schemas are. Both suites are green there; the point is that nobody knew.

### The `mc:Ignorable` / `CT_Extension` seam — diagnosed, reproduced, and deliberately not tolerated

**The first real Excel workbook, and any file carrying an Office chart, will report a schema
deviation, and it is a defect of this project rather than of the file.** The gate validates the
markup-compatibility-*resolved* view of a part, because `mc:Ignorable` names attributes the base
schema has no declaration for; resolution removes an ignorable element together with its content; and
`sml.xsd`'s and `dml-chart.xsd`'s `CT_Extension` declare their wildcard as a bare
`<xsd:any processContents="lax"/>`, whose `minOccurs` therefore defaults to **1**. The emptied
`<ext>` is then rejected with *Missing child element(s)*.

`xtask/tests/office_corpus.rs` reproduces **three views** of one worksheet, authored there for the
purpose and presented as nothing else: as a producer writes it (rejected — `mc:Ignorable` is not
allowed, which is why the gate resolves at all), with the compatibility attributes removed and the
ignorable content kept (**validates**), and fully resolved (rejected). So the schema does not object
to the extension; it objects to the **hole** resolution leaves. `pml.xsd`'s own `CT_Extension` and
`dml-main.xsd`'s `CT_OfficeArtExtension` both say `minOccurs="0"`, which is why the defect reaches
presentations and documents through their *charts* rather than through their main parts — it is not
Excel's alone.

**It is not recorded as a tolerance.** A tolerance is for one file and one message and never for a
defect of ours; recording this one would file a gate defect as a quirk of somebody's spreadsheet, and
it would then look for ever like a property of the corpus. It is filed as **MJXOFF-196** with the
reproduction, the schema sweep behind it and three candidate fixes, and it is the one thing that goes
red when the first original lands: the `-edited` artefact built from it reaches
`assert_authored_deck_is_schema_valid`, which tolerates nothing. The reproduction fails the day the
seam is fixed, which is the signal to delete it.

### Where the line is drawn on an ingested file

An ingested file is **not ours**, and that decides what may fail a build. Byte identity at the
container and through the facade, the fidelity tree, a package defect *we* introduced by saving, and
a part out of `xsd:sequence` all **fail**. A defect the file **arrived** with is *reported* — A7b's
scope rule is that such a file must still open and re-save unchanged — and so is a part its producer
wrote that the ECMA-376 XSDs reject, because MJXOFF-103 measured Apache POI 5.5.1 writing an empty
`<c:tx/>` that `dml-chart.xsd` refuses, and reddening a build over somebody else's markup teaches
nobody anything.

The suite proves itself able to fail rather than asserting that it can: the same engine is run over
four deliberately broken packages — bytes that are not a ZIP, a package cut in half, a worksheet
renamed at the root, and a relationship with no target — and each must be caught by the check that
owns it, with a sound package as the control. The first spelling of the last one pointed at
`xl/theme/theme1.xml`, which `sample.xlsx` *has*: it was not a corruption at all, and `Package::validate`
was right to hold. A mutation has to be reachable before its verdict means anything.

## [0.0.129] - 2026-09-07

### The validation checklist — every entry, all three formats, ordered by risk (MJXOFF-128, F2)

**Phase F's second child. It writes every checklist entry and marks nothing.** 113 checks across
`docs/validation/`, each a stable id, the MJXOFF id of the child that shipped the feature, an
artefact, an object, an action, an expected result, a risk level, three call chains and a **blank**
result line. Judging what real Office renders needs a person with Office in front of them; that pass
is the user's, and nothing here stands in for it.

### Added

- **`docs/validation/02-risk-order.md`** — the order the pass is worked through, which is
  deliberately not the order the pages are numbered in. **R1 first, and still the single
  highest-risk item in the repository**: the 0.0.58 tier-5 change, where a non-placeholder shape
  takes the master's `p:otherStyle` / `p:bodyStyle` per §19.3.1.35, real PowerPoint is believed to
  match the *previous* behaviour, and the change is isolated in one revertible commit. Then R2
  through R7 unchanged from MJXOFF-63, then Word's five risk areas (MJXOFF-74) and Excel's five
  (MJXOFF-79). It also collects, in one table, the **six checks that are design questions rather
  than checks** — each says *record which happens* and names the decision that follows, and none may
  be marked `differs`, because there is nothing to differ from.
- **`docs/validation/03-presentations.md`, `04-documents.md`, `05-workbooks.md`** — 62, 26 and 25
  checks. Every harvested number from the Phase A children's own completion reports survives in the
  terms its report used: 44 pt and 28 pt from `p:txStyles`, Widescreen 13.333 x 7.5 in, `accent1` =
  `4472C4`, Calibri Light and Calibri, 41.5 / 42.5 / 43.5 rather than 19.2 / 21.4 / 16.7, **100000**
  for a square chevron and **200000** for a 2:1 one, 45 degrees and ~3 pt out and ~4 pt blur, slice 1
  exploded 25 % and slice 0 `2E75B6`, a polynomial trendline of order 3, a merged total row of one
  row by two columns, and all sixteen plot types with `c:stockChart` drawing high-low-close from
  three series.
- **`V-PPTX-07` (`geometry`) and `V-PPTX-08` (`chart-decoration`)** — two new areas in
  `xtask validation-artefacts`, in all three languages. They exist because writing the checks found
  harvested expected results with no file to check them against, and MJXOFF-122's own rule is that
  an entry may not describe an artefact nobody produces. `V-PPTX-07` is **the one artefact authored
  at 4:3** (`9_144_000` x `6_858_000`) on a slide taken from the layout, so A3's rescaled-placeholder
  question has a file at last; it also carries the two chevrons whose `maxAdj` guide (`*/ 100000 w
  ss`) answers 100000 and 200000, the four arc-tangent presets `moon` / `arc` / `circularArrow` /
  `gear9`, and a `custGeom` with all five of `a:avLst`, `a:gdLst`, `a:cxnLst`, `a:rect` and
  `a:pathLst` whose apex is placed by the guide `apex = */ w 1 2`. `V-PPTX-08` carries the three
  label tiers merged, the exploded and recoloured pie slices, the order-3 trendline extended two
  categories forward with equation and R-squared, two sets of error bars on one scatter series (one
  per axis), a `c:dPt` left dangling at index 2, a value axis bounded 0-25 and reversed — the only
  artefact that writes a `CT_Scaling`, where `c:max` precedes `c:min` — a chart detached from its
  embedded workbook, and the sixteen plot types four to a slide.
- **`xtask/tests/validation_calls.rs`** — the gate that makes the two machine-checkable claims in a
  page of prose actually checked. Every `Calls:` line is resolved against three surfaces that have
  nothing to do with each other: every `pub fn` inside an `impl Deck` / `impl Document` /
  `impl Workbook` in the facade, every `def` inside the three classes of the committed `.pyi`, and
  every `#[wasm_bindgen(js_name = "…")]` in the same three `impl` blocks of the WebAssembly binding.
  The TypeScript half is checked *against the Rust half of the same chain* — the documented
  camelCase name must be the `js_name` the binding publishes for that exact `snake_case` method — so
  a chain that renamed one half and not the other fails even though both names exist. And every
  artefact a check names must be a file this repository produces: a name `validation-artefacts`
  writes, a path under `tests/fixtures/`, or an example's source.

### Fixed

- **A three-language call chain named a method a default build does not publish.**
  `Deck::vml_part_names` is `#[cfg(feature = "vml")]` in both bindings, so neither the Python stub
  nor the WebAssembly surface has it; the entry that named it now says so and uses the modern half
  of the same hop. Found by the new gate while it was being written, which is what it is for.

### Notes

- **Seven checks name no artefact and say **blocked**, each with the reason and what would unblock
  it.** They are not padding: `ColorSpec` carries a colour's kind and value and **no transform
  children**, so nothing can author the `comp` / `gray` / `gamma` / `invGamma` that R3 is about;
  `CharacterPropertiesSpec` has no font setter, so nothing can author a `+mj-sym` reference;
  `set_shape_transform` writes **only the fields its argument names** — an unset field means *leave
  it alone*, never *clear it* — so nothing can author the rotation-only transform R7 is about; and
  the facade has no `add_alt_chunk`. The rest wait on MJXOFF-130's Office-authored corpus.
- **MJXOFF-108's 28-row comparison table is carried in untouched.** Its **Excel says** and
  **Verdict** columns are still empty and unmarked. `V-XLSX-02.1` points at it and adds nothing.
- **`MJXOFF-143` had already closed the whole-part re-flow limitation**, and PowerPoint's gaps page
  had already moved that row to *What used to be here*. This child did not move it; it records the
  closure in `V-PPTX-08.11` so a reviewer does not report a re-flow as a defect.
- The Word and Excel area lists were derived from **the facade's own module structure** read against
  each format's gaps page, not from ticket text. Excel's `features`, `names`, `preserved`, `print`
  and `tables` modules carry readers and removers and no authoring call at all, which is why those
  areas are recorded as deliberately uncovered.

## [0.0.128] - 2026-09-07

### The consolidated validation harness — the artefacts a human Office pass reads (MJXOFF-122, F1)

**Phase F's first child, and the first thing in this repository that admits what it cannot check.**
Every fixture here was written by this project or by LibreOffice; no test has ever read a file
Microsoft Office wrote, and no gate in this workspace can answer *does real Office render what we
intended?* This child builds everything that question needs except the answer.

**It marks nothing.** There is no `pass` in any result line, no verdict inferred from a LibreOffice
conversion, and no place where an agent stands in for a person with Office in front of them. A test
asserts that, over every page of `docs/validation/`.

#### `xtask` grows a fourth command

`cargo run -p xtask -- validation-artefacts [--format pptx|docx|xlsx] [--area <id or number>]
[--out <dir>] [--list]` writes two artefacts for each of **eighteen** validation areas — six per
format. Every one is produced through `mjx-ooxml` and names no crate below it, so the human pass
validates the facade and its error mapping as a side effect.

* The **authored** variant is built from `Deck::blank`, `Document::blank` or `Workbook::blank`:
  nothing is read from disk, so every byte is one this library wrote.
* The **edited** variant is built by editing an Office-authored original from
  `tests/office-authored/`. That corpus is MJXOFF-130's to fill and is empty, so every edit variant
  **skips by name** — the area and the exact path it looked for — and `MJX_REQUIRE_OFFICE_CORPUS=1`
  turns any such skip into a failure, the arrangement `MJX_REQUIRE_SOFFICE=1` already makes for the
  `office_open` canary.

The command lives in a new **library target** for `xtask`, because `xtask/tests/validation_index.rs`
is written against the area catalogue and an integration test cannot see a binary's modules. The
alternative — parsing the binary's `--list` output — would have made a text format the contract
instead of a type. `codegen`, `fuzz` and `corpus` stay private to the binary; nothing depends on
`xtask`, and `xtask/tests/layering.rs` still says so.

#### The same eighteen artefacts, in three languages

`bindings/mjx-python/tests/test_validation_artefacts.py` and
`bindings/mjx-wasm/tests/node/validation_artefacts.mjs` are the same eighteen generators, call for
call, and each compares its output against the Rust one **part by part, byte for byte**. That is
A10's acceptance test generalised from one walkthrough to the whole catalogue: a binding method
wired to the wrong facade method changes one part payload, and a human reading the file in Office
would never know why it looked wrong.

Both comparisons state their artefact set over the *filesystem* rather than over a list in their own
file, so an area added in Rust and not in a binding fails rather than being silently skipped.

#### The index, and why it is not a tautology

`docs/validation/01-index.md` binds every entry id to the artefacts it is read against, and
`xtask/tests/validation_index.rs` compares two lists that cannot drift together: the **entry** side
is parsed out of hand-written markdown, the **artefact** side is a `read_dir` of the directory the
`xtask` binary has just written. Both are floored against the catalogue before either comparison
runs, so a parser that stopped matching table rows fails rather than passing an empty comparison.
The edited column is checked as an *if and only if* against the Office corpus, so an empty corpus is
still an assertion.

#### Every artefact through the gates a machine can answer

`xtask/tests/validation_harness.rs` runs the ECMA-376 schema gate, `Package::validate` and the
child-order audit over all eighteen, and quotes the audit's per-part `elements_visited` counts —
89 parts audited, none of them vacuous. Preserved-foreign skips are pinned by *label*, so a part
that starts skipping under a new one fails rather than quietly widening what the gate tolerates. Two
runs of the command are asserted **byte-identical**, without which the three-language comparison
means nothing. One artefact per format is converted by LibreOffice as a canary — one conversion,
never a sweep, and never a verdict.

#### Documentation

* `docs/validation/00-method.md` — the entry-id scheme, the three risk levels, the result
  convention, how to file an issue, and the rule that keeps the exercise honest: **a documented gap
  is never a validation failure**.
* `docs/validation/01-index.md` — the eighteen entries, and the areas the harness deliberately does
  not cover, each with its reason.
* `tests/office-authored/README.md` — the corpus slot, its naming convention, and why it is not
  under `tests/fixtures/`.

MJXOFF-108's 28-row effective-cell-format table is carried in **unchanged**: its *Excel says* and
*Verdict* columns are still empty and unmarked, and nothing here touched them.

## [0.0.127] - 2026-09-07

### The cross-format consistency pass — one reading of the whole public surface (MJXOFF-118, E6)

**Phase E's last child, and the last cheap moment to rename anything.** Three formats were built in
three phases, months apart, by different agents following the same rules; rules produce consistency
locally, and only a deliberate cross-cutting read produces it globally. This is that read. Nothing
here changes a byte any file receives.

#### The shared-markup reachability table, and the test that keeps it true

The gate MJXOFF-82 named: **nothing in `mjx-dml`, `mjx-sml`, `mjx-chart`, `mjx-vml` or `mjx-omml` is
reachable from one format's facade surface but not another's without a written reason.**
`crates/mjx-ooxml/docs/shared_markup_reachability.md` (rendered as
`mjx_ooxml::shared_markup_reachability`) is that table, and
`crates/mjx-ooxml/tests/shared_markup_reachability.rs` re-derives it from the workspace on every
`cargo test` — the crate-level grid out of three `Cargo.toml`s, the 102-capability grid out of the
facade's own `src/`. A method added to one surface and not the others, a note no row cites, a format
crate that starts modelling a shared markup: each fails naming the row it is about.

What the derivation found:

- **`mjx-chart` came out symmetric.** 25 of 28 chart capabilities are on all three surfaces — and
  read the other way, *every* capability on all three surfaces is a chart capability. The three that
  are not each have a reason in the file format, not in this library.
- **Sixty-seven DrawingML capabilities reach `Deck` alone**, in four groups with four different
  reasons. The shape-properties one names an open seam rather than hiding it: `mjx-docx` already
  models `wp:spPr` as `mjx_dml::ShapeProperties`, but that type is interner-bound and the facade's
  boundary is not, and only `mjx-pptx` built the interner-free spec layer that crosses it.
- **`theme`/`color_map` reaching `Deck` alone is the clearest gap.** `mjx-sml` resolves
  `<color theme="N"/>` to a slot number and says the theme part is `mjx-xlsx`'s to fetch;
  `mjx-xlsx` does not fetch it, so an Excel theme colour comes back as a position where the same
  colour in a `.pptx` comes back as `RRGGBB`. No ticket owned this before the table did.
- **`mjx-vml` and `mjx-omml` reach no surface as types** — both interner-bound trees with no
  binding-friendly projection — and the bytes-and-identifiers surface that does exist is not
  symmetric either: `Deck` and `Workbook` have one, `Document` has none.

#### Excel's effective-properties guide — the third page, in one shape

`crates/mjx-xlsx/docs/effective_properties.md` joins `mjx-pptx`'s and `mjx-docx`'s, wired the same
way (a documentation-only module over `include_str!`, so its three examples are doctests and its
links are checked). It says the thing that makes Excel different rather than restating the others:
**Excel inherits nothing** — a cell carries an index, that index names a record, and that record
carries four more plus a fifth into a second table of the same records — which is why an
`EffectiveCellFormat` reports *which layer* answered where the other two report only a value.

`crates/mjx-ooxml/tests/effective_properties_shape.rs` is what keeps the three one shape: same
opening sentence, the same four load-bearing sections in the same order, real compiled examples on
each, each wired into its crate. Two `mjx-docx` headings were renamed to the shared spelling.

#### Binding parity, in both directions

`chart_series_references` was bound for `Workbook` and for neither `Deck` nor `Document` — a facade
method two languages could not reach. Both bindings grow it, the `.pyi` grows two entries, and with
those four **every `pub fn` on all three facade surfaces is bound in both languages**, the escape
hatches excepted.

The reason nothing caught it is the more useful finding: three of the six coverage suites carried an
explicit *"remove one binding and this goes red"* case and three did not — including **both halves
of the `Deck` pair**, which is the pair the specification names. The two missing guards are added.

#### Naming and shape

- `ChartLabelScope`'s `plot_idx`/`series_idx`/`point_idx` are spelled out and are `u32`; so are
  `ShapeInfo::index` and `LayoutInfo::{index, master_index}`. Both rows are in the *Unreleased —
  0.1.0* ledger. Python and TypeScript are unchanged: **both bindings already published these
  names and this width**, and five conversions are gone.
- `ErrorDetail` now states, as a table, what each of its five fields means for a slide, a paragraph
  and a cell — including the two Excel answers a caller would otherwise have to discover by
  experiment (a sheet is reported through `index`, and an Excel cell address populates neither `row`
  nor `column`).
- `mjx_sml::CellReference`'s constructors take `(column, row)` where thirty-odd methods elsewhere
  take `(row, column)`; the reason is now on the type rather than in a ticket. **Reordering remains
  the user's call.**

#### Counts that had already expired

Every count this child quotes was measured, and several it found were not: the Excel guide's page
count was written in three places as thirteen, fourteen and fifteen (it is seventeen — the numeral
is now in none of the three); `error.rs` under-counted `DocxError` by six variants and `XlsxError`
by seven; `README.md` still called the project PowerPoint-first and listed two test-only crates
where there are three; and three rustdoc sites still described
`crates/mjx-chart/src/workbook.rs`, which MJXOFF-99 deleted, one of them as the live rationale of a
test.

## [0.0.126] - 2026-09-07

**Excel's legacy surfaces** (MJXOFF-114, Phase E position 5): a cell comment, the Transitional VML
box that draws it, and the identifier hop from a sheet's modern markup to the legacy shape an OLE
object or a form control is drawn as.

### Added

- **`mjx_sml::comments`** — `CT_Comments`, `CT_Authors`, `CT_CommentList`, `CT_Comment`,
  `CT_CommentPr` and `CT_LegacyDrawing`, the last slot of `CT_Worksheet` that had an owner
  (rank 30). `commentPr@anchor` consumes MJXOFF-127's `ObjectAnchor` rather than modelling
  `CT_ObjectAnchor` a second time, and every placement goes through a generated child-order table.
- **The comment family on `mjx_xlsx::Workbook`** — `sheet_comments`, `comment_at`,
  `comments_markup`, `edit_comments_markup`, `add_comment`, `set_comment_text`, `remove_comment`,
  and the VML side: `sheet_vml_drawing_part`, `vml_drawing_markup`, `edit_vml_drawing_markup`,
  `with_vml_shape_for_comment`, `with_vml_shape_for_ole_object`,
  `with_vml_shape_for_form_control`. A comment is **two parts**, and `add_comment` writes all seven
  things that have to agree while `remove_comment` takes them away.
- **`SpreadsheetDefect::CommentWithoutABox` and `CommentBoxWithoutAComment`** — the two-halves
  invariant, checked by `Workbook::validate` over a saved package rather than asserted by the
  surface that writes it. Neither half of a comment names the other by relationship, so nothing in
  the packaging layer could ever have noticed half of one.
- **`mjx_vml::Drawing::shape_by_numeric_identifier`** and **`mjx_vml::shape_identifier_for_number`**
  — the shared half of the hop. SpreadsheetML names a shape by a *number* (`x:oleObject@shapeId`,
  `x:control@shapeId`, `x:comment@shapeId`) where PresentationML names it by the string that number
  appears in; the `_x0000_s` spelling and the three attributes producers put it in are stated once,
  in the crate both formats reach.
- **The same surface on `mjx_ooxml::Workbook` and on both bindings** — `sheet_comments`,
  `cell_comment`, `add_cell_comment`, `set_cell_comment_text`, `remove_cell_comment`,
  `vml_shape_id_for_ole_object`, `vml_shape_id_for_form_control`, `sheet_vml_part_bytes`, with
  `SheetCommentInfo` and `CommentBoxInfo`.
- **Three producer-written fixtures**, none of them this project's: `cell_comments.xlsx` and
  `legacy_form_control.xlsx` from **LibreOffice 25.8.7.3** driven headless over UNO, and
  `comments_third_party.xlsx` from **XlsxWriter 3.2.9**. The Excel guide gains
  *Cell comments and legacy content*, whose every snippet is a compiled doctest.

### Fixed

- **`mjx-opc` treated an edited VML part as though it were not XML.**
  `XML_CONTENT_TYPES_WITHOUT_SUFFIX` spelled its one entry `…vmlDrawing` while `is_xml_content_type`
  folds its argument to lower case, so the entry matched nothing and an authored `.vml` sat outside
  `Package::authored_xml_parts` — outside `Package::validate`'s relationship checks and outside
  every format layer's markup checks — from the day the list was written. A test now fails on any
  entry written in a spelling the fold would swallow.
- **Deleting one comment could delete every comment box on the sheet.** Removal matched the shape by
  its `@id`, and LibreOffice gives every comment shape in a part the same one. It matches by
  position now; the fixture that found it is the producer file, and the two-halves invariant is what
  reported it.

## [0.0.125] - 2026-09-06

**Charts on the Excel surface** (MJXOFF-111, Phase E position 4): the third host for one body of
chart logic, and the one chart case that exists nowhere else in this library — a chart whose data
source is a **live range in the same workbook** rather than an embedded copy.

### Added

- **The chart family on `mjx_xlsx::Workbook`** — fifty methods, every one of which resolves
  `(sheet, anchor)` to a chart part and then calls the identically-named function in
  `mjx_chart::chart_ops`. MJXOFF-103 moved that body down for Word; Excel is the third wrapper
  around it and adds no Excel-local chart path. The address is MJXOFF-107's anchor index, so
  `add_chart`'s return value is accepted by `remove_sheet_drawing_object` exactly as
  `add_two_cell_anchored_picture`'s is.
- **`Workbook::resolve_range_reference`** and the `ResolvedRange` / `ResolvedArea` /
  `ResolvedRangeCell` / `RangeCellValue` / `RangeProblem` report — a chart's `c:f`, resolved against
  this workbook's cells. It resolves a **reference** and does not evaluate a formula; a cell holding
  one answers with its cached value, as `cell_text` does. A quoted sheet name, absolute markers, a
  multi-area union, a 3-D span and a defined name (sheet-scoped winning over workbook-scoped, as
  §18.2.6 says) all resolve; every unresolvable case is a typed `RangeProblem` on the area it came
  from rather than a failure of the whole call.
- **`Workbook::chart_series_freshness`** — each series' cache set beside what its cells actually
  say, with **each named**. `values_agree` has three answers: `Some(true)`, `Some(false)`, and
  `None` for *cannot say* — the values are a literal, or the reference resolved to nothing. "The
  cells disagree" and "there are no cells" are different facts.
- **`Workbook::refresh_chart_cache_from_cells`** — the opt-in repair, and the exact counterpart of
  `refresh_chart_workbook` pointing the other way. Writing a cell deliberately leaves a chart's
  caches alone (this library recalculates nothing), so this is how a caller makes the chart draw
  what the sheet now says.
- **`Workbook::add_range_chart`** with `SheetChartSource` / `SheetChartSeries` — a chart whose `c:f`
  name cells in this workbook, whose caches are seeded from those cells, and which carries **no
  embedded workbook at all**. `Workbook::add_chart` writes the other kind, taking the same
  `ChartData` a slide and a document take.
- **`mjx_chart::ChartData::ranges`**, `ChartRanges` and `ChartSeriesRange` — where a chart's data
  lives, when it is not the companion embedded workbook. A source no range names is written as a
  **literal** (`c:numLit` / `c:strLit`) rather than falling back to `Sheet1!$A$2:$A$N`, which would
  name a part that is not in the package.
- **`mjx_chart::chart_ops::series_references`** and `ChartSeriesReferences` — where each series says
  its data lives, as against what its cache holds. On **all three** surfaces: `Deck`, `Document` and
  `Workbook` each gained `chart_series_references`.
- **`mjx_dml::spreadsheet_drawing::new_anchored_graphic_frame`** — the frame a chart sits in on a
  sheet. `CT_GraphicalObjectFrame` declares `xdr:xfrm` `minOccurs="1"`, unlike the `a:xfrm` a picture
  may omit, so it is written (all-zero, as Excel and LibreOffice both write for a two-cell anchor).
- **`mjx_sml::ReferenceAreas`** — the areas of a reference that names more than one, split on the
  commas that are not inside a quoted sheet name or an external-book bracket. `Copy` and
  allocation-free, like the rest of that module.
- **`PartKind::Chart`**, with `REL_CHART`, `REL_PACKAGE` and `CONTENT_TYPE_CHART`. The part
  inventory names a chart part instead of leaving it unclassified; twenty-seven part kinds became
  twenty-eight.
- **`tests/fixtures/chart_in_sheet.xlsx` and `chart_stale_cache.xlsx`** — two workbooks **written by
  LibreOffice 25.8.7.3**, not by this project. The first carries a chart over a live range with no
  `c:externalData` at all; the second is the same package with the sheet and string table of a
  second run spliced in, so its **caches and its cells disagree on every point**. A fixture whose
  cached values equalled its cell values would prove nothing about which source a reader used.
- **The Excel guide's chart page** (`crates/mjx-xlsx/docs/guide/charts.md`), four compiled
  doctests, and `examples/chart_range_cost.rs`, which asserts with the counting allocator that a
  resolution is bounded by the range rather than by the sheet: on a 30,000-cell sheet a three-cell
  range costs **2,109 bytes** beyond the sheet's own read, and four areas in one call cost one sheet
  parse where four calls cost four.
- **Both bindings** gain the family: `workbook.add_range_chart(...)` in Python,
  `workbook.addRangeChart(...)` in TypeScript, with `ChartRangeSeries`, `ChartSeriesReferences`,
  `ChartSeriesFreshnessInfo`, `SheetChartWorkbookInfo`, `ResolvedRangeInfo` and `RangeCellInfo`
  projected alongside.

### Fixed

- **`crates/mjx-ooxml/tests/chart_surface_parity.rs`'s strongest assertion was vacuous.** It compared
  `deck.chart_part_bytes(...)` against `document.chart_part_bytes(...)` after twelve edits, and
  `mjx_opc::Package::part_bytes` answers `None` for a part whose body is `Edited` — so the comparison
  had been `None == None` since MJXOFF-103 wrote it, and a chart part wired to the wrong bytes would
  have satisfied it. It now compares the parts of the **saved** packages and asserts all three are
  really there. The three surfaces do agree, byte for byte.

### Changed

- **`Workbook::detach_chart_workbook` removes the embedded workbook part**, unless another chart
  still names it. `mjx_docx::Document::detach_chart_workbook` leaves it in the package and says so;
  this surface cannot, because `Workbook::save` runs `Package::validate`, which refuses a package
  holding a SpreadsheetML part no relationship chain reaches — so a detach that left it behind would
  hand back a workbook this library then declines to write.
- **`XlsxError` gains five variants**: `ChartAccess` (wrapping `mjx_chart::ChartAccessError` whole,
  the `mjx-docx` shape rather than `mjx-pptx`'s eight restated variants), `ChartData`,
  `InvalidChartData`, `AnchorIsNotAChart` and `ChartHasNoExternalData`. The facade's `classify_xlsx`
  routes the first through the *same* `chart_access_code` `DocxError::ChartAccess` goes through, so
  the same index refused from a workbook, a document and a presentation answers the same
  `ErrorCode`.

## [0.0.124] - 2026-09-06

**Worksheet drawings** (MJXOFF-107, Phase E position 3): the `xl/drawings` part, the three anchor
modes, and the last place DrawingML reaches that this workspace had not.

### Added

- **`mjx_dml::spreadsheet_drawing`** — all seventeen complex types of
  `dml-spreadsheetDrawing.xsd`, as fidelity wrappers: `WorksheetDrawing` (`xdr:wsDr`), the three
  anchors, `CellMarker` (`xdr:from`/`xdr:to`), `AnchorClientData`, and the six things an anchor can
  hold. It sits in `mjx-dml` for the reason `wordprocessing_drawing` does — `xdr` is a DrawingML
  satellite schema whose content is DrawingML — and knows nothing about packages.
- **`WorksheetDrawing::insert_rows` and its three axis siblings** — each anchor mode does what it
  promises: a two-cell anchor moves *and* sizes, a one-cell anchor moves and keeps its size, an
  absolute anchor does neither. The returned `AnchorShift` per anchor includes `promise_kept`, which
  is `false` in exactly one case — a two-cell anchor resized while its own `@editAs` forbids it —
  rather than leaving that anchor silently wrong.
- **`mjx_sml::SheetAnchors`, `ColumnMetrics`, `GeometrySource` and `ResolvedAnchorBounds`** — an
  anchor resolved to a rectangle in EMU against a sheet's own column widths and row heights, and the
  honesty half of that answer. A row height is exact (points are 12,700 EMU); **a column width is a
  character count and cannot be a length** without a font measurement this library never makes, so
  the metrics are the caller's and every answer carries them. Where the sheet states nothing that
  could place the object — no `x:sheetFormatPr`, so no `@defaultRowHeight` — the answer is `None`.
- **`CT_Worksheet`'s last three owned slots**: rank 29 `drawing` (reusing MJXOFF-129's
  `SheetDrawing`), rank 34 `oleObjects` and rank 35 `controls`, with `EmbeddedObjects`,
  `EmbeddedObject`, `FormControls`, `FormControl` and `FormControlProperties`. Thirty-nine slots,
  **thirty-four modelled, five held**. `CT_ControlPr` repeats MJXOFF-127's trap exactly: six of its
  booleans default to `true`.
- **`mjx_xlsx::Workbook`'s drawing surface** — `sheet_drawing`, `drawing_markup`,
  `edit_drawing_markup`, `sheet_anchor_bounds`, the three `add_*_anchored_picture` calls,
  `remove_sheet_drawing_object` and the four axis shifts. Adding a picture writes six things
  together, including the image relationship **from the drawing part** rather than from the sheet:
  an `a:blip@r:embed` is resolved against the part that contains it. Every edit goes back through
  `ToXml::write_back` and the document the part was parsed from, so a shift that moves nothing
  re-emits the part byte for byte — prologue included, which for a file Apache POI wrote is
  `<?xml version="1.0" encoding="UTF-8"?>` and not this project's own declaration.
- **The whole of it on `mjx_ooxml::Workbook` and both bindings** (A10's rule) — twelve methods, four
  value types and two enumerations, with the committed `.pyi` stub extended.
- **`mjx_ooxml_types::spreadsheetdrawing`** — `ResizingBehavior`, `ColumnIdentifier` and
  `RowIdentifier`, generated. `ST_EditAs`'s members are named from §20.5.3.2's own enumeration-value
  titles (`MoveAndResizeWithAnchorCells`, `MoveWithCellsButDoNotResize`,
  `DoNotMoveOrResizeWithRowsOrColumns`), which say what happens to the object rather than naming the
  anchor shape the wire token is spelled after.
- **`tests/fixtures/worksheet_drawings.xlsx`** — a workbook with all three anchor modes, **written
  by Apache POI 5.5.1**, not by this project. Its `twoCellAnchor` carries `editAs="oneCell"`, a value
  that disagrees with the element's own name; its columns are 3.5, 20.75 and 12 characters wide and
  it states no `defaultColWidth`; its picture starts mid-cell; and it anchors a PNG on two anchors
  and a JPEG on the third. The extent POI computed for that first anchor — `cx="2085975"
  cy="885825"` — is what this project's own resolver is asserted against.
- **A guide page**, `Worksheet drawings`, whose every snippet is a compiled doctest.

### Changed

- **`dml-spreadsheetDrawing` moved from `CHILD_ORDER_SCHEMA_DEPENDENCIES` to
  `CHILD_ORDER_SCHEMAS`** — the third schema to make that move, after `dml-wordprocessingDrawing`
  and `shared-math` — and its `UNCOVERED_SCHEMAS` row is gone, because a schema covered in both
  tables has no row there.
- **`mjx-schema-gate` gains the `xdr` arm.** Without it a drawing part reports `Uncategorised`,
  which reads like a pass; that is how `mjx-vml` sat unvalidated. Both halves are proved live: a
  stray `xdr:col` inside a `twoCellAnchor` fails validation naming the part, and moving
  `xdr:clientData` to the front of an anchor turns the ordering audit red naming
  `CT_TwoCellAnchor`.
- **`NON_XML_CONTENT_TYPES_UNDER_XL` gains `image/jpeg`**, and `image/png`'s reason now names two
  kinds of part rather than one. The list is keyed on the content type rather than on where the part
  sits, so a PNG under `xl/media/` needed no row of its own — the fixture carries a JPEG so that the
  media path is not proved by one format and assumed for the rest.
- **`XlsxError::UnrecognizedImageFormat`** is new; A9's exhaustive `classify_xlsx` refused to compile
  without an arm for it.

## [0.0.123] - 2026-09-06

**Charts reach the Word surface** (MJXOFF-103, Phase E position 2): a `c:chart` inside a
`w:drawing`, read, authored and edited, under the method names `mjx-pptx` already uses.

### Added

- **`Document`'s chart family — 42 methods**, in `crates/mjx-docx/src/document/charts.rs`. Reading
  (`chart_series`, `chart_kinds`, `chart_axes`, `chart_title`, `chart_legend`, `chart_style_id`,
  `chart_data_labels`, `chart_point_formats`, `chart_trendlines`, `chart_error_bars`,
  `chart_dangling_decoration`), authoring (`add_chart`, `add_chart_placed`), the embedded workbook
  (`chart_workbooks`, `refresh_chart_workbook`, `detach_chart_workbook`) and the whole edit and
  decoration tier. A chart is addressed by its drawing's own `wp:docPr` id — the address MJXOFF-131
  already gave every Word drawing — rather than by a second scheme.
- **`mjx_chart::chart_ops`** — every read and every edit a host surface performs on a chart, stated
  **once**, over a `ChartSpace`. `mjx-pptx` and `mjx-docx` are both rank 3.0, so neither may reach
  the other; the shared body had to move *down* to the crate that owns `c:chartSpace` or be written
  twice and kept in step by hand. Both surfaces are now thin wrappers around it: resolve an address
  to a chart part, call the identically-named function, refresh the workbook. `ChartAccessError` is
  its error type, deliberately **not** `#[non_exhaustive]` so both hosts must map it exhaustively.
- **`mjx_dml::GraphicData::for_chart` / `chart_relationship_id`, and `CHART_GRAPHIC_URI`** — the
  DrawingML envelope a chart reference sits in, which is the same envelope in every format.
- **`ChartPlacement` / `ChartWrap`** — inline or floating, with three of `EG_WrapType`'s five wrap
  modes. `wrapTight`/`wrapThrough` are left off the authoring surface deliberately: both need a
  `wp:wrapPolygon` whose coordinate space ECMA-376 does not state for `CT_WrapPath`, and guessing one
  would put a wrong polygon in every document. Reading either is unaffected.
- **`mjx_dml::wordprocessing_drawing::WrapSquare::new` and `WrapTopAndBottom::new`** — MJXOFF-131
  modelled all five wrap modes and gave constructors to two of them; `Anchor::new` had no caller at
  all until this child became its first.
- **`tests/fixtures/chart_in_word.docx`** — a `.docx` carrying a chart, **written by Apache POI
  5.5.1**, not by this project. It numbers its drawing `0`, ships no `word/styles.xml`, spells
  booleans `false` where this library writes `0`, and names its workbook
  `Microsoft_Excel_Worksheet1.xlsx` where this library writes `Microsoft_Excel_Sheet1.xlsx`.
- **The Word guide's chart page** (`crates/mjx-docx/docs/guide/charts.md`), three compiled doctests.
- **Both bindings** gain the family: `document.add_chart(...)` in Python,
  `document.addChart(...)` in TypeScript, with `ChartWrap`, `DocumentChartWorkbook` and `WrapText`
  projected alongside.

### Fixed

- **`Document::remove_drawing` left a chart's relationship dangling.** It looked only for a
  *picture's* image relationship, so removing a chart drawing left `word/_rels/document.xml.rels`
  pointing at a chart part nothing referenced — which `Package::validate` reports as a defect on the
  next `save`. It now sweeps a chart's relationship, and with it the workbook that chart part alone
  referenced.
- **The child-order audit never descended into an embedded workbook, in any format.** The validation
  half of the schema gate has opened a chart's `.xlsx` since A5; the ordering half walked the outer
  package only. `sml` has been in `CHILD_ORDER_SCHEMAS` since MJXOFF-132 and `mjx-sml`'s writer
  composes those parts, so nothing was checking the order of markup this project writes.
  `audit_deck_order` now descends, under the same `…xlsx!/…` naming the validation half uses — which
  closes the hole for `mjx-pptx` in the same commit that found it from Word.

### Changed

- **`ChartSeriesData`, `ChartAxisData`, `ChartLegendData`, `ChartLabelScope`,
  `ChartPointFormatData`, `ChartTrendlineData` and `ChartErrorBarData` moved from `mjx-pptx` to
  `mjx-chart`** (`mjx_chart::view`). They were declared in `mjx-pptx` because a chart was reachable
  from one surface; two surfaces at the same rank cannot share a type that lives in either. **No
  public path changed**: `mjx-pptx` re-exports all seven, and `mjx-ooxml` now names them from
  `mjx-chart` instead.
- **`mjx-pptx`'s chart methods are delegations.** Every one keeps its signature, its error variants
  and its documentation, and calls `mjx_chart::chart_ops` for the body. `PptxError` gains an
  exhaustive `From<ChartAccessError>`.
- **A7d's chart-part re-flow limitation is gone, and the Word path inherits that.** MJXOFF-143
  carried the source span through `FromXml`/`ToXml`; measured here on a producer-written part,
  moving a chart's legend changes the one attribute and leaves the other 2,962 bytes identical.
  `crates/mjx-docx/tests/charts.rs` asserts it rather than the CHANGELOG claiming it.

## [0.0.122] - 2026-09-06

**The workspace's one sanctioned duplicate is deleted: a chart's embedded workbook is written by
`mjx-sml`** (MJXOFF-99, Phase E position 1).

### Removed

- **`crates/mjx-chart/src/workbook.rs`** — 686 lines of minimal SpreadsheetML writer, and the public
  items listed under *Unreleased — 0.1.0* above. It opened by naming its own executioner: *"a
  duplicate with a scheduled removal is a debt; a duplicate nobody removes is an architecture."*
  Written because a chart embeds a whole `.xlsx` package at `/ppt/embeddings/*.xlsx` and no
  SpreadsheetML crate existed, it proposed `mjx-xlsx` as its replacement — which would have been an
  **upward** edge (2.2 → 3.0). `mjx-sml` is rank 2.1, so `mjx-chart → mjx-sml` points down, and that
  is the edge the deletion rides on.
- **`crates/mjx-chart/tests/workbook_parity.rs`** — MJXOFF-112's gate, which existed only to compare
  the two writers byte for byte. With one writer left there is nothing to compare; everything it
  asserted about the surviving writer is also asserted in `crates/mjx-sml/tests/package_writer.rs`.
- **`mjx_chart`'s private `column_letters`.** A chart's `c:f` formulas name their columns through
  `mjx_sml::address::column_letters`, so the chart and its workbook cannot disagree about which
  column is which.

### Changed

- **`mjx-chart` holds no SpreadsheetML at all** — not an element name, not an `xl/` part name, not a
  namespace constant, not in a test. `crates/mjx-chart/src/embedding.rs` decides only *which cell* a
  chart's data belongs in and hands the rows to `mjx_sml::write::WorkbookPackage`. That is the whole
  crate's involvement with spreadsheets now.
- **`mjx-pptx` registers an embedded workbook with `mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE`**
  and gained a direct `mjx-sml` dependency for it (3.0 → 2.1, downward). `add_chart` and
  `refresh_chart_workbook` keep their shape exactly: everything fallible that does not touch the
  package still happens first, and a refresh still answers `false` rather than erroring for a chart
  with no `c:externalData`, an unresolvable relationship, an `External` target mode or a missing part.
- **`mjx_sml::write::WorkbookPackage::push_row` advances past a row that writes nothing.** Fixed
  forward here rather than worked around in `mjx-chart`. `Blank` and a non-finite number write no
  cell, so a row of them left no `<row>` behind — and the next row's number was measured off the
  *populated* rows, so it took the empty row's place and slid the whole grid up by one. A chart read
  back from a file whose series carry no `c:tx` has exactly that header row, and its own `c:f` says
  `Sheet1!$A$2:$A$3`. `AuthoredWorksheet::appended_row_count` is the new cursor, public and
  documented, and `set_cell_value` still counts, so mixing the two doors never overwrites.

### Fixed

- **`the_refreshed_workbook_holds_the_edited_values` could not fail for the thing it names.** It set
  a series' values *and* its categories, and `set_chart_series_categories` refreshes the workbook
  too — so removing `set_chart_series_values`'s refresh entirely left it green. Found by mutation
  while rerouting the writer. It is now one test per setter, each asserting that the labels or the
  numbers the fixture carried are *gone*, and each independently red when its own refresh is removed.
  A11's R4 (`editing_a_chart_dirties_only_the_chart_xml_and_its_workbook`) always caught the values
  case, so nothing was unguarded; one of the two guards was simply not the guard it read as.

### Documentation

- **The gaps page's standing paragraph is a closed *What used to be here* row**, naming `mjx-sml`
  rather than `mjx-xlsx` — the original sentence named the wrong crate, and the edge it implied was
  illegal.
- **`xtask/src/corpus/xlsx.rs` states, at the call site, why its hand-written worksheet stays.** It is
  the fourth writer of SpreadsheetML in this repository and the only one left; MJXOFF-93 reported it
  and left the decision open. It is kept on purpose: it is the *input* to a benchmark of the library's
  reader, so generating it through the library would make `docs/BENCHMARKS.md` a measurement of our
  reader against our own writer; it writes a file `WorkbookPackage` cannot (no shared strings, no
  styles, `t="inlineStr"`, `spans` on every row); building it through the model would pay the cost
  the harness exists to measure; and `xtask` is a host-only binary outside the ranked graph that
  ships nowhere. **So the workspace's "one sanctioned duplicate" claim is retired with the writer it
  described: exactly one SpreadsheetML writer ships, and the remaining hand-written one is tooling.**

## [0.0.121] - 2026-09-06

**Excel through the facade and both bindings — and one API decision made from a measurement rather
than from taste** (MJXOFF-137, Phase D position 20; **Phase D complete**).

### Added

- **`mjx_ooxml::Workbook`** — the curated Excel surface, eleven modules mirroring `deck/`'s and
  `document/`'s split. `detect_format` already answered `Format::Workbook`; it now yields a workbook
  that opens, reads, edits, validates and saves. Tabs, cells, geometry, cell formats, hyperlinks,
  tables, defined names, print setup, the preserved-part reports and the part graph, all with
  concrete types: **A1 text for every address** (`"B7"`, `"A1:C3"`), `u32` for every index, `&str`
  for every part name.
- **`mjx_ooxml::Workbook` in Python and TypeScript**, and the classes their arguments and results are
  made of — twenty-nine value classes and nineteen enumerations each. The walkthrough exists three
  times (`crates/mjx-ooxml/examples/build_a_workbook.rs`,
  `bindings/mjx-python/tests/test_build_a_workbook.py`,
  `bindings/mjx-wasm/tests/node/build_a_workbook.mjs`) and the two bindings are compared against the
  Rust one **part by part, byte for byte**.
- **[Through the facade and the bindings](crates/mjx-xlsx/docs/guide/through_the_facade.md)**, the
  Excel guide's fourteenth page: the translation table, the range decision, and what the facade does
  not carry.
- **`crates/mjx-ooxml/benches/workbook_boundary.rs`** — the instrument MJXOFF-135's figures did not
  have. `docs/BENCHMARKS.md`'s own harness drives `Package::part_tree_mut`, which `mjx-xlsx` never
  calls, so it could not see this cost at all.
- **`mjx_allocation_counter::total_allocated`** — a monotonic byte counter. `peak` and `live` cannot
  tell one parse from two hundred parses that each free before the next; this can, and it is what
  lets the boundary gate below be deterministic instead of a stopwatch.

### The range decision

**There is no per-cell reader or writer on the facade, or in either binding.** `mjx_xlsx::Workbook`
holds no parsed worksheet, so every per-sheet accessor re-parses the part: measured on the
300,000-cell corpus, in release, **405 ms to read one cell** against **12.0 ms to open the whole
file**, and a 4,000-cell write loop against 410 ms for the same 4,000 cells batched. The Rust answer
— hold the `WorksheetPart` yourself — cannot cross a foreign function boundary, so a facade with
`cell_value(sheet, "A1")` would ship the slow loop as the natural idiom in the one place a caller
cannot reach past it.

So the cell door is a **range** in both directions: `read_range`, `read_sheet` and `write_cells`
parse once each, whatever they are asked for. `crates/mjx-ooxml/tests/workbook_boundary.rs` holds
that to a **deterministic allocation ratio** — 209x for the write, 199x for the read — because a
fallback to per-cell would produce the same file, byte for byte, and no correctness gate could see
it. The per-cell calls stay reachable from Rust through `Workbook::workbook_mut`.

### Changed

- **`Format::is_editable` is now true for every format but `Format::WorkbookBinary`.** `.xlsb` is
  refused with its own message — its main part is the MS-XLSB binary record stream, not
  SpreadsheetML — and that refusal is a design decision rather than a schedule.
- **`mjx_ooxml::Error`'s mapping reaches three enumerations further.** `classify_xlsx` names every
  `XlsxError` variant and, through `sml_code` and `address_code`, every `SmlError` and
  `AddressError` variant, with **no wildcard arm anywhere** — so a new failure mode is a compile
  error rather than a silent `Unknown`.
- **`mjx_sml::GridAnomaly` is no longer `#[non_exhaustive]`**, for the reason the error enumerations
  never were: the facade's flat `GridAnomalyInfo` projection must be a compile-time gate.

### Fixed

- **`crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` and
  `crates/mjx-docx/docs/guide/fidelity_and_gaps.md` both claimed `mjx-docx`/`mjx-xlsx` "have no
  editing surface — they are scaffolds", and one promised a removal "once `mjx-xlsx` can write
  (`v0.3`)".** All false, and they are published docs.rs pages. Corrected to what is true today; the
  `mjx-chart` embedded-workbook writer's removal condition is now *met*, and the removal itself is
  MJXOFF-99's work.

### Handed over

- **`docs/EXCEL_FACADE_HANDOFF.md`** — twenty-six checklist entries for MJXOFF-128 (F2), **every one
  unmarked**, plus what has no runtime coverage anywhere. No agent has Excel.

## [0.0.120] - 2026-09-06

The Excel usage guide and its runnable examples — every snippet compiled, every example asserting
(MJXOFF-135, Phase D position 19).

### Added

- **Two guide pages**, bringing `crates/mjx-xlsx/docs/guide/` to thirteen. The other eleven were
  written by the children that shipped the features they describe; these two had no owner:
  - **[Large workbooks](crates/mjx-xlsx/docs/guide/large_workbooks.md)** — the memory model in a
    caller's terms. What a sparse sheet costs (nothing proportional to the grid), what a populated
    cell costs (**36.8 B**, or **76.8 B** with a formula, against a `RawElement` tree's **802 B**),
    and the awkward figure: on the 300,000-cell corpus workbook, `Workbook::open` is **14.8 ms and
    16.8 MB**, while **the first `worksheet_markup` is 507 ms with a 269 MB allocation peak — and it
    is paid again on every subsequent per-sheet call**, because `Workbook` holds no parsed worksheet.
    Filling a sheet through `set_cell_value` is therefore quadratic: 4,000 cells one at a time is
    **18.39 s** against **1.12 ms** for one read, N edits and one write, a measured 16,427×. The page
    says so, states the shape to reach for, and names the one `Arc` copy that cannot be avoided
    because `mjx_opc::Package::part_bytes` answers `&[u8]` rather than shared bytes.
  - **[Deliberate limitations](crates/mjx-xlsx/docs/guide/deliberate_limitations.md)** — the page to
    read before filing a bug. The two-crate split (which of `mjx-sml` and `mjx-xlsx` to reach for,
    and why the split is what makes `mjx-chart → mjx-sml` legal), the three standing refusals that
    all follow from having no calculation engine (stale cached values, unevaluated
    conditional-formatting conditions, unapplied filters/sorts/validation), the half of `sml.xsd`
    preserved rather than modelled, and a closing list of what is genuinely **absent and unowned**
    rather than deliberately refused.
- **Six runnable examples** under `crates/mjx-xlsx/examples/`, which had none at all. Each reopens
  its own output and asserts on it, and CI's `examples` job picks them up by directory:
  `build_a_workbook` (two tabs, shared strings, a format, resolved back through the reopened
  stylesheet), `edit_a_workbook` (**exactly two part payloads may differ** after one cell edit and
  one rename, checked part by part), `read_formulas` (eleven formula cells, the five-member shared
  group whose text lives on one of them, and the cached value that is **still 2** after its
  dependency became 50), `style_a_range` (one `xf`, nine cells, and a tenth outside that must not
  wear it), `table_and_autofilter` (a table part, its relationship, its `tablePart` entry, and a
  filter that hides no row) and `large_sparse_sheet` (a counting global allocator, one cell at
  `XFD1048576` under a 32 KiB bound, and 30,000 cells under the 48 B/cell bound).

### Changed

- **`README.md`, `PLAN.md` and the `mjx-ooxml` docs hub** point at the Excel guide. The README's
  guide and example sections had been left at PowerPoint's alone — Word's five pages and nine
  examples shipped in MJXOFF-150 without being linked — so all three formats are now listed, and the
  format-support table no longer calls Word and Excel *planned*.
- **`mjx-sml`'s crate documentation** gains a pointer to the two guide pages that explain it to a
  caller, and its rank table is corrected: it listed `mjx-xml` at rank 0.0 beside `mjx-ooxml-core`
  and `mjx-derive`, where `CLAUDE.md` and `xtask/tests/layering.rs` both put it at **0.1** in a row
  of its own.

### Fixed

- **Four stale prose counts and one stale forward reference**, all documentation:
  - the Excel guide README said *"Ten pages"* over a table of eleven, and its arc stopped at
    MJXOFF-127 (D16) although D17 and D18 had shipped;
  - `the_sheet_grid.md` said **eighteen** of `CT_Worksheet`'s thirty-nine slots were modelled and
    twenty-one held, a figure last true at MJXOFF-125; MJXOFF-127 and MJXOFF-129 have since taken it
    to **thirty-one modelled, eight held**, which is what `crates/mjx-sml/src/worksheet/frame.rs`
    itself says;
  - `authoring_a_workbook.md` said *"Setting a formula is MJXOFF-115's"*. MJXOFF-115 shipped, and
    modelled formulas for **reading and preservation only** — there is no `set_cell_formula` on
    `Workbook` or on `mjx_sml::SheetData`, and no later child owns adding one. The page now says that
    plainly and the limitations page lists it as an unowned gap;
  - `mjx_ooxml::FormatFamily::WordProcessing` was documented *"Detected, not yet editable"* after
    Word became fully editable, and `Spreadsheet` said the same of Excel.

## [0.0.119] - 2026-09-06

The half of `sml.xsd` this project deliberately does not model — recognised, reported and proved to
survive (MJXOFF-133, Phase D position 18).

### Added

- **`mjx_sml::preserved`** — read-only *identity views* over the parts of the nine unmodelled
  clusters: `PivotTableIdentity` (name, cache id, `CT_Location@ref`), `PivotCacheIdentity` /
  `PivotCacheSource`, `ExternalLinkIdentity` / `ExternalLinkTarget`, `ConnectionIdentity`,
  `QueryTableIdentity`, `XmlMapsIdentity` / `XmlMapIdentity`, `RevisionHeadersIdentity` /
  `RevisionSession`, and `SharedWorkbookUsersIdentity` / `SharedWorkbookUser`. **None of them
  implements `ToXml`**, so there is no path by which reading one changes a byte — which is the
  difference between identifying a part and modelling it.
- **`mjx_xlsx::Workbook::preserved_parts`** and `PreservedParts` — every preserved part of a
  workbook, resolved from relationships alone with **no markup parsed at all**. Beside it, the typed
  reports: `pivot_tables`, `external_links`, `connections`, `query_tables`, `xml_maps` and
  `revision_state`, with `SheetPivotTable`, `WorkbookExternalLink`, `WorkbookConnection`,
  `SheetQueryTable`, `WorkbookXmlMaps` and `RevisionState`. A caller can ask *"does this workbook
  have pivot tables, and where are they?"* and get the sheet, the range, the cache and every part
  name — **without a ninety-seven-type model** behind it.
- **`mjx_xlsx::PartKind` now names every ECMA-376 Part 1 §12.3 part type** — the six MJXOFF-91 left
  out are in: `CustomProperty` (§12.3.5), `CustomXmlMappings` (§12.3.6), `RevisionHeaders`
  (§12.3.16), `RevisionLog` (§12.3.17), `SharedWorkbookUserData` (§12.3.18) and
  `SingleCellTableDefinitions` (§12.3.19). Twenty-seven kinds in all. **Recognising a part is not
  modelling it:** every one of them is still carried through a save as the bytes it arrived as.
- **`PartKind::from_relationship_type`** and `parts::AMBIGUOUS_CONTENT_TYPES`. Two §12.3 part types
  are not identified by their content type — a Custom Property part carries *"any content, support
  for which is application-defined"*, and the Custom XML Mappings part carries plain
  `application/xml`, which in a real package is also the `Default` for every `.xml` part with no
  `Override`. `preserve::classify` therefore asks the content type first and the relationship graph
  second, and `from_content_type` refuses to answer from an ambiguous string rather than
  misidentifying `docProps/custom.xml` as an XML map.
- **New part-graph edges**: `WorkbookParts::custom_xml_mappings` / `revision_headers` /
  `shared_workbook_user_data`, `WorksheetParts::custom_properties` /
  `single_cell_table_definitions`, and the three sub-graphs `PivotTableParts` (a table to its
  cache), `PivotCacheParts` (a cache to its records) and `RevisionHeadersParts` (the headers part to
  its logs).
- **`SpreadsheetDefect::WorkbookReferenceTargetIsWrongKind`** — a `pivotCaches/pivotCache@r:id` or
  `externalReferences/externalReference@r:id` that leads to a part of the wrong kind. The direction
  packaging cannot see, for the two lists in `CT_Workbook`'s sequence that point outward at parts:
  those parts are preserved and unmodelled, which is exactly why nothing else here would notice one
  going stale.
- **`tests/fixtures/preserved_parts.xlsx`** — a workbook carrying one part of every cluster at once,
  and `crates/mjx-xlsx/tests/preserved_parts.rs`, which edits a cell **on the very sheet the pivot
  table sits on** and compares all fourteen preserved parts against the bytes they went in with.
  *A part nothing asserts on is a part that silently disappears.*

### Documented

- **The scope decision, in writing.** `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md`
  carries a cluster-by-cluster table: 184 of `sml.xsd`'s 367 complex types, what "preserved"
  guarantees, what it does not, and **why** for each. The pivot cluster alone is 97 types — a
  quarter of the schema — and it is derived data of a calculation model this project does not have:
  **if it is ever modelled it is a phase of its own**, not a gap for a later child. A documented gap
  is never a validation failure.
- **`CT_Worksheet`'s ownership table was stale and is now re-derived from the code.**
  MJXOFF-129 typed six slots (13, 19–22, 33) without updating
  `crates/mjx-sml/src/worksheet/frame.rs`'s module table, which still described 25 modelled and 14
  held. It is **31 modelled and 8 held**, and three of the eight — `phoneticPr` (15),
  `legacyDrawingHF` (31) and `drawingHF` (32) — belong to no ticket at all, each with the reason it
  does not. All nineteen of `CT_Workbook`'s slots are modelled.
- **A place where ECMA-376 contradicts its own schema**, recorded as a tolerated deviation rather
  than papered over: `CT_Schema` (`sml.xsd:340`) is one required child under a **strict** wildcard,
  and what §12.3.6's own example puts there is an inline XML Schema document, which no schema
  `sml.xsd` imports declares. `xl/xmlMaps.xml` is therefore a part no conformant instance can
  satisfy — omitting the child breaks `minOccurs`, supplying the specification's own one breaks the
  wildcard.

## [0.0.118] - 2026-09-06

Print setup, headers and footers, custom views, and the three sheet kinds that are not worksheets
(MJXOFF-129, Phase D position 17).

### Added

- **`mjx_sml::features::print`** — the markup every sheet *kind* carries. `CT_PrintOptions`
  (`PrintOptions`), `CT_PageMargins` (`PageMargins`, in inches, all six `use="required"`),
  `CT_PageSetup` (`PageSetup`), `CT_CsPageSetup` (`ChartSheetPageSetup` — the same element name on a
  chartsheet and a *different complex type*, without the six attributes that only mean something
  over a grid), `CT_HeaderFooter` (`HeaderFooter`, `HeaderFooterText`, `HeaderFooterSlot`,
  `HeaderFooterSection`) and `CT_SheetBackgroundPicture` (`SheetBackgroundPicture`).
  **Nothing paginates:** `fitToWidth` is reported, and where a page breaks is rendering.
- **`mjx_sml::features::custom_views`** — `CT_CustomSheetViews`/`CT_CustomSheetView`, filling
  `CT_Worksheet`'s rank 13. The nine children come from four existing clusters — MJXOFF-102's pane
  and selection, MJXOFF-117's breaks, MJXOFF-123's autofilter and this child's print block — and not
  one of them is modelled a second time. **A record, never applied:** a view's `@hiddenRows` is that
  view's memory, not the sheet's rows.
- **`mjx_sml::sheets`** — `CT_Chartsheet` (`ChartSheetPart` and its seven-type cluster),
  `CT_Dialogsheet` (`DialogSheetPart`) and `CT_Macrosheet` (`MacroSheetPart`), over one shared part
  frame with the same slot-level copy-on-write, generated placement and byte writer `WorksheetPart`
  has. **A chartsheet has no cell accessor at all** — the absence is in the type, so asking one for
  its cells does not compile.
- **Six more `CT_Worksheet` slots are typed** rather than held: `customSheetViews` (13),
  `printOptions` (19), `pageMargins` (20), `pageSetup` (21), `headerFooter` (22) and `picture` (33).
  Thirty-one of the thirty-nine are now modelled; `phoneticPr` (15) is the only one left that
  belongs to nobody.
- **`mjx_xlsx::SheetMarkup`** and `Workbook::sheet_markup` / `sheet_markup_of` /
  `write_sheet_markup` — the markup behind any tab, whichever of the four kinds it is. A macrosheet
  is dispatched on its **root element**, because ECMA-376 declares no content type for one, so
  `SheetKind` still reports the three kinds §12.3.23 names.
- **`Workbook::sheet_printer_settings` and `Workbook::sheet_background_image`** — the part a sheet's
  own `pageSetup@r:id` and `picture@r:id` reach, resolved against that sheet part's `.rels`. Neither
  part is ever opened: a printer-settings blob is a Windows `DEVMODE` ECMA-376 Part 1 §15.2.13
  places no requirement on, and an image is bytes.
- **`mjx_xlsx::REL_IMAGE`** and `WorksheetParts::background_image` — the image relationship a sheet's
  background picture reaches (Part 1 §15.2.14).
- **`SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType`** — a `pageSetup` or `picture`
  naming a relationship of the wrong *type*. The half `mjx_opc`'s dangling-reference check cannot
  see: the id is declared, so only a reader that knows what a `pageSetup` means can tell it points
  at the wrong kind of part. Reported for markup this library will write, never for markup it merely
  opened.
- **`tests/fixtures/print_and_sheet_kinds.xlsx`** — a worksheet with the full print block, a
  `customSheetView` carrying all nine of its children, two printer-settings blobs, a background
  image and a dialogsheet. Every header/footer string in it is a different shape (a quoted font
  name, a `&G`, a literal `&&`, a character reference, a CDATA section), because those are the five
  ways a re-serialising writer changes a file.
- **`crates/mjx-xlsx/docs/guide/print_setup_and_sheet_kinds.md`** — the guide page, six compiled
  doctests.

### Changed

- **`mjx-xlsx`'s `no_part_under_xl_is_skipped_as_foreign_or_uncategorised` now distinguishes two
  kinds of skip.** A part whose *payload is not XML* has nothing a schema could be applied to, so
  skipping it is correct; a part whose *root namespace has no arm* is the false green MJXOFF-110
  exists to close. The guard rejected both. It now accepts a `SkippedBinary` whose content type is
  on a pinned, reasoned allowlist and rejects every other outcome exactly as before — with two new
  cases proving it: one feeds the rule each shape of false green and asserts it is still rejected,
  the other fails an allowlist entry no committed fixture witnesses.

- **`tests/fixtures/hyperlinks.xlsx`'s custom-property part moves from `/customProperty1.bin` to
  `/xl/customProperty1.bin`.** MJXOFF-127 put it at the package root because the guard above
  rejected a binary part under `xl/`, and said so at the time. A Custom Property part's target is
  relative to the workbook (ECMA-376 Part 1 §12.3.5), so `xl/` is where it belongs; with the guard
  fixed, the workaround goes. The part's bytes and the sheet markup that names it are unchanged.

### Fixed

- **A header or footer string's own spelling survives an edit elsewhere in the part.** The
  `#[xml(text)]` escaping gap the epic recorded as latent becomes live here — `&#65;` decodes to
  `A`, a CDATA section decodes to its contents, and a rebuilt text node that differs from the
  original denies its element and every ancestor of it the verbatim source range subtree
  copy-on-write would give it. `HeaderFooterText` therefore has the hand-written `FromXml`/`ToXml`
  pair `DefinedName` has: it replays the file's own children until `set_text` replaces them. The gap
  itself is still in the derive, and still owned by no work item.

## [0.0.117] - 2026-09-06

Hyperlinks, the object-anchor vocabulary three Phase E children share, and the last small worksheet
children nothing else in Phase D had claimed (MJXOFF-127, Phase D position 16).

### Added

- **`mjx_sml::features::hyperlinks`** — `Hyperlinks` (`CT_Hyperlinks`, `sml.xsd:2739`) and
  `Hyperlink` (`CT_Hyperlink`, `sml.xsd:2744`), filling **rank 18 of `CT_Worksheet`**. `@ref` is an
  `ST_Ref`, so **a hyperlink covers a range and not a cell**, and nothing splits a `B4:D6` entry into
  one link per cell. `CT_Hyperlink` declares `@r:id` and `@location` both optional, so there are
  three shapes and not two — and **an entry carrying both is a real file Excel writes, not a defect
  to clean up**. Nothing here drops either because the other is present.
- **`mjx_sml::features::objects`** — `ObjectAnchor` (`CT_ObjectAnchor`, `sml.xsd:238`) and
  `ObjectProperties` (`CT_ObjectPr`, `sml.xsd:3063`). **Modelled in `mjx-sml` on purpose**: `sml.xsd`
  reaches `CT_ObjectAnchor` from `CT_CommentPr` (MJXOFF-114, E5), `CT_ObjectPr` (MJXOFF-107, E3) and
  `CT_ControlPr` (both), so one type in the shared-markup tier is what stops three Phase E children
  inventing three. The two `xdr:from` / `xdr:to` markers are **held as raw elements** — `CT_Marker`
  is `dml-spreadsheetDrawing.xsd`'s and MJXOFF-107 models it — while their *placement* goes through
  the generated `OBJECT_ANCHOR` table, which ranks them across a namespace boundary no `sml` local
  name reveals. Nine of `CT_ObjectPr`'s twelve attributes are booleans and **six default to `true`**,
  which is the opposite of every other flag family in `sml.xsd`.
- **`mjx_sml::features::annotations`** — `CellWatches`/`CellWatch` (rank 26),
  `IgnoredErrors`/`IgnoredError` (rank 27), and `SmartTags`/`CellSmartTags`/`CellSmartTag`/
  `CellSmartTagProperty` (rank 28). An `IgnoredError` says *do not draw the indicator*, never *there
  is no error*; nothing here evaluates a cell. `CT_CellSmartTag`'s `@deleted` is read through
  `smart_tag_was_deleted`, and `.github/scripts/check-suppress-naming.sh` gains an allow-list entry
  for it — scoped to that exact token in that exact file — because a tag the user removed is still
  written out, which is `CT_InputCells@deleted`'s situation and not the chart family's *"draw nothing
  here"*. The worksheet `smartTags` cluster is **not**
  `xl/workbook.xml`'s near-identically-named `smartTagTypes`, which MJXOFF-100 modelled.
- **`mjx_sml::features::publishing`** — `DataConsolidation`/`DataReferences`/`DataReference`
  (rank 12), `CustomProperties`/`CustomProperty` (rank 25) and `WebPublishItems`/`WebPublishItem`
  (rank 36). A consolidation is a **record**, never performed. A `webPublishItem@destinationFile` is
  an untrusted path on somebody else's disk, carried exactly and never opened.
- **Seven new `WorksheetPart` slots** — `hyperlinks`, `data_consolidation`, `custom_properties`,
  `cell_watches`, `ignored_errors`, `smart_tags` and `web_publish_items`, each with its `_mut` and
  `set_` companions, plus **`hyperlink_covering`, `hyperlink_position_covering`, `add_hyperlink` and
  `remove_hyperlink`**. **Twenty-five of `CT_Worksheet`'s thirty-nine slots are now modelled and
  fourteen held raw**, and the frame's own documentation now names the owner of every one of the
  fourteen.
- **`Workbook::sheet_hyperlinks`, `cell_hyperlink`, `set_cell_hyperlink`, `remove_cell_hyperlink`,
  `add_hyperlink_relationship`**, plus **`SheetHyperlink`**, **`HyperlinkTarget`** and
  **`HyperlinkKind`**. `HyperlinkTarget::Url` keeps `mjx_pptx::Hyperlink::Url`'s name exactly;
  Excel's *internal* kind is a `@location` string and **no relationship at all**, unlike
  PowerPoint's slide jump. **A hyperlink and its relationship are one thing**: setting one writes
  both, removing one removes both, and a relationship survives only while another entry still names
  it.
- **`SpreadsheetDefect::OrphanedHyperlinkRelationship`** — the half packaging cannot state.
  `mjx-opc` reports a dangling `r:id` and is explicit that a relationship nothing names is legal (a
  `comments` relationship is found by *type*), so this fires for the **`hyperlink` relationship type
  alone**, over worksheets this library will write.
- **`mjx_xlsx::parts::REL_HYPERLINK`** — the one relationship in that file that reaches no part.
- **Four generated child-order exports** — `WORKSHEET_IGNORED_ERRORS`, `OBJECT_ANCHOR`,
  `OBJECT_PROPERTIES` and `DATA_CONSOLIDATION` in `mjx_ooxml_types::child_order`, from `xtask`'s
  curated list.
- **`tests/fixtures/hyperlinks.xlsx`** — **all three hyperlink shapes on one sheet**, because one
  kind tests one branch: an internal `@location` over the multi-cell `B4:D6`, an external `@r:id`,
  and one carrying both. The two external entries name `rId2` then `rId1`, **the reverse of the
  `.rels` order**; the `mailto:` target spells its query `%20` and `&amp;`, so any normalisation of
  an untrusted URI shows; one `@display` is **single-quoted**; `dataConsolidate@function` is
  `stdDev`, whose generated variant is `SampleStandardDeviation`; `dataRefs@count` and
  `webPublishItems@count` are both **stale**; one `dataRef` states no attribute at all; the two
  `cellWatch`es are out of address order; `ignoredErrors` carries an `extLst` after its records; and
  the sheet holds an unmodelled `customSheetViews` (rank 13) and `phoneticPr` (rank 15) so the
  modelled and held slots genuinely interleave.
- **A guide page** — *Hyperlinks*, with three compiled doctests.

### Changed

- **`Workbook::next_sheet_relationship_id` is now `pub(crate)`** (it was private to
  `worksheet/tables.rs`). Hyperlinks allocate from the same sheet `.rels`, and a second allocator
  could hand out an id the first had already promised.

### Notes

- **`customSheetViews` (rank 13) is explicitly preserved, and it is MJXOFF-129 (D17)'s**, which names
  `CT_CustomSheetViews`/`CT_CustomSheetView` in its own work list. `CT_CustomSheetView` embeds
  `pageMargins`, `printOptions`, `pageSetup` and `headerFooter` — D17's own print block — so
  modelling it here would have meant a second copy of it or a model holding it raw twice over.
- **Three `CT_Worksheet` slots still have no owner**: `phoneticPr` (15), `legacyDrawingHF` (31) and
  `drawingHF` (32). `CT_PhoneticPr` is already modelled once as `PhoneticProperties`, a value decoded
  from the shared-string store's packed bytes rather than a `RawElement`-backed slot, so giving rank
  15 a type means unifying two call sites — a design question, not a slot to fill. The other two are
  the header/footer half of the drawing family and belong with E3's and E5's. All three round-trip
  verbatim today; `crates/mjx-sml/src/worksheet/frame.rs` records this table for MJXOFF-133 (D18).

## [0.0.116] - 2026-09-06

Worksheet tables — the first feature of Phase D that lives in a **part of its own**, and the first
place this library creates one (MJXOFF-125, Phase D position 15).

### Added

- **`mjx_sml::features::tables`** — `WorksheetTable` (`CT_Table`, `sml.xsd:3946`, the root of
  `xl/tables/tableN.xml`), `TableColumns`/`TableColumn`, `TableFormula` (`CT_TableFormula`),
  `XmlColumnProperties` (`CT_XmlColumnPr`), `TableStyleReference` (`CT_TableStyleInfo`), and
  `TableParts`/`TablePart` — the last of which fills **rank 37 of `CT_Worksheet`**, so eighteen of
  its thirty-nine slots are now modelled and twenty-one held raw. A table's `autoFilter` and
  `sortState` are MJXOFF-123's own `AutoFilter` and `SortState`; there is no second filter or sort
  model. **`sortState` now has three distinct homes** — rank 11 of `CT_Worksheet`, rank 1 of
  `CT_AutoFilter`, rank 1 of `CT_Table` — and they are three different elements.
- **`mjx_sml::styles::table_styles`** — `TableStyles` (`CT_TableStyles`), `TableStyleDefinition`
  (`CT_TableStyle`) and `TableStyleRegion` (`CT_TableStyleElement`). This is **rank 8 of
  `CT_Stylesheet`, the last of its eleven slots to be modelled**; only `extLst` is held raw now.
- **`builtin_table_style_name`, `BuiltInTableStyle`, `BuiltInTableStyleFamily`,
  `TableStyleLookup`, `TableStyleOrigin`** — the distinction the ticket names. Excel's 144 preset
  table styles are in **no `.xlsx` at all**, so a lookup has three answers and not two:
  locally defined, built-in, or genuinely undefined. The six contiguous families and their bounds
  are read off ECMA-376 Part 1's own `presetTableStyles.xml`, on
  `builtin_cell_style_name`'s precedent, and a unit test walks all 144 plus both boundaries.
- **`mjx_sml::WorksheetTableSpec`, `TableColumnSpec`, `TableStyleReferenceSpec`** — plain-data
  authoring descriptions with no interner, and **`mjx_sml::write::AuthoredTable`**, the whole-part
  writer, which seeds `<table xmlns="…"/>` as bytes and writes back the root it *read*.
- **`WorksheetPart::table_parts`/`table_parts_mut`/`set_table_parts`**, and
  **`WorksheetPart::bind_relationship_prefix`** — the second declares `xmlns:r` on a worksheet root
  that binds none, because a `tablePart` is nothing but an `r:id` and a sheet authored from nothing
  declares only the SpreadsheetML namespace. It never overwrites a binding the file made.
- **`StylesheetPart::table_styles`/`table_styles_mut`/`set_table_styles`**.
- **`Workbook::sheet_tables`, `table_markup`, `edit_table_markup`, `table_style_origin`,
  `next_table_id`, `add_table`**, plus the owned reports **`SheetTable`** and **`SheetTableColumn`**.
  `add_table` writes the **four things a table is** — the part, its content-type override, a `table`
  relationship from the *sheet* part, and a `tablePart` entry — in one call.
- **`SpreadsheetDefect::TablePartTargetIsNotATable`, `DuplicateTableId`,
  `DuplicateTableDisplayName`** — the directions packaging cannot see. The last two fault only a
  table **this library wrote**; a workbook that arrived with a collision still saves. A
  `tablePart@r:id` naming *no* relationship is deliberately not restated here: `mjx-opc` already
  reports it over exactly the same set of parts.
- **`SmlError::TableGeometryDoesNotFit`, `TableHasNoColumns`** — the two refusals.
- **`tests/fixtures/worksheet_tables.xlsx`** — **two tables on one sheet**, because one tests neither
  the id allocation nor the built-in/local distinction. They differ in every way that matters: ids
  **1 and 4** (a gap, so the next free id is 5 and not the table count plus one); one wears the
  preset `TableStyleMedium2` and the other the workbook's own `AcmeBlue`; one has a totals row with
  `sum`, `average` *and* `custom` and the other none; one declares `tableColumns@count="9"` against
  four columns. The sheet also lists the two **in the reverse of the relationship order**, and the
  calculated-column formula spells `>` as `&gt;` — an entity XML does not require, so re-escaping it
  would change the bytes.
- **Two generated child-order exports** — `WORKSHEET_TABLE` and `WORKSHEET_TABLE_COLUMN` in
  `mjx_ooxml_types::child_order`, from `xtask`'s curated list.
- **A guide page** — *Worksheet tables*, with four compiled doctests.

### Fidelity

- **A table's `@ref` and its two row counts move together or not at all.** There is deliberately no
  `set_range`: §18.5.1.2 makes the three one statement, so `WorksheetTable::resize` takes all three
  and refuses a combination the range cannot hold.
- **An unrelated cell edit never moves a table's boundary.** Setting a value inside a table rewrites
  one row of the worksheet part; the table part is not opened, and comes back byte for byte.
- **A table's `@id` is never reused and never renumbered.** `Workbook::add_table` allocates one past
  the highest id any table part in the package writes, and `spec.id` is ignored.
- **A calculated column is never expanded** into per-cell formulas, a totals row is never computed,
  and a structured reference (`Sales[[#This Row],[Q1]]`) is never parsed. Formulas are text on
  MJXOFF-115's terms, at two more doors.
- **A preset style name is not a missing style.** Reporting "not found" for `TableStyleMedium2`
  would be a confident wrong answer where the honest one is a distinction.
- **`tableColumns@count`, `tableStyles@count` and `tableParts@count` are producer caches** — refreshed
  when the collection is edited *and* the file declared one, never added to an element that wrote
  none, and never corrected on a read.
- **`xmlColumnPr` is preserved and never resolved**; the XML map part it names is modelled nowhere.

### Fixed

- **`crates/mjx-sml/src/styles/stylesheet.rs` said MJXOFF-127 owned `tableStyles`.** It is
  MJXOFF-125's (D15); MJXOFF-127 is D16. Corrected in three places.

## [0.0.115] - 2026-09-06

Data validation, autofilters and sort state — a cluster whose whole discipline is that **nothing in
it is ever applied**: a filter hides no row, a sort reorders none, and a `list` validation's range
source is text this library never resolves (MJXOFF-123, Phase D position 14).

### Added

- **`mjx_sml::features::filters`** — the autofilter cluster, all thirteen complex types of
  `sml.xsd:16-228`: `AutoFilter` (`CT_AutoFilter`, rank **10** of `CT_Worksheet`), `FilterColumn`,
  `Filters`/`Filter`/`DateGroupItem`, `CustomFilters`/`CustomFilter`, `Top10Filter`, `ColorFilter`,
  `IconFilter`, `DynamicFilter`, `SortState` and `SortCondition`. It is its own module, not part of
  the worksheet, because MJXOFF-125's `CT_Table` embeds `autoFilter` and `sortState` directly and
  MJXOFF-133's pivot filters reference the same types.
- **`mjx_sml::FilterKind`** — `CT_FilterColumn`'s `xsd:choice` as a Rust enum rather than six
  `Option` fields. **Six modelled filter kinds**, not seven: the choice has seven *members* and the
  seventh is `extLst`, the extension slot, which lands in `FilterKind::Raw` and round-trips byte for
  byte. `FilterColumn::set_filter` replaces whichever kind is there, in its position, which is what
  a choice asks for.
- **`mjx_sml::features::validation`** — `DataValidations` (`CT_DataValidations`, rank **17**) and
  `DataValidation` (`CT_DataValidation`), all thirteen attributes and both formula slots.
  `@count` is a producer's cache and is never rewritten, on read or after an append.
- **`mjx_sml::FormulaElement`** — the `ST_Formula` **element**, one type for the three slots that
  share it (`cfRule/formula`, `dataValidation/formula1`, `dataValidation/formula2`). It carries its
  own local name. See the breaking-change row below: this replaces
  `ConditionalFormattingFormula` rather than sitting beside it.
- **`WorksheetPart::auto_filter`/`auto_filter_mut`/`set_auto_filter`**,
  **`sort_state`/`sort_state_mut`/`set_sort_state`** and
  **`data_validations`/`data_validations_mut`/`set_data_validations`**, plus the curated
  `data_validation_rules`, `data_validations_for`, `add_data_validation` and
  `remove_data_validation`. Three more of `CT_Worksheet`'s thirty-nine slots are modelled —
  seventeen now, twenty-two held raw — and the third is the one easy to miss: **`sortState` is a
  slot of the worksheet at rank 11 as well as a child of `autoFilter` at its own rank 1**, two
  different elements of the same complex type.
- **`mjx_sml::AutoFilterSpec`** and its four companions (`FilterColumnSpec`, `FilterSpecKind`,
  `CustomFilterSpec`, `SortStateSpec`, `SortConditionSpec`) and **`mjx_sml::DataValidationSpec`** —
  plain-data authoring descriptions with no interner, on MJXOFF-105's precedent. Unlike
  `ConditionalRuleSpecKind`, **all six** filter kinds are describable, because every one of them is
  completely stated by what the caller passes.
- **`Workbook::auto_filter`, `data_validations`, `set_auto_filter`, `remove_auto_filter`,
  `add_data_validation`, `remove_data_validation`** — the package tier. Each authoring call rewrites
  the worksheet part and **nothing else**, which is asserted against the original file's bytes.
- **`tests/fixtures/validation_and_filters.xlsx`** — **all six filter kinds, one per column**, plus a
  seventh column holding only the choice's `extLst`; `@colId`s that do **not** ascend
  (`1, 0, 4, 2, 5, 3, 6`); a two-condition sort state; four validations including two `list` rules
  whose sources are a range reference and a quoted literal respectively; a hidden row beside a
  visible one; a deliberately stale `@count`; a `@sqref` with a double space in it; and an `x14`
  cross-sheet `dataValidations` in the worksheet `extLst`. One filter kind repeated four times would
  have tested one code path.
- **Five generated child-order exports** — `AUTO_FILTER`, `FILTER_COLUMN`, `FILTERS`, `SORT_STATE`
  and `DATA_VALIDATION` in `mjx_ooxml_types::child_order`, from `xtask`'s curated list. Every
  placement in this cluster goes through them. `FILTER_COLUMN` is the first `ContentModel::Choice`
  entry any model here consumes, and every one of its members ranks 0 — which is the schema's answer
  rather than a shortcoming of the table.
- **A guide page** — *Filters and data validation*, with four compiled doctests, one of which
  asserts that a filter matching no row hides no row.

### Changed

- **`mjx_sml::features`' subject modules are public**, as `mjx_sml::formula`'s and
  `mjx_sml::styles`' already were, so a reader who reaches one of these types through its re-export
  can reach the design record behind it. Purely additive.

### Fidelity

- **A filter never hides a row.** Reading, writing or removing an `autoFilter` leaves every row's
  `@hidden` exactly as the file wrote it. A hidden row is MJXOFF-117's row property, and this
  library did not put it there.
- **A sort state never reorders one.** `CT_SortState` records a sort that happened.
- **A `list` validation's source is never resolved.** `formula1` may hold `$G$2:$G$4`,
  `Lookups!$A$1:$A$9` or `"Low,Medium,High"`; all three go in and come out as the same bytes, and
  neither spelling is converted into the other.
- **Excel's caches are reported, never recomputed** — `top10@filterVal`, `dynamicFilter@val`/
  `@maxVal`, and `dataValidations@count`.
- **`@calendarType` and `@blank` on `CT_Filters` are modelled.** The first says which calendar the
  date groups are in; the second is *(Blanks)*, which cannot be expressed as a `filter` child.
- **The `x14` cross-sheet validations are preserved and not modelled**, prefix, `uri` and bytes
  intact, through an unrelated edit.

## [0.0.114] - 2026-09-05

Conditional formatting: the rule kinds, the priority order that runs *across* blocks, and the `dxf`
layer that is reported beside a cell's format rather than folded into it (MJXOFF-120, Phase D
position 13).

### Added

- **`mjx_sml::features`** — a directory of subject modules from the first commit:
  `conditional_rules` (`CT_ConditionalFormatting`, `CT_CfRule`, a rule's `formula`),
  `conditional_scales` (`CT_Cfvo`, `CT_ColorScale`, `CT_DataBar`, `CT_IconSet`),
  `conditional_chain` (the cross-block order and the two-layer answer) and `conditional_specs`
  (the plain-data authoring vocabulary). Nothing over 500 lines.
- **`mjx_sml::ConditionalFormatting`** — `CT_ConditionalFormatting` (`sml.xsd:2709`), at rank **16**
  of `CT_Worksheet`. It and `cols` are the **only two** of that type's thirty-nine children declared
  `maxOccurs="unbounded"`, so `WorksheetPart` grows a *list* surface for it —
  `conditional_formatting_blocks`, `conditional_formatting_block_mut`,
  `push_conditional_formatting`, `remove_conditional_formatting` — never an `Option`. Merging the
  blocks would change the file and would destroy the thing that makes the feature hard.
- **`mjx_sml::ConditionalFormattingRule`** — `CT_CfRule` (`2717`), all thirteen attributes and all
  four child kinds, with every accessor named from ECMA-376 Part 1 §18.3.1.10's own prose
  (`stops_lower_priority_rules`, `top_or_bottom_count`, `ranks_from_bottom`, `includes_the_average`).
- **`mjx_sml::ColorScale`, `DataBar`, `IconSet`, `ConditionalValueObject`** — `CT_ColorScale`
  (`2769`), `CT_DataBar` (`2775`), `CT_IconSet` (`2784`), `CT_Cfvo` (`2793`). `@iconSet`'s schema
  default `3TrafficLights1` is the **generated** `IconSetType::ThreeTrafficLights`; nothing here
  writes a table of icon-set names.
- **`mjx_sml::ConditionalFormattingFormula`** — `cfRule/formula`, on MJXOFF-115's terms exactly:
  text in, the same text out, never parsed and never rewritten. Its `FromXml`/`ToXml` pair is
  hand-written for the reason `DefinedName`'s is — minimal re-escaping is lossy for preservation.
- **`WorksheetPart::conditional_rules_for`** and **`mjx_sml::ConditionalRuleChain`** — every rule
  that applies to a cell, merged across every block whose `@sqref` covers it and sorted by
  `@priority` across all of them at once. Stable, so equal priorities keep document order.
- **`mjx_sml::ConditionalCellFormat`** and **`CellFormatResolver::conditional_cell_format`** — a
  cell's base format and its conditional candidates, **side by side, with no call that merges
  them**. MJXOFF-108 documented this seam; it is now filled *beside* the resolver rather than inside
  it, and `CellFormatResolver::differential_format` is the one addition to that type.
- **`StylesheetPart::append_differential_format`** and **`Workbook::append_differential_format`** —
  appends a `dxf` and answers the index it appended at. Appending is the table's only mutation:
  a `@dxfId` is a position, so inserting or reordering would silently repoint every rule above it.
- **`Workbook::conditional_rules_for`, `conditional_cell_format`, `add_conditional_formatting`** and
  **`SheetFormatting::conditional_cell_format`** — the package tier, which adds exactly one thing the
  markup tier cannot reach: conditional formatting spans *two* parts, and authoring a highlighted
  rule writes both.
- **`mjx_sml::ConditionalRuleSpec`** and its four companions (`ColorScaleSpec`, `DataBarSpec`,
  `IconSetSpec`, `ConditionalValueObjectSpec`, `DifferentialFormatSpec`) — plain-data descriptions
  with no interner, on MJXOFF-105's precedent. They cover the five rule kinds whose markup is
  *completely* determined by their arguments; the other thirteen `ST_CfType` members are authored
  through the model, because a spec that wrote only `type="top10"` would author markup known to be
  incomplete.
- **`tests/fixtures/conditional_formatting.xlsx`** — **four blocks whose priorities interleave**
  (block 0 holds 1 and 4, block 1 holds 2, block 2 holds 3, block 3 holds 2 again and 7), a
  multi-range `@sqref`, a duplicate priority, a gap, a `stopIfTrue`, all four rule kinds, two `dxf`
  entries, and an `x14` `extLst` in two places. A fixture with one block per priority range tests
  nothing: a per-block sort and a cross-block sort agree on it.
- **Four generated child-order exports** — `WORKSHEET_CONDITIONAL_FORMATTING`,
  `CONDITIONAL_FORMAT_RULE`, `CONDITIONAL_FORMAT_COLOR_SCALE`, `CONDITIONAL_FORMAT_DATA_BAR`, added
  to `xtask`'s curated list and regenerated. `CT_IconSet` and `CT_Cfvo` are deliberately absent:
  each declares a single repeating child or none, so both are appends with no order to hold.
- **`SmlError::ConditionalFormattingBlockHasNoRange`** and
  **`ConditionalFormattingRuleHasNoPriority`** — the two things a chain cannot be answered *around*.
  A block whose `@sqref` is missing might be the one covering the cell, and a rule with no
  `@priority` has no place in the order; a silently shortened chain would report the wrong rule as
  winning.
- **A guide page**, `crates/mjx-xlsx/docs/guide/conditional_formatting.md`, beside the formulas page
  that draws the same boundary.

### The position, stated rather than implied

- **Reporting which rules apply is in scope. Deciding whether a rule is *true* is not, ever.** That
  needs a calculation engine, which MJXOFF-115 settles as absent by design. `stopIfTrue` is
  therefore reported as a *position* in the chain and never applied as a truncation — §18.3.1.10
  makes the stop conditional on the rule firing.
- **Priorities are as read; nothing renumbers.** Excel's own files have gaps and duplicates, and
  renumbering changes which rule wins. The fixture has both, and the byte-identity gate is what
  keeps it that way.
- **A cell's conditional layer is reported alongside its base format, never folded in.** Every member
  of a `dxf` is optional and absent means *inherited*, so even a fold performed by a consumer that
  *could* evaluate would need both halves; producing one merged answer here would assert a rule
  fired.
- **The `x14` extensions round-trip and are not modelled.** Data bars with negative fills and
  icon-set overrides live in that namespace; they come back through an unrelated edit byte for byte,
  prefix, `uri` and GUIDs included.

## [0.0.113] - 2026-09-05

The sheet grid — merging, row and column geometry, outline levels, page breaks, sheet protection and
scenarios (MJXOFF-117, Phase D position 12).

> **Recorded late.** MJXOFF-117's own release commit (`c2b965f`) bumped the workspace to `0.0.113`
> and the wasm package with it, but added no entry here; the omission was found by MJXOFF-120 while
> writing the entry above. What follows is reconstructed from that child's own commits and code, so
> that the ledger has no hole in it.

### Added

- **Six more of `CT_Worksheet`'s slots modelled** — `sheetProtection` (7), `protectedRanges` (8),
  `scenarios` (9), `mergeCells` (14), `rowBreaks` (23) and `colBreaks` (24) — in a directory of
  subject modules under `mjx_sml::worksheet`: `merges`, `breaks`, `protection`, `scenarios`, `rows`
  and `anomalies`.
- **`WorksheetPart::merge_cells` / `unmerge_cells` / `merge_anchor` / `is_covered_by_merge` /
  `cell_span`**, and the same surface at the package tier, plus
  `Workbook::effective_merged_cell_format`.
- **Run-length column splitting** — setting one column's width inside a `col` run breaks it into up
  to three, and only the middle piece takes the new value.
- **`RowHeight` / `ColumnWidth`** — the `customHeight`/`customWidth` flag travels with its value in
  the type, so no call can write a height without saying where the number came from.
- **`WorksheetPart::grid_anomalies`** — overlapping merges, merges over populated cells, and
  conflicting `col` runs are described rather than repaired.
- **`tests/fixtures/sheet_grid.xlsx`**, and `SmlError::MergeOverlapsExistingMerge` /
  `DegenerateMerge`.

### Fixed

- **`Slot::rank` ranks a held element through the generated table.** MJXOFF-102 modelled ranks 0–6,
  a *prefix*, which was the only reason an unmodelled child could safely be treated as unrankable.
  Promoting ranks 7, 8, 9, 14, 23 and 24 broke the prefix, and a `mergeCells` (14) inserted beside a
  held `autoFilter` (10) would have landed in schema-invalid order. MJXOFF-120 depends on this fix:
  `conditionalFormatting` is rank 16, between a held `phoneticPr` (15) and a held `dataValidations`
  (17).

## [0.0.112] - 2026-09-05

Formulas, carried as text — and the written-down guarantee that nothing here ever acts on one
(MJXOFF-115, Phase D position 11).

### Added

- **`mjx_sml::formula`** — a directory of subject modules: `cell` (`CT_CellFormula`), `cached`
  (the `<v>` beside an `<f>`), `shared` (the `@si` grouping) and `calc_chain` (`CT_CalcChain` /
  `CT_CalcCell`). Nothing over 400 lines.
- **`mjx_sml::CellFormula`** — `CT_CellFormula` (`sml.xsd:2751`), all **twelve** attributes, as a
  *borrowed view over the `<f>` element's own bytes*. It adds **zero bytes per cell**: MJXOFF-95's
  cell store already kept the formula's byte range, and this gives those bytes a type rather than a
  second home. A decoded struct per cell — a `String` for the text, `Option<CellRange>` for `@ref`,
  seven `bool`s — was rejected on both counts it would fail: memory (a million formula cells) and
  fidelity (nothing decoded reproduces `&quot;`, a single-quoted value, or `si` written before `t`).
- **`mjx_sml::FormulaKind`** — `ST_CellFormulaType`'s four values, **re-exported from the generated
  `mjx_ooxml_types::spreadsheetml::CellFormulaType` rather than declared a second time.**
  `has_written_kind` is the distinction a round trip turns on: `t` is declared
  `use="optional" default="normal"`, so an absent `t` and `t="normal"` mean the same thing and must
  not be written the same way.
- **`mjx_sml::CachedValue`** and **`Cell::cached_value`** — the result a producer last computed, read
  through the `c@t` beside it. `None` for a `<v>` with no `<f>`: a value somebody typed and a result
  Excel computed are different facts about the file.
- **`mjx_sml::SharedFormulaGroups` / `SharedFormulaGroup`** and **`SheetData::shared_formula_groups`**
  — the `@si` grouping, indexed on demand and held nowhere. It reports the host, the group's range
  and its cell count; it deliberately does **not** answer "what is this member's formula", because
  that answer is the host's text shifted by the offset between the two cells, and shifting references
  is translation.
- **`mjx_sml::CalculationChain` / `CalculationChainCell`** and **`mjx_xlsx::Workbook::calculation_chain`**
  — `CT_CalcChain` (`sml.xsd:257`) and `CT_CalcCell` (`263`), with §18.6.1's two carry-forward rules
  (`@i` and `@s` take the previous entry's value when absent) resolved by `CalculationChain::resolved`
  and by nothing on the write path.
- **`tests/fixtures/formulas.xlsx`** — a normal formula writing no `t`, one writing `t='normal'`
  single-quoted, a shared group of five (host plus four text-less members, written four different
  ways), an array formula over a range whose other cells carry no `<f>` at all, a data-table formula
  with all six of its attributes, formula text carrying `&lt;`/`&amp;`/`&quot;`, and an
  `xl/calcChain.xml` of eleven entries.
- **A fifth case in `crates/mjx-sml/tests/cell_store_allocation.rs`** — 300,000 cells, *every one* a
  formula cell, 295,000 of them text-less shared-group members. Measured: **76.8 B/cell**, against
  36.8 for the same sheet of values and the 913 B/cell `docs/BENCHMARKS.md` (MJXOFF-147) records for
  a `RawElement` tree of it. The 40-byte difference is MJXOFF-95's `CellExtras`, which a formula cell
  already paid for; **a group member costs what any formula cell costs and nothing for its
  membership**.
- **A guide page**, `crates/mjx-xlsx/docs/guide/formulas_and_cached_values.md`, and a new section on
  the fidelity page — both stating the stale-cache limitation in prose and naming it deliberate.

### The position, stated rather than implied

- **Nothing here calculates, and a stale cached value is correct behaviour.** An edit that changes a
  cell a formula depends on leaves the cached `<v>` exactly as it was. Recalculating needs an engine
  this workspace does not have and will not grow; blanking the `<v>` destroys data in a file the
  caller opened to change a label; marking the workbook dirty writes into a part nobody asked to
  edit. `crates/mjx-sml/tests/formulas.rs` and `crates/mjx-xlsx/tests/formulas.rs` fail if any of the
  three ever starts happening.
- **A shared group's text distribution is preserved exactly.** The host carries the text and the
  members carry none. Expanding a group to per-cell text on write is a corruption and not an
  optimisation: it changes bytes nobody asked to change, and because a shared formula's references
  are written relative to the host, a copied expression states a different formula from the one that
  cell has.
- **`xl/calcChain.xml` is left exactly as found** — neither maintained nor dropped. Maintaining it
  means computing a dependency order, which means parsing expressions. Dropping it is an edit the
  caller did not ask for, made on every save, and it loses a record they may be reading the file to
  inspect. §18.6 says a consumer "is free to perform calculations in a different order at run time",
  which is what makes leaving a stale chain safe.

### Fixed

- `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md` said "Styles and formulas are not
  modelled at all yet", which stopped being true at MJXOFF-105.

## [0.0.111] - 2026-09-05

The `mjx-sml` package writer that replaces `mjx_chart::EmbeddedWorkbook`, `Workbook::blank`, and the
Excel authoring surface (MJXOFF-112, Phase D position 10).

### Added

- **`mjx_sml::write`** — the SpreadsheetML **package writer**, a directory of subject modules:
  `constants` (part names, content types, relationship types), `workbook` (`xl/workbook.xml`),
  `sheet` (one authored worksheet), `stylesheet` (`xl/styles.xml` and its four appends),
  `style_specs` (the plain-data descriptions those appends take) and `package`
  ([`WorkbookPackage`], which assembles the whole `.xlsx`). It authors `[Content_Types].xml`,
  `_rels/.rels`, `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, `xl/worksheets/sheetN.xml`,
  `xl/sharedStrings.xml`, `xl/styles.xml` and — on request — `docProps/core.xml` and
  `docProps/app.xml`.
- **`mjx_sml::write::AuthoredCellValue`** — a cell value stated *before* there is a shared-string
  table to point into, which is the shape a caller actually holds. `WorkbookPackage::push_row`
  interns text in first-use order and writes nothing at all for a `Blank` or a non-finite number,
  which is what a grid with holes in it means; `WorkbookPackage::set_cell_value` takes the wire-shaped
  `CellValue` and **refuses** a non-finite number instead, because naming one cell is stating a value.
- **`mjx_sml::write::{PatternFillSpec, BorderSpec, BorderEdgeSpec, CellFormatSpec}`** — plain-data
  descriptions of the four `styles.xml` resources, with no interner and no lifetime. A `Border` is
  **nine** edges (`start`, `end`, `left`, `right`, `top`, `bottom`, `diagonal`, `vertical`,
  `horizontal`), not four. Fonts reuse `FontProperties`, which is already that description.
- **`mjx_xlsx::Workbook::blank`** and **`blank_with_properties`** — a workbook authored from nothing,
  through the `mjx-sml` writer. **There is no schema-valid empty workbook**: `CT_Workbook` has
  nineteen slots and only `sheets` is mandatory, and `CT_Sheets` requires at least one `sheet`, so
  `blank()` necessarily authors a worksheet part, its content type, its relationship and a
  `sheetData` as well. Deterministic — two calls produce byte-identical containers.
- **`mjx_xlsx::Workbook::add_sheet`, `set_cell_style`, `intern_shared_string`, `append_font`,
  `append_pattern_fill`, `append_border`, `append_cell_format`** — the authoring surface. Concrete
  types, no closures in a signature, nothing returning a borrowed view into the workbook.
- **`mjx_sml::SmlError::AuthoredPartSeedRejected`** and **`SheetIndexOutOfRange`**.
- **`mjx-chart` now depends on `mjx-sml`** — the legal downward edge (2.2 → 2.1) MJXOFF-99 needs, in
  place and green under `xtask/tests/layering.rs`. **Nothing is removed here**: `EmbeddedWorkbook`
  and `to_package_bytes` are untouched, and MJXOFF-99 owns their removal.

### Changed

- **`mjx_xlsx::parts` re-exports eight constants from `mjx_sml::write::constants`** rather than
  declaring them a second time — `REL_OFFICE_DOCUMENT`, `REL_WORKSHEET`, `REL_SHARED_STRINGS`,
  `REL_STYLES`, `CONTENT_TYPE_WORKBOOK`, `CONTENT_TYPE_WORKSHEET`, `CONTENT_TYPE_SHARED_STRINGS`,
  `CONTENT_TYPE_STYLES`. Same paths, same values, one definition. The other twenty-one stay this
  crate's: reading a part graph needs them and authoring one does not.
- **`SharedStringTable::authored` writes an empty table self-closing** (`<sst … count="0"
  uniqueCount="0"/>`), which is what Excel and `mjx-chart`'s writer both emit. The non-empty form is
  unchanged, and the parity gate compares the two byte for byte.

### Verified

- **The parity gate** (`crates/mjx-chart/tests/workbook_parity.rs`): the same grid through
  `EmbeddedWorkbook` and through `WorkbookPackage`, compared part by part. `[Content_Types].xml`,
  `_rels/.rels`, `xl/_rels/workbook.xml.rels`, `xl/workbook.xml`, `xl/worksheets/sheet1.xml` and
  `xl/sharedStrings.xml` are **byte-identical**. `xl/styles.xml` states the same six tables with the
  same counts and the same record behind every index-0 reference, and differs only in the order of a
  font's children — `CT_Font` is an `xsd:choice`, so the schema imposes none.
- **The writer needs nothing above `mjx-sml`** (`crates/mjx-sml/tests/package_writer.rs`): a whole
  package is authored, read back and asserted on from inside a crate whose manifest names no format
  crate, no facade and no `mjx-chart` — checked by reading that manifest rather than claimed.
- Every part `Workbook::blank` authors validates against `sml.xsd` and the OPC schemas, and is in
  `xsd:sequence` order (`crates/mjx-xlsx/tests/schema_gate.rs`).
- **The office-open canary now covers Excel.** `crates/mjx-xlsx/tests/office_open.rs` drives
  LibreOffice over the blank workbook, an authored-and-filled-in one, `sample.xlsx` and a
  round-tripped `sample.xlsx`; `.github/workflows/ci.yml` installs `libreoffice-calc` and names
  `-p mjx-xlsx` on the canary step. Schema validity is necessary and not sufficient — a package can
  satisfy every XSD and still be refused for a broken relationship graph — and a package filter that
  named only two of the three crates would have dropped these four cases with the job still green,
  which is precisely the failure MJXOFF-98 found for Word.

## [0.0.110] - 2026-09-05

`xl/styles.xml` part 2: the `xf` indirection, number formats, named styles, and the resolver that
answers what formatting a cell actually carries (MJXOFF-108, Phase D position 9).

### Added

- **`mjx_sml::styles::formats`** — `CT_Xf` (`sml.xsd:3598`) as `CellFormat`, and `CT_CellXfs` /
  `CT_CellStyleXfs` as one `CellFormatTable` with a `CellFormatTableKind` to say which slot an
  authored one stands in. The two complex types are character for character identical; declaring
  them twice would have been two copies of one index-identity discipline to keep in step.
  **`CT_Xf` carries thirteen attributes, not the fourteen its ticket claimed.**
- **`mjx_sml::ApplyFlag`** — `applyNumberFormat`, `applyFont`, `applyFill`, `applyBorder`,
  `applyAlignment` and `applyProtection` in **all three** of the states the schema gives them. Every
  one is declared `use="optional"` with **no `default=`**, so *absent*, `"1"` and `"0"` are three
  distinct values, and collapsing absent into false is a defect no schema validator can see — both
  spellings are valid documents. `ApplyFlag::participates` is where the meaning lives: absent
  participates, because §18.8.9's worked example contrasts a record that "does not express any
  'apply' attributes" (the `Normal` style, which is applied) with records that suppress by writing
  `applyX="0"`.
- **`mjx_sml::styles::number_formats`** — `CT_NumFmts` as `NumberFormatTable`, keyed by
  `@numFmtId` rather than by position (the only table in the part that is not an array), plus
  ECMA-376 Part 1 §18.8.30's **implied** format codes: `builtin_format_code` for the twenty-eight
  ids listed under *All Languages*, `builtin_format_code_in` and `NumberFormatLanguage` for the
  ids whose code depends on the UI language, and `is_locale_dependent`. The all-languages set is
  **not** `0..=49` — 5–8, 23–26 and 41–44 are not built in and §18.8.30 says so — and the
  locale-dependent ids run to **81**, not 49. Ids 37 and 38 carry a space before their semicolon
  while 39 and 40 do not; that asymmetry is the published table's and is reproduced exactly.
- **`mjx_sml::styles::named_styles`** — `CT_CellStyles` / `CT_CellStyle` as `NamedCellStyles` /
  `NamedCellStyle`, with **all six** of `CT_CellStyle`'s attributes (its ticket named three), and
  Annex G.2's fifty-one built-in names through `builtin_cell_style_name`. `builtinId` 1 and 2 are
  `RowLevel_` / `ColLevel_` plus the style's own `@iLevel`, so `BuiltInCellStyleName` keeps them
  apart from the forty-nine fixed names rather than inventing a level to concatenate.
- **`mjx_sml::styles::effective`** — the resolver. `CellFormatResolver` decodes every `xf` in both
  tables **once**, and then `EffectiveCellFormat` is `Copy` and per-cell resolution parses and
  allocates nothing. `cell_style_index` walks cell → row (gated on `customFormat`, per §18.3.1.73)
  → column → the default record and reports which layer answered as a `StyleIndexSource`;
  `ResolvedAspect` reports, per aspect, the `applyX` it saw, the `FormatLayer` that supplied the
  value and the resource index it points at. `ColumnStyles` decodes a sheet's `col@style` runs once.
  Reading takes `&self` throughout and cannot mark a part dirty.
- **`mjx_xlsx::Workbook::styles_markup`, `sheet_formatting`, `effective_cell_format`**, and
  `SheetFormatting` / `SheetFormatResolver` — the package tier finds the two parts and hands the
  `mjx-sml` resolver the bytes. **The resolution order is not repeated there**; a second walk would
  be a second answer to one question, free to drift.
- **`mjx_sml::SmlError::CellFormatIndexOutOfRange`** — a style index naming no `xf` is refused
  rather than answered with record 0. A dangling `@xfId` on a record that *does* exist is a
  different thing: that is a layer which is absent, reported as `FormatLayer::Neither`.
- **`STYLESHEET_CELL_FORMAT`** — the generated child-order table for `CT_Xf`'s three children.
- **`tests/fixtures/effective_cell_format.xlsx`** — a workbook whose `cellXfs` and `cellStyleXfs`
  entries **deliberately disagree, property by property**: different number format, font, fill,
  border, alignment and protection on the two layers of one cell, so reading the wrong layer gives a
  visibly wrong answer. Four `cellXfs` records name the same underlying record and the same four
  indices and differ only in their `applyX` — false, absent, true, and one record mixing false with
  true — because that is the only arrangement that can tell the three states apart. A fifth sits on a
  style record that suppresses `applyFont` itself. The worksheet sets a cell, its row and its column
  to three different fonts, and writes one row with `s` and **no** `customFormat` so the gate on that
  attribute is load-bearing.
- **`docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md`** — the comparison table for MJXOFF-122 (F1), handed
  over **unmarked**: twenty-eight rows of answers this workspace gives, with the *Excel says* and
  *Verdict* columns deliberately empty. Two rows are inferences from a worked example rather than
  from a normative sentence, and are labelled as such.

### Changed

- **`mjx_sml::StylesheetPart` now models nine of `CT_Stylesheet`'s eleven slots**, up from five.
  `numFmts` (rank 0), `cellStyleXfs` (4), `cellXfs` (5) and `cellStyles` (6) join the four resource
  tables and `colors`; only `tableStyles` (MJXOFF-127) and `extLst` are still held raw. The
  interleaving MJXOFF-105 found is narrower and has not gone away — `tableStyles` at rank 8 still
  sits between `dxfs` at 7 and `colors` at 9 — so placement still ranks unmodelled elements through
  the generated table by their own name.

## [0.0.109] - 2026-09-05

`xl/styles.xml` part 1: the resource tables a style index resolves into — fonts, fills, borders,
`dxf`s and the indexed-colour legacy (MJXOFF-105, Phase D position 8).

### Added

- **`mjx_sml::styles`** — `CT_Stylesheet` (`sml.xsd:3387`) as `StylesheetPart`, in eight subject
  modules. Five of the eleven slots are modelled — `FontTable`/`Font`, `FillTable`/`Fill`/
  `PatternFill`/`GradientFill`/`GradientStop`, `BorderTable`/`Border`/`BorderEdge`,
  `DifferentialFormats`/`DifferentialFormat` and `ColorTable`/`IndexedColors`/`MruColors`/`RgbColor`
  — and the other **six are held as the markup the file wrote, in their schema position**:
  `numFmts`, `cellStyleXfs`, `cellXfs` and `cellStyles` are MJXOFF-108's, `tableStyles` is
  MJXOFF-127's, and `extLst` is nobody's on purpose.
- **`mjx_sml::styles::palette`** — the indexed-colour legacy, sourced from ECMA-376 Part 1 §18.8.27:
  `IndexedColorPalette::DEFAULT` is the spec's own sixty-four rows (`0`–`7` are the spec's stated
  duplicates of `8`–`15`), `IndexedColor::SystemForeground`/`SystemBackground` are its `indexed="64"`
  and `"65"`, which have no ARGB, and a workbook's `indexedColors` block replaces the whole table.
  `theme_color_slot` is §20.1.6.2's index table — SpreadsheetML addresses a theme colour by
  *position*, not by a `schemeClr` token. `apply_tint` is §18.8.19's luminance algorithm, and
  `resolve_color` puts the three together, taking `mjx_dml::SchemeColors` so that a workbook's theme
  colour resolves to exactly what a DrawingML `a:schemeClr` on the same slot resolves to.
- **`mjx_sml::styles::cell_format`** — `CellAlignment`, `CellProtection` and `NumberFormat`
  (`CT_CellAlignment`, `CT_CellProtection`, `CT_NumFmt`), in a module belonging to neither subject
  because `CT_Dxf` needs all three now and `CT_Xf` needs them at MJXOFF-108.
- **`mjx_sml::ColorElement`** — `CT_Color` as an *element*, one type for all five local names it
  stands under (`color`, `fgColor`, `bgColor`, `tabColor`). Preservation; `Color` stays the decoded
  snapshot.
- **`mjx_dml::SchemeColors::rgb`** is public. The type exists to be the interner-free bridge between
  a colour in one part and a theme in another, and a bridge only `mjx-dml` can cross is a bridge to
  nowhere.
- **Four generated child-order tables** for the styles cluster: `STYLESHEET_BORDER`,
  `STYLESHEET_PATTERN_FILL`, `STYLESHEET_DIFFERENTIAL_FORMAT` and `STYLESHEET_COLOR_TABLE`.
- **`tests/fixtures/style_resources.xlsx`** — a styles part authored against this subject's own
  traps: **two byte-identical `<font>` entries**, so deduplicating on write breaks the
  index-identity case rather than passing it; a `<border>` exercising all **nine** edges, `start` and
  `end` included; four `dxf`s (fill only, font only, all six members, and `<dxf/>`); an
  `indexedColors` block differing from the default palette at exactly one row; and four of the six
  raw slots, a doubled space in two start tags, a single-quoted attribute, an element written
  `<top …></top>`, a comment between two slots and an `ext` in a foreign namespace. Schema-valid, so
  it needs no tolerance entry.

### Changed

- **`mjx_sml::TabColor` is gone; the slot is `ColorElement`.** MJXOFF-102 declared a `tabColor`
  attribute bag of its own; MJXOFF-105 found four more slots of the same complex type in
  `styles.xml`, and five bag types for one `CT_Color` is exactly the duplication this crate already
  has a scheduled child to undo once. `SheetProperties::tab_colour` and
  `SheetProperties::tab_color_element` are unchanged in meaning.
- **The styles frame ranks its unmodelled slots too**, which neither `WorkbookPart` nor
  `WorksheetPart` has to. Those two model a *prefix* of their sequence (ranks 0–17 of nineteen, 0–6
  of thirty-nine), so a modelled child always belongs before every raw one and unranked-means-stepped
  -over is harmless. `CT_Stylesheet`'s modelled ranks are **1, 2, 3, 7, 9** and its raw ones **0, 4,
  5, 6, 8, 10**: they interleave, so `StylesheetPart`'s setters take an `&Interner` and rank a raw
  element through the same generated table by its own name. Treating one as unranked put an inserted
  `colors` before the `numFmts` already in the file.

### Notes on the specification

- **The indexed palette has sixty-four rows, not fifty-six**, plus two indices that are not colours.
  §18.8.27's own note explains it: *"0-7 are redundant of 8-15 to preserve backwards
  compatibility"*, so the distinct BIFF palette is `8`..=`63` and a lookup table starting at `8`
  would answer nothing for the eight indices a file most often uses for black and white.
- **§18.8.19's third worked tint example rounds.** It prints
  `100 * .25 + (255 - 255 * .25) = 25 + (255 - 63) = 217`, but `255 * .25` is `63.75`; the exact
  result is `216.25`, and `217` comes of truncating an intermediate in integer HLS arithmetic. This
  crate computes on the unit interval — both branches are linear in luminance, so `HLSMAX` cancels —
  and rounds once, at the end.
- **`CT_Border` declares nine edges**, `start` and `end` among them, while §18.8.4's prose enumerates
  five and §18.8 carries no entry for either of the two. They are documented in WordprocessingML
  instead (§17.4.33 *Leading Edge Border*, §17.4.12 *Trailing Edge Border*), which is where this
  crate takes `Border::leading_edge`/`trailing_edge` from — the spelling `mjx_docx::Indentation`
  already uses for the same pair.

## [0.0.108] - 2026-09-05

The worksheet spine: `CT_Worksheet`'s thirty-nine slots, the widest content model in the schema
(MJXOFF-102, Phase D position 7).

### Added

- **`mjx_sml::worksheet`** — `CT_Worksheet` (`sml.xsd:2170`) as `WorksheetPart`, in four subject
  modules. Seven of the thirty-nine slots are modelled — `SheetProperties` (`sheetPr`, with
  `TabColor`, `OutlineProperties` and `PageSetupProperties`), `SheetDimension`, `SheetViews` /
  `SheetView` / `SheetPane` / `Selection` / `PivotSelection`, `SheetFormatProperties`,
  `ColumnBlock` / `ColumnRun`, `mjx_sml::SheetData` (MJXOFF-95's store) and
  `SheetCalculationProperties` — and the other **thirty-two are held as the markup the file wrote,
  in their schema position**. A worksheet whose `pageSetup` survives is proof the frame works, not
  proof `pageSetup` was modelled.
- **`mjx_xlsx::Workbook`** grows the worksheet surface: `worksheet_markup`, `worksheet_markup_of`,
  `write_worksheet_markup`, `set_cell_value`, `cell_text`, `shared_strings` and `package`. A third
  guide page, *Reading and editing cells*, covers them.
- **`mjx_xml::fidelity::serialize_start_tag` / `serialize_end_tag`** — an element's tags on their
  own. `serialize_element` already existed for a model that holds no tree but does hold whole
  elements; the worksheet frame holds no tree and its `sheetData` child is a packed byte store rather
  than a `RawElement`, so it writes `<worksheet …>`, then each slot's bytes, then `</worksheet>`.
- **`tests/fixtures/worksheet_spine.xlsx`** — a worksheet carrying one child from every later Phase D
  child's territory, **two `<cols>` blocks** rather than one (the slot is `maxOccurs="unbounded"`, so
  merging them changes the file), `mergeCells` present with `autoFilter` deliberately absent, a
  frozen pane with two selections, a comment between two slots, and — on one modelled slot and one
  unmodelled one — a doubled space inside a start tag that nothing but the verbatim source range
  reproduces. Schema-valid, so it needs no tolerance entry.
- **`mjx_sml::Color::read_attributes`** — `Color::read` for a caller holding the attribute list
  rather than the element, so `sheetPr/tabColor` can be *preserved* as an attribute bag and *decoded*
  on demand rather than stored as a lossy snapshot.

### Changed

- **`WorksheetPart` consumes the document it is read from**, rather than borrowing a tree the package
  caches. `docs/BENCHMARKS.md` records 913 bytes of peak resident set per cell for a 300,000-cell
  worksheet held as a `RawElement` tree; the packed store holds the same sheet in 36.8, and a frame
  that kept the tree alive beside it would hand the 25× straight back. Copy-on-write is therefore
  restated at a fourth granularity — per **slot** — with the same *exactly one door* rule the cell
  store uses: a modelled slot's verbatim bytes are given up by the `_mut` accessor and by the setter,
  and by nothing else. `crates/mjx-sml/tests/cell_store_allocation.rs` gains two cases that measure
  both claims through the real part.
- `mjx_sml`'s attribute-bag macro moved from `workbook/leaf.rs` to `leaf.rs`: nine more of the types
  it declares are the worksheet's, and the macro never belonged to the workbook.
- `crates/mjx-xlsx/tests/schema_gate.rs`'s "no part under `xl/` is skipped" rule now sweeps **every**
  committed `.xlsx` rather than `sample.xlsx` alone — `worksheet_spine.xlsx` is the first fixture with
  a part under `xl/` that `sample.xlsx` does not have, and pinning one fixture would have let it join
  the sweep as a skip.

### Fixed

- `crates/mjx-sml/tests/cell_store_fidelity.rs`'s corpus sweep listed every part under
  `/xl/worksheets/` as a worksheet, including the `_rels` streams. No committed fixture had a
  worksheet-level `.rels` until this child added one, so the defect was latent rather than wrong.

## [0.0.107] - 2026-09-05

`xl/workbook.xml`: the part that names every sheet (MJXOFF-100, Phase D position 6).

### Added

- **`mjx_sml::workbook`** — `CT_Workbook` (`sml.xsd:4097`) and the twenty-nine complex types its
  cluster runs to at `sml.xsd:4439`, in eight subject modules. `WorkbookPart` is the nineteen-slot
  sequence; `SheetList`/`SheetEntry`, `WorkbookProperties`, `FileVersion`, `FileSharing`,
  `WorkbookProtection`, `FileRecoveryProperties`, `EmbeddedObjectSize`, `BookViews`/`WorkbookView`,
  `CustomWorkbookViews`/`CustomWorkbookView`, `CalculationProperties`, `DefinedNames`/`DefinedName`,
  `ExternalReferences`/`ExternalReference`, `PivotCaches`/`PivotCache`,
  `FunctionGroups`/`FunctionGroup`, `SmartTagProperties`/`SmartTagTypes`/`SmartTagType`,
  `WebPublishing`/`WebPublishObjects`/`WebPublishObject` are the rest.
- **`mjx_sml::BuiltInName`** — the eight names ECMA-376 Part 1 §18.2.6 reserves
  (`_xlnm.Print_Area`, `_xlnm.Print_Titles`, `_xlnm.Criteria`, `_xlnm._FilterDatabase`,
  `_xlnm.Extract`, `_xlnm.Consolidate_Area`, `_xlnm.Database`, `_xlnm.Sheet_Title`), matched on the
  exact token and taken from the standard's own prose rather than guessed.
- **`mjx_xlsx::Workbook`** grows the navigation surface over that model: `sheet_index_by_name`,
  `sheet_by_name`, `worksheet_by_name`, `visible_sheets`, `defined_names`, `defined_name`,
  `print_area`, `date_system`, `calculation_settings`, `window_views`, `active_sheet`,
  `rename_sheet`, and the whole-part pair `workbook_markup` / `edit_workbook_markup`. New public
  types: `DateSystem`, `CalculationSettings`, `WorkbookWindow`, `DefinedNameEntry`,
  `DefinedNameScope`; new error variant `XlsxError::NoSuchSheet`.
- **`tests/fixtures/workbook_sheet_order.xlsx`** — three sheets whose list order, `@sheetId` order
  and relationship order **all disagree**, one `hidden` and one `veryHidden` tab, a global defined
  name, a sheet-scoped one, a `_xlnm.Print_Area` and a `@localSheetId` that names no sheet. A
  workbook whose orderings agree cannot tell a correct resolver from three wrong ones. Schema-valid,
  so it needs no tolerance entry.

### Changed

- `mjx_xlsx`'s sheet-list reader no longer walks the workbook tree by hand: it reads through
  `mjx_sml::WorkbookPart` and then resolves each entry's `r:id` against the package. That is the
  whole `mjx-sml` / `mjx-xlsx` seam in one function — the markup layer never names a part, and an
  embedded workbook inside a `.pptx` needs the first half without the second.
- `mjx-derive` gains its first user in `mjx-sml`, as that crate's manifest predicted it would.

### Fixed

- **A defined name's character-data spelling now survives an edit elsewhere in the part.**
  `mjx-derive`'s `#[xml(text)]` grammar decodes on read and re-escapes **minimally** on write, so
  `&apos;Sheet&apos;!$A$1` came back as `'Sheet'!$A$1` — the same string, different bytes — and a
  rebuilt text node that differs from the original denies its element, and every ancestor of it, the
  verbatim source range subtree copy-on-write would otherwise give it. Nothing notices while a part
  is untouched, because then the model never writes at all; renaming a *sheet* was enough to make it
  visible. `CT_DefinedName` therefore has a hand-written `FromXml`/`ToXml` pair that keeps the
  original children until `set_definition` replaces them. **The same property still holds for every
  other `#[xml(text)]` leaf in the workspace** (`a:t`, `w:t`, …); fixing it in the derive is a
  foundation change no ticket owns yet, and it is recorded here so that the next child to trip over
  it does not re-derive it.

## [0.0.106] - 2026-09-05

The shared string table: what a `t="s"` cell's index actually means (MJXOFF-97, Phase D position 5).

### Added

- **`mjx_sml::strings`** — `xl/sharedStrings.xml` and everything reached through it. `CT_Sst`
  (`SharedStringTable`), `CT_Rst` (`StringItem`), `CT_RElt` (`RichTextRun`), `CT_PhoneticRun`
  (`PhoneticRun`), `CT_PhoneticPr` (`PhoneticProperties`), the `<is>` of a `t="inlineStr"` cell
  (`InlineString`), and `RichTextRunSpec` for authoring. `crates/mjx-sml/docs/SHARED_STRINGS.md` is
  the decision record: the measurements, the alternatives that lost, and the two lifetime policies in
  full.
- **`mjx_sml::font`** — `FontProperties`, `FontPropertyOwner` and `Color`. `CT_RPrElt` (a run's
  `rPr`) and `CT_Font` (a `styles.xml` font-table entry) are the same fifteen slots over the same
  eight `val`-wrapper complex types, differing only in `rFont` vs `name` and in `family`'s declared
  type, so they are one Rust type with a two-valued owner. **MJXOFF-105 (D08) reuses this module
  rather than copying it**; a copy would arrive with no executioner, which is the debt MJXOFF-99
  exists to discharge for `mjx-chart`'s duplicate SpreadsheetML writer.
- **`tests/fixtures/shared_strings_rich_text.xlsx`** — authored to disagree with the naive answer:
  `count="9"`, `uniqueCount="6"` and **seven** entries, an entry nothing references, an
  `xml:space="preserve"` entry, three rich-text runs in two `rPr` shapes, an East Asian entry with
  `rPh` and `phoneticPr`, an empty `<t/>`, a duplicate entry, two `t="inlineStr"` cells and a `t="s"`
  cell whose index points past the end of the table.
- **`crates/mjx-sml/tests/shared_strings_fidelity.rs`** (31 cases) and
  **`crates/mjx-sml/tests/shared_string_allocation.rs`** — the fidelity contract, and a second
  `harness = false` memory gate. A global allocator is process-wide, so a second measurement inside
  MJXOFF-95's binary would have started from whatever that one left live.

### Changed

- **`crates/mjx-sml/src/arena/`** — the byte arena, the checked start-tag split and the attribute-run
  scanner move out of `cells/` to sit below both packed stores. `PLAN.md` names two bulk-data cases,
  not one; two copies of `decompose` would have been two copies of the invariant that a source range
  is a *claim about somebody else's buffer* and has to be re-checked before it is believed. Gains
  `span_over` (an authored range lives in the second half of the address space) and
  `attribute_run_of` (an element whose prefix the caller does not know).

### Notes

- **48.0 bytes per entry, against 660 for a `RawElement` tree of the same table**, and zero bytes
  authored by a table nobody has edited. The bound is not the gate, and this is worth stating: against
  twelve-character strings a `Vec<String>` costs 24 bytes of header plus the text — **less** than a
  48-byte record — so a bytes-per-entry bound would have passed the design this one rejects. The
  load-bearing assertion is that the table retains the same bytes *to the byte* for entries whose
  text is ten times longer, which an entry holding a span has and an entry owning its text cannot
  have at any string length.
- **`count` and `uniqueCount` are hints, and only one of them is knowable here.** `uniqueCount` is
  the entry count, which the table is; `count` is the number of `t="s"` cells in the workbook, which
  it cannot see. Both round-trip as read. Only a change to the entry list moves `uniqueCount`, and
  only if the file wrote the attribute at all; nothing ever derives `count`, and
  `set_reference_count` is the only thing that writes it.
- **Nothing is ever renumbered.** An index is written into cells in every sheet, so removing an entry
  rewrites the meaning of every later one. Entries are append-only and an unreferenced entry stays;
  `compact` is an explicit call that returns the old-to-new map the caller must then apply to every
  sheet itself. The consequence, stated rather than discovered later: a workbook edited many times
  accumulates dead entries, which is the cheaper of the two wrong answers.
- **`xml:space="preserve"` is written only where its absence would change the value.** `sml.xsd`
  types a `t` as the simple type `ST_Xstring`, which can carry no attribute, so the attribute both
  Excel and LibreOffice write does not validate — and without it a consumer may collapse leading and
  trailing whitespace and `"  total  "` becomes `"total"`. Losing the string is worse; confining the
  divergence to strings that need it keeps an ordinary authored table schema-valid and byte-identical
  to `mjx-chart`'s writer.
- **`CT_Color` is not `mjx_dml::Color`, and MJXOFF-97's ticket was wrong to say it could be.**
  DrawingML's colour is a choice of six *elements* whose name is the kind; SpreadsheetML's is one
  element with five *attributes*, and `indexed`, `theme` (a position, not a token) and `tint` have
  nowhere to go in the other. There is still exactly one spreadsheet colour type, shared with
  everything D08 colours.
- **`CT_RPrElt` is an `xsd:choice`.** The generated `child_order` table says so — every slot at rank
  zero — so nothing here imposes an order on a run's properties. The fixture writes them in a
  non-canonical order on purpose, and it round-trips.

## [0.0.105] - 2026-09-05

The cell store: `PLAN.md`'s hybrid memory model stops being theoretical (MJXOFF-95, Phase D
position 4).

### Added

- **`mjx_sml::cells`** — `CT_SheetData`, `CT_Row` and `CT_Cell`, held as three flat arrays over one
  byte arena rather than as a tree. `SheetData` (read, edit, write), the `Row` and `Cell` views,
  `CellValue`, `PayloadShape` and `SheetDataAnomaly`. `crates/mjx-sml/docs/CELL_STORE.md` is the
  decision record: every alternative that was costed, what each would have cost, and the machine the
  numbers came from.
- **`mjx_sml::SmlError::SheetDataTooLarge`** and **`::UnrepresentableNumber`** — the fifth and sixth
  variants: a worksheet whose bytes outgrow the store's `u32` address space, and a `NaN` or infinity
  asked of `CellValue::Number`. SpreadsheetML has no numeric spelling for the latter — Rust's `inf`
  is not `xsd:double`, `xsd:double`'s `INF` does not parse back, and Excel writes an error cell — so
  the store refuses and the message names `CellValue::Error("#NUM!")` as the answer. The enum stays
  deliberately exhaustive, so MJXOFF-137's facade mapping cannot silently file either under a
  wildcard.
- **`mjx_xml::fidelity::serialize_element` / `serialize_node`** — serialize one element or node
  against an interner and an optional source buffer, without a `RawDocument`. A model that holds
  rows rather than a tree has both and no document to put them in; the alternative was a second
  serializer in a crate that must not have one.
- **`crates/mjx-allocation-counter`** — the counting global allocator MJXOFF-146 wrote inside
  `xtask/src/fuzz/`, moved so that a `mjx-sml` test binary can install it too. Nothing may depend on
  `xtask`, and a second `unsafe impl GlobalAlloc` is the last thing a workspace with
  `unsafe_code = "deny"` should have. Dependency-free and outside the shipped graph, like
  `mjx-fixtures`; `CLAUDE.md`'s "two test-only crates" and "three places allow unsafe" both become
  three, and `xtask/tests/layering.rs` grows the tier that keeps the claim checked.
- **`tests/fixtures/row_spans_and_extensions.xlsx`** — a real package carrying `row@spans` on two
  rows and none on a third, plus a `c/extLst` of foreign markup. MJXOFF-93 could only assert
  `ST_CellSpans` against authored markup, `sample.xlsx` being LibreOffice-authored and carrying none.
- **`crates/mjx-sml/tests/cell_store_allocation.rs`** — the memory gate: one target, one `main`, one
  thread, `harness = false`, and a hard byte bound measured by the allocation counter.
- **`crates/mjx-sml/tests/cell_store_fidelity.rs`** — the round-trip, edit-isolation, unknown-bucket,
  `spans` and untrusted-input cases.

### Changed

- **`xtask`'s SpreadsheetML corpus writes `row@spans`**, as Excel does. The hint is advisory and
  changes nothing about the file's meaning; the worksheet part moves from 8,955,423 to 9,020,423
  bytes and the package from 1,233 to 1,235 KiB, and `docs/BENCHMARKS.md` says so where the figures
  are. Element and cell counts are unchanged.
- **`cargo run --release -p xtask -- corpus --mem xlsx` gained a fifth checkpoint** — what holding
  the corpus worksheet costs as a packed store rather than a tree — taken with the tree still alive,
  so the reading is the honest cumulative one.

### Notes

- **36.8 bytes per cell, against the 913 MJXOFF-147 measured for a `RawElement` tree of the same
  worksheet.** That benchmark also said where the 913 comes from — not the 72-byte element struct but
  the two small heap allocations every element carries — so this store has no per-cell allocation at
  all: 36 bytes a cell, 48 a row, 40 for the rare cell that carries something unusual, and, for a
  worksheet nobody has edited, not one byte of its own, because every value it preserves is a range
  into the part's buffer, shared with the package rather than copied. A sheet whose only populated
  cell is `XFD1048576` allocates 368 bytes and holds one row record.
- **Holding is 25x cheaper; opening is unchanged.** The store is built from a `RawElement` tree, so
  the +274 MiB first materialisation MJXOFF-147 recorded is still paid on open. Building it straight
  from the part's bytes would need a streaming reader in `mjx-xml`, `quick-xml` being allowed behind
  that crate and nowhere else, and is not in this child's scope.
- **The unknown bucket in a packed store.** `CLAUDE.md` states the rule as `extra: Vec<RawNode>`; a
  `Vec<RawNode>` per cell is precisely the allocation the 913 is made of. The same rule is kept in
  raw bytes, which is the stricter of the two — it preserves the whitespace inside a start tag, which
  a decomposed attribute list does not record. A cell's start tag keeps the file's bytes unless
  regenerating it from `r`, `s` and `t` would reproduce them, decided by doing the regeneration and
  comparing; editing such a cell rewrites the run in place, so an unmodelled attribute survives the
  edit as well as the row.
- **One thing a file can say is refused**: a `c@r` that is not a cell reference, because the store is
  keyed on it. Rows out of order, duplicated row numbers, a `c@r` naming a different row than its
  `row@r`, cells out of column order and a `t` that disagrees with the child element present are all
  preserved as read and described by `SheetData::anomalies`, never repaired.

## [0.0.104] - 2026-09-05

Cell addressing: the SpreadsheetML reference vocabulary eleven later Phase D children consume
(MJXOFF-93, Phase D position 3).

### Added

- **`mjx_sml::address`** — `crates/mjx-sml/src/address.rs` was a module slot with a note naming this
  child; it is now the workspace's one model of a cell address. `CellReference` (`ST_CellRef`),
  `CellRange` (`ST_Ref`) in all four forms (`A1`, `A1:C3`, `A:A`, `1:1`), `CellRangeList`
  (`ST_Sqref`), `CellSpans`/`CellSpan` (`ST_CellSpans`), `SheetQualifiedReference`/`SheetName`,
  `R1C1Reference`/`R1C1Range`/`R1C1Coordinate`, the bijective base-26 conversion both ways
  (`column_letters`, `column_index_from_letters`), `Anchoring`, `ColumnBound`, `RowBound`,
  `GridBounds`, `AddressText` and `AddressError`. `ReferenceMode` (`calcPr@refMode`) is re-exported
  from the generated `sml` simple types rather than declared a second time.
- **`mjx_sml::SmlError::Address`** — the fourth variant, carrying `AddressError`. The enum stays
  deliberately exhaustive, so MJXOFF-137's facade mapping cannot silently file it under a wildcard.
- **`crates/mjx-sml/tests/fixture_addressing.rs`** — every address in `tests/fixtures/sample.xlsx`
  parses and re-emits byte-identically, asserted against a **pinned** list of the thirteen the
  worksheet carries rather than against a count, so a scan that stops finding anything fails rather
  than passing vacuously. `sample.xlsx` carries no `row@spans`, so the other half of the `spans`
  rule — never drop one that was written — is asserted against authored `x:`-prefixed markup in the
  same suite.

### Notes

- **Eight bytes, `Copy`, no allocation on the parse path.** MJXOFF-95 (D04) will parse a reference
  for every cell of a sheet that may hold 1,048,576 x 16,384 of them, so `CellReference` is a `u32`
  row, a `u16` column and two one-byte anchorings with no padding waste; parsing is one forward pass
  over `&str` with no intermediate `String`, and formatting writes into `AddressText`, a `Copy`
  48-byte stack buffer. `Copy` is the proof rather than the decoration — a `Copy` type cannot own a
  heap allocation.
- **Round-trip is exact, not canonical.** `$A$1` writes `$A$1`, `A1` never widens to `A1:A1`,
  `C3:A1` stays backwards, `RC` never becomes `R[0]C[0]`, and a `sqref`'s separator run survives
  until the list is edited. The ordered view is a separate answer (`CellRange::normalized_bounds`),
  because a reference-formatting "improvement" is an edit-isolation failure.
- **Out-of-grid input is refused, never clamped.** `XFE` is an error, not `XFD`; a row number is
  accumulated with saturating arithmetic, so an absurd digit run is reported rather than wrapped.
- **Column letters are not case-folded.** `"a1"` is a typed error rather than a silent rewrite to
  `A1`. Every producer this workspace has read writes uppercase, and folding would canonicalize a
  file that said otherwise.
- **`mjx_chart::workbook::column_letters` is superseded but not yet retired.** It returns a `String`
  per call and has no parser; MJXOFF-112 (D10) switches `mjx-chart` over to `column_letters` here and
  MJXOFF-99 (E1) retires the copy. A second, independent copy lives in `xtask/src/corpus/xlsx.rs`.

## [0.0.103] - 2026-09-05

The `mjx-xlsx` package spine: the part graph, a workbook that opens and saves without touching a
byte, and the SpreadsheetML invariants a save is held to (MJXOFF-91, Phase D position 2).

### Added

- **`mjx_xlsx::Workbook`** — the format-tier half of the Excel split MJXOFF-132 opened.
  `crates/mjx-xlsx/src/lib.rs` was thirteen lines with zero public items and an
  `assert_eq!(2 + 2, 4)` placeholder; it is now a crate with a module tree, a resolved part graph and
  a byte-exact round trip. `open`/`from_package` find the workbook part through the package-root
  `officeDocument` relationship and identify it by its **root element** (never by its content type —
  which is what lets a macro-enabled workbook open at all, see below); `sheets` reads the `x:sheets`
  list, in document order, resolving each entry's `r:id`; `parts` and `worksheet(i).parts()` resolve
  the workbook-level and sheet-level part graphs; `part_inventory` says what this crate made of every
  part; `validate`/`save`/`save_unchecked` are the write path.
- **`mjx_xlsx::parts`** — the SpreadsheetML part graph: twenty-one `PartKind`s, each pairing a
  relationship type with its content type(s), plus `SheetKind`, `WorkbookParts` and `WorksheetParts`.
  Every string is quoted from ECMA-376 Part 1 §12.3 (the theme from §14.2.7, the VML drawing from
  **Part 4 §8.2**), with the Strict `purl.oclc.org` prefix substituted for the Transitional one every
  fixture in this workspace actually carries — the same convention `mjx_docx::constants` documents.
  The four a workbook cannot open without match `mjx-chart`'s own embedded-workbook writer string for
  string, which is what MJXOFF-112 will delete.
- **`mjx_xlsx::preserve`** — the tier-1 contract written down, and `PartClassification`. A part this
  crate cannot classify is **preserved, never rejected**: a `.xlsm`'s macro-enabled workbook, a custom
  XML mapping and a vendor's private sidecar all round-trip through a crate that knows nothing about
  any of them.
- **`mjx_xlsx::SpreadsheetDefect`** — five SpreadsheetML invariants on top of `mjx-opc`'s packaging
  ones, split by scope exactly as `mjx_pptx::validate` splits its own. Two are **graph** invariants,
  checked over the whole package: the package-root `officeDocument` relationship must still name the
  workbook part (§12.3.23), and no `…spreadsheetml.*` part may be unreachable from the root. That
  second one deliberately narrows `mjx-opc`'s "an unreferenced part is not a defect" for one family
  and one reason — a shared string table nothing relates to is not dead weight, it is every `t="s"`
  cell in the workbook indexing into a table no consumer loads. The other three are **markup**
  invariants over `Package::authored_xml_parts` only, so a workbook opened and saved untouched is
  never faulted for markup it arrived with: a `x:sheet` entry must lead to a sheet, a related sheet
  part must be listed, and `@sheetId`/`@name`/`r:id` must each be unique (§18.2.19).
- **`crates/mjx-xlsx/tests/roundtrip.rs`, `part_graph.rs`, `workbook_validation.rs`** — the tier-1
  proof over the directory-derived `.xlsx` corpus (part by part on decompressed payloads, never a
  container hash), the part-graph and preservation clauses, and the refusals proved on the real
  fixture through the public `Workbook::save`.
- **Three cases added to `crates/mjx-xlsx/tests/schema_gate.rs`** (MJXOFF-110 created the file,
  MJXOFF-132 added the ordering case): invalid worksheet markup — a `s:c` planted outside its
  `s:row`, which `CT_SheetData` rejects — must fail *naming `/xl/worksheets/sheet1.xml`*, which is
  the part every later Phase D child writes into; **no** part under `xl/` may be skipped rather than
  validated, which is the general form of the four-part clause the file already pinned by name; and a
  workbook opened and saved through `Workbook` (not through `Package`) is still schema-valid and
  still in child order.
- **A two-page guide** at `crates/mjx-xlsx/docs/guide/`, every snippet a compiled doctest.

### Notes

- **The macro-enabled content types are deliberately not declared.** `macroEnabled` appears nowhere
  in ECMA-376 Parts 1-4, so declaring one would be guessing a wire token. A `.xlsm` still opens (the
  workbook part is found by its root element) and still round-trips; its workbook part simply reports
  as unclassified. `parts.rs` carries a test that fails if a later child ever adds the string, so the
  note cannot go stale silently.
- Patch digit only. No existing public identifier changed name or shape: every item here is new, in a
  crate that previously had none.

## [0.0.102] - 2026-09-05

`mjx-sml`, the shared-markup layer Excel was missing, and the first mechanical check of the layering
rule (MJXOFF-132, Phase D position 1) — the first child of Phase D.

### Added

- **`crates/mjx-sml`**, a new workspace member at rank **2.1**: beside `mjx-dml`, beneath
  `mjx-chart`. It holds the SpreadsheetML *markup* — cells, rows, sheet data, shared strings, styles,
  number formats, formulas as text — while `mjx-xlsx` keeps the `Workbook` surface and the package
  graph in the format tier. **Excel is two crates, not one**, because an authored chart embeds a
  whole `.xlsx` inside a `.pptx` or a `.docx`: SpreadsheetML is shared markup even though the package
  is Excel's. That is what makes `mjx-chart → mjx-sml → mjx-dml` a chain of downward edges, and it is
  what lets MJXOFF-112 and MJXOFF-99 finally delete `mjx-chart`'s duplicate workbook writer — with
  one Excel crate the retirement would have needed `mjx-chart → mjx-xlsx`, which points **up**.
  This child **emits no markup and models nothing**: it is the crate, the module tree (each module a
  named home carrying the work item that fills it), `SmlError`, the generated `sml` ordering table and
  the layering test.
- **`xtask/tests/layering.rs`** — the layering rule, checked. `CLAUDE.md`'s downward-only rule was
  the one architectural rule in the repository with no mechanical check, and two queued children are
  specified to rely on it existing. The test reads the real graph out of `cargo metadata --no-deps`
  (rather than scanning manifests, where an unrecognised dependency spelling would drop an edge
  silently) and fails on any normal or build edge that does not point to a **strictly lower** rank,
  naming both crates and both ranks. Sideways is as illegal as upward. It also refuses to pass on
  fewer edges than the shipped graph has, so it cannot go green by reaching nothing.
- **The `sml` child-order table.** `xtask`'s `CHILD_ORDER_SCHEMAS` gains `"sml"`, so
  `mjx_ooxml_types::child_order` now carries the `xsd:sequence` position of every child of all 367
  SpreadsheetML complex types — `CT_Worksheet`'s **39 slots**, the largest sequence in the workspace,
  among them. Five curated exports name the types the Phase D children place children into first:
  `WORKSHEET`, `WORKBOOK`, `STYLESHEET`, `WORKSHEET_ROW` and `WORKSHEET_CELL`. `sml.xsd` reaches
  `dml-spreadsheetDrawing` through `CT_ObjectAnchor`, so that schema joins
  `CHILD_ORDER_SCHEMA_DEPENDENCIES` (parsed only; its own table stays MJXOFF-107's).

### Changed

- **Every `x:`-rooted part is now audited for child order.** `mjx-schema-gate`'s `sml` entry moves
  from `OrderingCoverage::Pending { owner: "MJXOFF-132", … }` to `Generated`, which is what
  `parts_that_must_be_audited` reads. Before this, the ordering gate on a `.xlsx` recognised only
  `/xl/theme/theme1.xml` — a DrawingML part that happens to live in a workbook — and passed. A new
  case in `mjx-xlsx` pins the four SpreadsheetML parts of `sample.xlsx` as required *and* audited
  non-vacuously, from both ends of the mechanism.
- **`CLAUDE.md`, `README.md` and `PLAN.md`** state the rank table, including the sub-ranks. Two of
  them were not previously written down: shared markup is not flat (2.0 `mjx-dml` → 2.1 `mjx-sml` →
  2.2 `mjx-chart`/`mjx-omml`/`mjx-vml`) and neither are the foundations (`mjx-xml` is built on
  `mjx-ooxml-core`, so 0.0 → 0.1). A flat foundations tier would have made a shipped edge illegal.

### Removed

- Two dead rows from `xtask`'s `UNCOVERED_SCHEMAS`: `sml` (this child covers it) and `shared-math`
  (MJXOFF-134 covered it and left its row behind). A row exists for a schema a `COVERAGE.md` table
  does *not* cover; a row for one that both tables cover is read by nothing. Regenerating with the
  rows gone produces a byte-identical `COVERAGE.md`, which is the proof they were dead.

## [0.0.101] - 2026-09-05

The Word usage guide, its examples, and the `wml` preserve-only ledger (MJXOFF-150, Phase C position
22) — the last child of Phase C.

### Added

- **Four Word guide pages**, so `crates/mjx-docx/docs/guide/` is now the five-pages-plus-a-README
  shape `crates/mjx-pptx/docs/guide/` settled on, in the same reading order:
  [`text_and_formatting`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/text_and_formatting/)
  (addressing a run, positions shifting under an insert, equations, run-level content that is not
  text, comments/notes/bookmarks, tracked changes read but never applied),
  [`tables_sections_and_headers`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/tables_sections_and_headers/)
  (both kinds of merge, the grid-discrepancy report, sections ending rather than starting at a
  `w:sectPr`, header inheritance, fields, structured content), [`styles_and_inheritance`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/styles_and_inheritance/)
  (the six-rung ladder, the `w:basedOn` chain and its typed cycle error, the toggle-property XOR
  rule, numbering, the table-cell rung) and [`fidelity_and_gaps`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/fidelity_and_gaps/).
  `crates/mjx-docx/src/guide.rs` grows from two `include_str!` to six.
  **Every snippet is a compiled doctest that asserts on a value it computed** — 38 doctests in
  `mjx-docx`, up from 24 — and every one of the new ones runs against a real `Document::blank`
  rather than being `no_run`, so an assertion that stopped holding would go red rather than merely
  still compiling.
- **`crates/mjx-docx/docs/guide/fidelity_and_gaps.md`**, the artefact MJXOFF-128 (F2) named and no
  unit produced. Same structure as PowerPoint's: the round-trip guarantee, what `save` refuses,
  the content that is not WordprocessingML, and four lists under *The gaps* — **Non-goals** (twelve
  rows, each with the reason it is a decision), **Built, not yet verified against Office** (eight
  rows, F2's input; nothing here is marked verified, and no agent may mark it so), **Whole formats**
  and **What used to be here**. It also names, rather than leaves implied, that `mjx-docx` has no
  `DocumentDefect` counterpart to `mjx-pptx`'s `PresentationDefect`: Word's `save` runs the package
  graph check alone, and the WordprocessingML-level invariants are enforced at the point of each edit
  instead.
- **The `wml` preserve-only ledger**, in that page: the one complex type of 285 with no Rust type
  (`CT_ShapeDefaults`, whose whole content model is `xsd:any` in the VML office namespace), the
  elements typed as `Unmodeled` and why each is (most are `CT_Empty`, where "unmodelled" is the
  complete truth), the eight clusters whose *reference* is typed and whose *payload* is preserved
  (`w:altChunk` payloads, custom XML data, printer settings, embedded fonts, OLE streams, `w:subDoc`,
  charts and SmartArt inside a `w:drawing`, VML beyond `mjx-vml`'s coverage), and the one
  deliberately unmodelled recursion (`w:divsChild`).
- **Eight new `mjx-docx` examples**, taking the crate from one to nine and matching `mjx-pptx`'s
  eight: `read_document`, `edit_text`, `build_table`, `styles_and_numbering`, `sections_and_headers`,
  `fields_and_hyperlinks`, `annotations` and `structured_content`. Each takes a CLI argument with a
  `target/examples/` default, prints section banners, and **reopens what it wrote and asserts on
  it** — `read_document` compares every part's decompressed payload against the original to prove
  that reading dirties nothing, and `edit_text` proves the converse for four parts nothing addressed.
- **`crates/mjx-docx/examples/support/mod.rs`**, `mjx-pptx`'s four helpers (`fixture_dir`, `fixture`,
  `template`, `output_path`) by the same names and signatures, with `fixture_dir` delegating to
  `mjx_fixtures::fixtures_dir()` rather than recomputing the path — two spellings of one directory is
  the drift `mjx-fixtures` exists to end. `blank_document.rs` moves onto it, losing its own private
  copy of `output_path`.

### Changed

- `mjx-ooxml`'s crate-level *Guides* section lists all five Word pages by name, as it already did for
  PowerPoint's five, instead of naming only `building_a_document`.

## [0.0.100] - 2026-09-05

The Word facade, the error mapping and both bindings (MJXOFF-139, Phase C position 21) — closes
Phase C. `mjx_ooxml::Document` (`crates/mjx-ooxml/src/document.rs` + `document/`, 14 files) is the
curated Word surface over `mjx_docx::Document`, mirroring [`Deck`]'s own treatment of
`mjx_pptx::Presentation`: [`BlockPath`]/[`RunPath`] (`u32`-addressed) replace `impl Into<BlockPath>`/
`impl Into<RunPath>`, a concrete return type replaces every closure `mjx_docx::Document` takes to
read or edit a part, and `DocxError`'s 35 variants collapse into the existing eleven `ErrorCode`s —
none needed a twelfth.

**The curated surface**: lifecycle (`open`/`blank`/`save`/`save_unchecked`/`validate`/`conformance`),
paragraph and run reading/editing, effective properties (run, paragraph, table cell — a documented
subset of the full ladder, not every field), read-only style lookup, numbering attach/detach,
sections (page size/margins) and headers/footers, tables (dimensions, spans, merges, structural
insert/remove), fields, hyperlinks, comments, footnotes/endnotes, revisions (read-only), and inline
pictures. Mail merge, web settings, the font table, recipients, bookmarks, move ranges, custom-XML
data binding, `altChunk`, the glossary document, legacy form fields and equations stay reachable
through [`Document::document_mut`] — documented on [`Document`]'s own module doc, including why
content controls need no dedicated method (MJXOFF-138 already made paragraph/run addressing recurse
through one transparently, so nothing new was needed to reach a content control's own text).

**Both bindings** (`bindings/mjx-python`, `bindings/mjx-wasm`) project the same curated surface:
`mjx_ooxml.Document` in Python (identity-mapped, reusing the existing eleven exception classes) and
`Document` in the wasm package (camelCase, `free()`-mandatory, reusing `CellExtent`/`CellAddress`
for the `(rows, columns)`/`(row, column)` pairs `Deck` already projects the same way). New value
classes and enumerations extend the existing `value_class!`/`sealed_enums!`/`open_enums!` machinery
in both bindings rather than inventing a second pattern. The Python `.pyi` stub carries every new
class and method; `bindings/mjx-wasm/tests/node/document_surface.mjs` and (unverifiable locally —
`maturin`/`pytest` are absent) `bindings/mjx-python/tests/test_document_surface_coverage.py` are the
mis-wiring guards, the Word siblings of `surface.mjs`/`test_surface_coverage.py`.

**One walkthrough, three languages** — `crates/mjx-ooxml/examples/build_a_document.rs`,
`bindings/mjx-python/tests/test_build_a_document.py`,
`bindings/mjx-wasm/tests/node/build_a_document.mjs` — a document authored from `Document::blank`
through the curated surface only (paragraphs and runs, a numbered list, a hyperlink, a table, a
header, a comment, a footnote), saved and reopened.

**`Deck::open`'s Word refusal now names `Document::open`**, and `Format::is_editable` covers
WordprocessingML alongside PresentationML — Excel remains detected but not editable.

**Fixed while writing the Rust walkthrough, in `mjx-docx` (MJXOFF-124's own code, not this child's
facade)**: `Document::add_footnote`/`add_endnote` silently lost every entry — the two reserved
separator entries included — on save + reopen, whenever called on a document with no existing
`footnotes.xml`/`endnotes.xml`. `create_footnotes_part`/`create_endnotes_part` wrote a literal
`<w:footnotes xmlns:w="...">` template, then wrote back a *fresh* `Footnotes::blank(interner)` —
built with `attributes: Vec::new()` — over the just-parsed root to seed the two reserved entries,
discarding the `xmlns:w` the parse had just preserved; the saved bytes were schema-shaped XML with
every `w:footnote` child under a `w:footnotes` root that never declared its own `w:` prefix, so a
reopen resolved no `w:` element at all. `Footnotes::seed_reserved_entries`/
`Endnotes::seed_reserved_entries` push the reserved entries onto the already-parsed value instead of
replacing it — the same safe shape `create_comments_part`/`create_header_footer` already used.
Reproduced directly against `mjx-docx`'s own public API, independent of the facade; two regression
tests added, mutation-proved.

The `wml` ownership audit (MJXOFF-139's other deliverable, per MJXOFF-133's template) is in this
child's own pull request description: every member of `CT_Body`'s content groups
(`EG_BlockLevelElts`/`EG_BlockLevelChunkElts`/`EG_ContentBlockContent`/`EG_RunLevelElts`/
`EG_PContent`/`EG_ContentRunContent`/`EG_RPrBase`) and all fourteen of `wml.xsd`'s global elements,
each mapped to its owning MJXOFF id — nothing unowned.

## [0.0.99] - 2026-09-05

Content controls, custom XML, smart tags, `w:dir`/`w:bdo`, `w:altChunk` and the glossary document's
building blocks (MJXOFF-138, Phase C position 20) — `crates/mjx-docx/src/document/structured_content.rs`
(new). MJXOFF-69–74 named no owner for this cluster either; the other half of the third of `wml.xsd`
Phase C's own six specifications never allotted.

**`w:sdt` and `w:customXml` are both members of all four content groups** (`EG_ContentBlockContent`,
`EG_ContentRunContent`, `EG_ContentRowContent`, `EG_ContentCellContent`), so a content control or a
custom-XML wrapper can appear anywhere a paragraph, a run, a table row or a table cell can.
[`ContentControlBlock`]/[`ContentControlRun`]/[`ContentControlRow`]/[`ContentControlCell`] and
[`CustomXmlBlock`]/[`CustomXmlRun`]/[`CustomXmlRow`]/[`CustomXmlCell`] type every placement — and
each one's own content **reuses** the exact enum ([`BlockContent`], [`ParagraphContent`],
[`TableContent`], [`RowContent`]) its placement already has a container for, so MJXOFF-92's
paragraph/run APIs and MJXOFF-116's row/cell addressing reach through a wrapper unchanged.
[`Table::rows`]/[`Row::cells`] now recurse into a row- or cell-level wrapper, so `(row, column)`
addressing stays correct when a repeating-section control wraps one or more rows;
[`Paragraph::text`]/[`Paragraph::run`] now reach through a run-level wrapper the same way they
already reach through `w:hyperlink`.

**`w:sdtPr`** — [`ContentControlProperties`]: the twelve control kinds ([`ContentControlKind`]:
rich text, plain text, picture, combo box, drop-down, date, building-block gallery/list, group,
citation, bibliography, equation), the lock ([`Lock`]), the placeholder ([`Placeholder`]) and the
XML data binding ([`DataBinding`]) all get typed accessors, plus `set_lock`/`set_placeholder`/
`set_data_binding` writers that insert at `CT_SdtPr`'s own schema rank regardless of call order.
`w14:`/`w15:` extensions (checkbox, repeating section) round-trip through the unknown-element bucket
— dropping them would turn a working form into inert text.

**A content control's data binding is a two-part reference** —
[`Document::resolve_data_binding`] enumerates every related Custom XML Data Storage part
(`customXml/itemN.xml`), matches `storeItemID` against each one's own properties part
(`customXml/itemPropsN.xml`'s `ds:itemID`), and resolves `xpath` (the absolute, `[n]`-indexed
element path Word itself emits) against the matching part's own tree — [`resolve_xpath`]. A binding
naming a part the package does not carry reports [`DocxError::DataBindingPartNotFound`], never a
panic.

**`w:altChunk`** — [`AltChunk`]/[`AltChunkProperties`]: an embedded HTML/RTF/`.docx` part this crate
imports the relationship for and never converts. [`Document::add_alt_chunk`]/
[`Document::alt_chunk_payload`]/[`Document::alt_chunk_parts`] round-trip the payload, the relationship
and the content type byte-identically.

**The glossary document** — [`Document::glossary_document`] reads `word/glossary/document.xml`'s own
[`DocParts`]/[`BuildingBlock`] list; a building block's own content is an ordinary [`Body`]
([`BuildingBlock::body`]) — the exact same block-content API the main document body uses, no
glossary-specific duplicate.

**The adversarial fixture** `structured_content.docx` nests a run-level control inside a paragraph
inside a cell-level control inside a table inside a block-level control, with a `w:customXml`
row wrapper and a repeating-section row-level control wrapping two `w:tr` interleaved alongside it —
proved, with a mutation to each of the two recursion sites (row-level `w:customXml`, run-level
`w:sdt`) confirmed red and reverted by hand.

`mjx-schema-gate`: two new `PreservedForeignMarkup` entries — Custom XML Data Storage Properties'
own fixed namespace, and this fixture's own representative Custom XML Data Storage namespace —
classifying both as foreign markup rather than validating them, pinned by a test.

`Body::content`/`Paragraph::content` are now `pub`, matching the precedent
`Table::content`/`Row::content`/`Cell::content` already set (MJXOFF-116).

## [0.0.98] - 2026-09-05

`word/settings.xml`, `word/webSettings.xml`, `word/fontTable.xml` and `word/recipients.xml`
(MJXOFF-136, Phase C position 19) — `crates/mjx-docx/src/document/{settings,web_settings,
font_table,mail_merge}.rs` (all new). MJXOFF-69–74 allotted Word six children and, between them,
named no owner for a third of `wml.xsd`; this child takes the document-configuration half.

**`CT_Settings` — all 98 children modelled**, none silently dropped: [`DocumentSettings`] and
[`SettingsContent`]. Every `CT_OnOff` flag, `CT_DecimalNumber` and `CT_TwipsMeasure` leaf gets a
full, individually named get/set pair; every "own type" child (`w:view`, `w:zoom`,
`w:documentProtection`, `w:compat`, `w:docVars`, `w:rsids`, `w:mailMerge`, `w:captions`, …) gets a
full get/set pair over its own richly typed value. `w:compat`'s sixty-two individual flags are the
one deliberate exception: each is a bare `CT_OnOff` nothing in Phase C names by name, so they round
-trip exactly through the unknown-element bucket rather than each getting a near-identical bespoke
method; `w:compatSetting` (the schema's own generic escape hatch) is fully typed.
`sl:schemaLibrary` (a foreign Smart Tag schema namespace) is the only schema-known child left
read-only, preserved via the same bucket. `m:mathPr` wires in `mjx-omml`'s `MathProperties`
(MJXOFF-134's own module doc named this exact seam). `Document::even_and_odd_headers` now reads
through `DocumentSettings` instead of MJXOFF-113's ad-hoc raw-tree scan.

**`word/webSettings.xml`** — [`WebSettings`]: legacy framesets (`CT_Frameset`, recursively boxed)
and the `w:div`/`w:divs` tree `w:divId` (`CT_PPrBase`, C4) points into. `w:divsChild` (`CT_Div`'s own
recursive nesting) is deliberately unmodelled — see that module's own doc comment.

**`word/fontTable.xml`** — [`FontTable`]/[`Font`]: identity, PANOSE classification, character set,
family, pitch, signature, and the four embedded-font relationships. An embedded font's binary
payload and its `fontKey` obfuscation key are opaque — never re-encoded, never decoded.

**`word/recipients.xml`** — [`Recipients`]/[`RecipientData`], the Mail Merge Recipient Data part;
`DocumentParts::recipients` (C1 declared `PartKind::Recipients` but never resolved it).
`w:settings/w:mailMerge` and its ODSO (Office Data Source Object) cluster — [`MailMergeSettings`],
[`Odso`] — share the mail-merge vocabulary with this part.

**`w:documentProtection`'s password hash is preserved exactly, never recomputed, never cleared** —
typed as opaque text (the base64 wire form needs no decoding), proven by an edit to an unrelated
flag leaving the hash byte-identical.

**The adversarial fixture** `settings_document_configuration.docx` carries `w14:`/`w15:` elements
interleaved between modelled ones inside `word/settings.xml`, two `w:compatSetting` entries, a
`w:docVars` block, an embedded font, and both new parts — round-tripped byte-identically through the
typed model, with the unknown-bucket order proved to survive exactly (a mutation that drops it was
applied by hand, confirmed red, and reverted).

## [0.0.97] - 2026-09-04

Office MathML in Word: `mjx-omml` ends the Phase 0 scaffold deferral (MJXOFF-134, Phase C position
18) — `crates/mjx-omml/src/{support,leaf,arg,objects,math,properties}.rs` (all new), 2,638 lines from
13. `crates/mjx-docx/src/document/{body,mod,revisions}.rs` (Word-side integration).

**All 72 `shared-math.xsd` complex types are modelled.** `Math` (`m:oMath`), `MathParagraph`
(`m:oMathPara`), `Argument` (`CT_OMathArg`, the recursive core every object's operand slot bottoms
out at), `Run` (`m:r`), `Text` (`m:t`), and every math object — accent, bar, box, border box,
delimiter, equation array, fraction, function-apply, group character, lower/upper limit, matrix
(with its row/column/column-properties family), n-ary operator, phantom, radical, and the four
script forms — with its own paired `*Pr` properties type. Twenty leaf `CT_*` value types (`CT_OnOff`,
`CT_Shp`, `CT_Integer255`, …) collapse into one shared read/write mechanism rather than twenty
near-identical Rust types, and six single-`ctrlPr`-child `*Pr` types collapse into
`ControlOnlyProperties` — the same "one shape, many meanings" reuse `mjx-docx` already established
for `CT_OnOff`/`CT_String`. Consumes MJXOFF-144's generated `mjx-ooxml-types::officemath` simple
types throughout.

**The layering tension `CT_CtrlPr` poses — a `wml`-typed `w:rPr`/`w:ins`/`w:del` nested inside a
`shared-math` type, which the schema itself only resolves by importing `wml.xsd` — is resolved by
preserving `ControlProperties`'s children wholesale and raw**, the same mechanism `mjx-dml`'s
`WordprocessingGroup`/`WordprocessingCanvas` already use for their own WordprocessingML-typed member
content. `mjx-docx` (which depends on `mjx-omml`) adds typed accessors over a `ControlProperties`'s
raw children where it needs them: `MathControlInsert`/`MathControlDelete` (`CT_MathCtrlIns`/
`CT_MathCtrlDel`) and `math_control_properties`, the reachable call site MJXOFF-126 declined those
two types for want of.

**Word-side integration:** `ParagraphContent` grows `Math`/`MathParagraph` variants (`m:oMath`/
`m:oMathPara`, folded in from `EG_RunLevelElts`'s own `EG_MathContent` — a sibling of `w:r`, not
nested inside one), wired into all four `Vec<ParagraphContent>` hosts. `Paragraph` grows
`append_math`/`append_math_paragraph`/`equations`/`equations_mut`; `Document` grows `append_math`
(closure-based, mirroring `edit_numbering`) and `set_equation_run_text` (an edit several nesting
levels deep, through the same `ToXml::write_back` span-preserving path every other mutation in this
crate uses).

**A real bug the crate's own integration tests caught:** `shared-math.xsd` is
`attributeFormDefault="qualified"` (the only other modeled schema besides `wml.xsd` with this shape),
so every `val`/`alnAt` attribute is wire-qualified `m:val`/`m:alnAt`, never bare — fixed in
`crate::support`'s `VAL_ATTRIBUTE_PREFIX`. A freshly authored equation spliced into a blank
document's `word/document.xml` (which binds only `w:`/`r:`) also produced markup using the
undeclared `m:` prefix — `Document::append_math` now declares it on the newly inserted subtree's own
root, the same pattern `document/drawing.rs`'s `Drawing::new` already established for `wp:`/`a:`/
`pic:`.

**Codegen:** `shared-math` moves from `CHILD_ORDER_SCHEMA_DEPENDENCIES` to `CHILD_ORDER_SCHEMAS` (its
own generated child-order table) and gains 43 `CHILD_ORDER_EXPORTS` rows. No new entry was needed in
the schema gate's `schema_for_namespace`: `m:` never roots a part of its own — `wml.xsd` already
imports `shared-math.xsd`, so `word/document.xml`'s own validation against `wml.xsd` already covers
nested `m:` content transitively, proved by a mutation (`m:f` missing its required `m:num`) that
turns the sweep red naming the part.

## [0.0.96] - 2026-09-04

DrawingML in Word: `w:drawing`, `w:pict`, `w:object` and `w:control` (MJXOFF-131, Phase C position
17): `crates/mjx-dml/src/wordprocessing_drawing.rs` (new), `graphic.rs` (new), `picture.rs` (new),
`shape_properties.rs` (new), `nonvisual.rs` (new), `crates/mjx-docx/src/document/drawing.rs` (new).

**The `wp:` schema (`dml-wordprocessingDrawing.xsd`, 287 lines, 20 complexTypes) had no model
anywhere in `crates/` before this child.** Seventeen of the twenty land in `mjx-dml` —
`wp:inline`/`wp:anchor`, the five wrap modes and `wp:wrapPolygon`, `wp:graphicFrame`,
`wp:wgp`/`wp:wpc`, `wp:contentPart` — reusing `mjx-dml`'s existing `SolidFill`/`Transform2D`/
`PresetGeometry`/`CustomGeometry`/`LineSpec`/`EffectList` pieces through a new `ShapeProperties`
(`a:CT_ShapeProperties`, the type MJXOFF-107 and this ticket both found missing) and a new
`Picture`/`Graphic`/`GraphicData` (`pic:pic`, `a:graphic`/`a:graphicData`). The remaining three
(`CT_WordprocessingShape`, `CT_TextboxInfo`, `CT_TxbxContent`) live in `mjx-docx` instead: their
content is `w:EG_BlockLevelElts`, WordprocessingML's own vocabulary, so typing them below `mjx-docx`
would reach `mjx-dml` upward past its own tier. `TextBoxContent` reuses `body.rs`'s own
`BlockContent`/`block_paragraph*` mechanism as its sixth container (MJXOFF-126's "extend, don't
copy"), so a text box's paragraphs read through the same model every other container does.

**`Document::add_inline_picture`/`remove_drawing`** add the image-part/relationship/content-type
plumbing a picture needs (and sweep the media part on removal via `Package::
remove_unreferenced_parts`, so a removed picture leaves no orphan); **`Document::
paragraph_run_content`** is the new public reading surface for a paragraph's own `w:drawing`/
`w:pict`/`w:object`/`w:control` content. `w:pict` needed no new wrapper at all: it reads and writes
directly as `mjx_vml::Drawing`, the same type MJXOFF-113 already uses for a header's own `w:pict`.

**A cross-schema element-namespace bug found while building the fixture, fixed the same session:**
`NonVisualDrawingProps`/`ShapeProperties`/`PictureFill`'s constructors defaulted their element to
DrawingML-main (`a:`), which is correct only when the host schema genuinely is `dml-main.xsd` —
`cNvPr`/`blipFill`/`spPr` inside `pic:pic` and `docPr` inside `wp:inline`/`wp:anchor` are *local*
element declarations of their own host schema (`dml-picture.xsd`, `dml-wordprocessingDrawing.xsd`)
and take that schema's own namespace on the wire (`pic:cNvPr`, `wp:docPr`), never literally `a:`.
Each type now also has a `with_name`/`new_qualified` constructor taking the host's own qualified
name, used at every pic:/wp: call site.

**Codegen:** `dml-wordprocessingDrawing` moves from `CHILD_ORDER_SCHEMA_DEPENDENCIES` to
`CHILD_ORDER_SCHEMAS` (its own generated child-order table) and gains a `SimpleTypeModule` for its
five `ST_*` enums (`WrapText`, `HorizontalAlignment`, `HorizontalRelativeFrom`, `VerticalAlignment`,
`VerticalRelativeFrom` — the last four curated overrides on the shared naming table, since the
schema's own `H`/`V` suffixes are ECMA-376's own contraction for "Horizontal"/"Vertical"). No new
entry was needed in the schema gate's `schema_for_namespace`: `wp:` never roots a part of its own —
`wml.xsd` already imports `dml-wordprocessingDrawing.xsd`, so `word/document.xml`'s own validation
against `wml.xsd` already covers nested `wp:`/`pic:` content transitively.

## [0.0.95] - 2026-09-04

Word revision marks: tracked changes as a first-class case in every mutation path (MJXOFF-126,
Phase C position 16): `crates/mjx-docx/src/document/revisions.rs` (new).

**`w:ins`/`w:del`/`w:moveFrom`/`w:moveTo` are typed as run-level containers, recursively** — an
insertion nested inside a deletion is one `ParagraphContent::Ins` inside another `Del`'s own
content, exactly the shape a real tracked-change history produces. `crates/mjx-docx/src/document/
revisions.rs` adds `RunTrackChange` (`CT_RunTrackChange`), `TrackChangeMarker` (bare `CT_TrackChange`
— `w:cellIns`/`w:cellDel`, the paragraph mark's own `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo`, the four
`customXml*RangeStart` elements), `MoveBookmark` (`CT_MoveBookmark`), `CellMergeTrackChange`,
`TrackChangeNumbering`, and eight `*Change` property wrappers (`RunPropertiesChange`,
`ParagraphPropertiesChange`, `ParagraphMarkPropertiesChange`, `SectionPropertiesChange`,
`TablePropertiesChange`, `TableExceptionPropertiesChange`, `TableGridChange`, `CellPropertiesChange`,
`RowPropertiesChange`), each reusing the live property type MJXOFF-94/96/109/119 already built for
its own "previous properties" payload rather than a parallel type.

**One rule, structurally enforced, for every mutation path in this crate:** `w:ins`/`w:del`/
`w:moveFrom`/`w:moveTo` are opaque containers to ordinary run/paragraph addressing, field scanning
and range resolution — content nested inside one is preserved exactly but never reached by the
editing surface, and every property setter already only ever replaces or inserts the one content
variant it owns, so a `*Change` sibling is never disturbed. `revisions.rs`'s own module doc carries
the full mutation-path table (MJXOFF-92/109/116/119/121/124, each with a stated and tested
behaviour). `Document::revisions`/`text_with_revisions_accepted`/`text_with_revisions_rejected` are
new read-only entry points (enumeration and computed accept/reject text — the ticket's own required
"Reading" bullet); mutating accept/reject operations are declined, with the reasoning recorded in
the same module doc.

**A malformed `w:date` is preserved verbatim, never normalised, and refused on authoring** — the
same fidelity-vs-validity split `fields.rs` (MJXOFF-121) established for over-long strings, now
applied to `ST_DateTime`, via a new `DocxError::MalformedDateTime`.

**Two corrections to this ticket's own pre-dispatch note**, both verified directly against
`wml.xsd`: `CT_TrackChangeNumbering` is *not* unreachable — it has two real use sites (`w:numPr`'s
own `numberingChange` and `w:fldChar`'s own), both already wired as `Unmodeled` placeholders by
MJXOFF-96/121 awaiting this child — and is modelled here. `CT_TrackChangeRange` genuinely *is*
unreachable (declared, never referenced anywhere in `wml.xsd`) and is not modelled.
`CT_MathCtrlIns`/`CT_MathCtrlDel` are reached only through `shared-math.xsd`, which no math content
in a Word run is typed against yet, so neither has a reachable call site in this crate either.

## [0.0.94] - 2026-09-04

Word comments, footnotes, endnotes and bookmarks (MJXOFF-124, Phase C position 15):
`crates/mjx-docx/src/document/annotations.rs` (new), `ranges.rs` (new).

**`word/comments.xml`, `word/footnotes.xml` and `word/endnotes.xml` are typed, read, round-tripped and
authored** — three of `wml.xsd`'s fourteen global elements that could not be reached at all before
this child. Each is read/edited through a `style_sheet`-shaped pair
(`Document::comments`/`edit_comments`, `footnotes`/`edit_footnotes`, `endnotes`/`edit_endnotes`),
created on demand with its content type and relationship, and `Document::add_comment`/`add_footnote`/
`add_endnote`/`add_bookmark` wrap the whole target paragraph in the matching range markers or
reference and assign a fresh id — never one already in use.

**One range-resolution mechanism (`ranges.rs`) serves both `w:bookmarkStart`/`w:bookmarkEnd` and
`w:commentRangeStart`/`w:commentRangeEnd`, pairing every marker by its own `id` attribute alone —
never by a stack.** ECMA-376 Part 1 §17.13.6.2 states the rule directly ("matched … by matching the
value of the id attribute"); a stack pairs whichever range opened most recently with the next end
marker it sees, which is wrong the instant two ranges overlap without nesting. A hand-built fixture
(`A` starts, `B` starts, `A` ends, `B` ends — no writer that only emits well-nested ranges can produce
it) proves this both ways: it resolves correctly against the shipped `id`-keyed implementation, and a
LIFO-stack mutation of the same function turns exactly that one test red. `RangeIndex::build` takes a
classifier closure rather than being hard-coded to one marker kind, so MJXOFF-126's own
`moveFromRangeStart`/`moveToRangeStart`/`customXml*RangeStart` reuse the same engine once they have
typed variants. Range resolution recurses into every table cell (not into a `w:hyperlink`'s own nested
content, a documented scope limit), so a bookmark starting inside a cell and ending after the table
resolves correctly.

**The reserved `separator`/`continuationSeparator`/`continuationNotice` footnote/endnote entries are
identified by `w:type`, never by `w:id`.** The ticket's own "conventionally ids `0`/`-1`" turned out
to be exactly that — a convention: ECMA-376 Part 1's own worked examples (§17.11.1, §17.11.23) use
`id="1"` and `id="0"`, not `-1`/`0`. `FootnoteEndnote::is_user_visible` and
`Footnotes::user_footnotes`/`Endnotes::user_endnotes` filter on `w:type` alone; a freshly authored
part always carries both reserved entries (Word repairs a file that lacks them), and the part itself
is never removed even once every user footnote is gone.

**MJXOFF-121's `Hyperlink::anchor` seam is closed, not left as a documented gap**:
`Document::resolve_bookmark` takes the raw anchor name `HyperlinkTarget::Anchor` carries and resolves
it against the body's own bookmark index, returning the bookmark's id and the text it covers, or
reporting an unmatched start (real files have these; ECMA-376 calls them non-conformant, not
impossible) rather than panicking.

Section-level `w:footnotePr`/`w:endnotePr` (`FootnoteProperties`/`EndnoteProperties`: position, number
format, start number, restart rule) — left `Unmodeled` by MJXOFF-109 for this child — are typed too,
via two new curated `mjx_ooxml_types::child_order` constants (`FOOTNOTE_PROPERTIES`,
`ENDNOTE_PROPERTIES`).

## [0.0.93] - 2026-09-04

Word fields, hyperlinks and form fields (MJXOFF-121, Phase C position 14): `crates/mjx-docx/src/document/fields.rs` (new), `hyperlinks.rs` (new).

**Both field wire forms — `w:fldSimple` and the `begin`/`separate`/`end` `w:fldChar` sequence — read
through one model, [`Field`], with instruction and cached result always distinct accessors.**
Nesting (a `TOC` field's cached result containing its own `PAGEREF` fields) is paired with a
recursive-descent stack, not a counter: a mutation that counts markers instead of nesting them turns
three tests red, including the committed `fields_and_hyperlinks.docx` fixture's own nested-`TOC`
case. An instruction split across several `w:instrText` runs concatenates for reading and, on write,
collapses to a single new run positioned at the edited field's own marker — every other field, and
every other part, stays byte-identical (proved both directions: editing an instruction leaves the
cached result untouched, and vice versa). A `w:fldChar` sequence that does not balance —
schema-valid markup ECMA-376 imposes no ordering constraint on — is a typed error
(`DocxError::UnbalancedField`), never a panic or a silent mispairing; a field with no `separate` (a
legal, resultless field) reads correctly and is not an error.

**Hyperlinks** (`Hyperlink`, typed for `r:id`/`anchor`/`tgtFrame`/`tooltip`/`docLocation`/`history`;
`Document::insert_hyperlink`/`remove_hyperlink`/`hyperlink_target`) wrap the runs they link, matching
WordprocessingML's own structural (not attribute) model. Adding one creates a valid external
relationship; removing one removes it — unless another hyperlink still names the same relationship —
and `Package::validate` reports no orphan either way. `w:anchor` (a bookmark name) is read
unresolved; MJXOFF-124 owns the bookmark index it would resolve against.

**Form fields** — `FormFieldData` (`w:ffData`) and its checkbox/drop-down-list/text-input kinds —
round-trip names, help/status text, macros and each kind's own options
(`Document::insert_form_field`/`edit_form_field`/`form_field`). Four `ST_*` members are
length-bounded strings, not enumerations (`ST_FFName` 65, `ST_FFHelpTextVal` 256,
`ST_FFStatusTextVal` 140, `ST_MacroName` 33); every setter refuses an over-long value with
`DocxError::ValueTooLong` at the API boundary rather than writing schema-invalid markup, while
reading an already-over-long value from an untrusted file is never rejected. `CT_FFCheckBox`,
`CT_FFDDList` and `CT_FFTextInput` are `xsd:sequence`-shaped (unlike `CT_FFData`'s own unordered
`xsd:choice`); every setter on the three places a new member at its schema rank via three curated
`mjx_ooxml_types::child_order` constants added for this child (`FORM_FIELD_CHECK_BOX`,
`FORM_FIELD_DROP_DOWN_LIST`, `FORM_FIELD_TEXT_INPUT`) — an append-only first draft of
`FormFieldDropDownList::set_selected_index` wrote `w:result` after every `w:listEntry`, which is
schema-invalid; the committed fixture's own schema-gate test catches the regression directly.

## [0.0.92] - 2026-09-04

Word table properties, table styles and conditional formatting (MJXOFF-119, Phase C position 13):
`crates/mjx-docx/src/document/table_properties.rs` (new), `table_regions.rs` (new), and the
`CT_TblPrBase`/`CT_TrPrBase`/`CT_TcPrBase` rungs `tables.rs`/`styles.rs` left opaque.

**Every remaining member of `CT_TblPrBase`, `CT_TblPrExBase`, `CT_TrPrBase` and `CT_TcPrBase` is
typed.** `Table`'s own `w:tblPr` carries a real `TableProperties` (was `Unmodeled`); `Row` gains
`w:tblPrEx` (`TableExceptionProperties`, the row-level override the table's own properties for that
row alone) and `w:trPr` (`RowProperties`); `CellProperties` grows from three typed members
(`gridSpan`/`hMerge`/`vMerge`, MJXOFF-116) to all fourteen. `w:style[@type='table']`'s own base
`w:tblPr`/`w:trPr`/`w:tcPr` and each `w:tblStylePr`'s own (MJXOFF-101's `TableStyleOverride`) reuse
these same three types directly — verified against `wml.xsd` that they are the identical complex
types, not merely similarly-shaped ones, so there is one table-formatting model, not two.

**Table-style conditional-formatting resolution** (`table_regions.rs`): which of a table style's
twelve regions (`ConditionalFormatRegion`, a reuse of the generated `TableStyleOverrideType`) cover a
cell is computed once per `(row, column)` from `w:tblLook`'s six flags and the band sizes
(`applicable_regions`), in the **application order ECMA-376 Part 1 §17.7.6.6 states verbatim** — whole
table, banded columns, banded rows, first/last row, first/last column, corners — so **column edges
beat row edges** and **row banding beats column banding**, both easy to get backwards and each pinned
by its own test (`tests/table_formatting.rs`). `w:tblLook`/`w:cnfStyle`'s legacy `val` bitmask is
preserved for round-trip and never consulted for region membership — Part 1's own prose for both
elements documents only the named `ST_OnOff` attributes.

**The table style joins the toggle-property XOR as a fourth term**, not a plain override rung:
`combine_toggle`/`recombine_toggles` (MJXOFF-106) gain a `table` operand alongside numbering,
paragraph-style and character-style, so a bold table style layered over a bold paragraph style
resolves to *not* bold (`true XOR true`), matching ECMA-376 Part 1 §17.7.3's own "true for an odd
number of levels" rule generalized to a fourth level.

`Document::effective_cell_fill`/`effective_cell_border`/`effective_cell_run_properties` are the three
new readers, named and shaped after `mjx_pptx::Presentation`'s own `effective_cell_*` trio (table
index in place of a slide surface plus shape path, since a `.docx` has no layout/master analogue).
`crates/mjx-docx/docs/effective_properties.md` documents the extended ladder and the region
precedence, with a compiled doctest.

## [0.0.91] - 2026-09-04

Word tables (MJXOFF-116, Phase C position 12): the grid, rows, cells, spans and structural edits —
`crates/mjx-docx/src/document/tables.rs`.

**`CT_Tbl`/`CT_Row`/`CT_Tc` are typed, and `w:tbl` stops being opaque.** `BlockContent::Table` (MJXOFF-92's
own enum, shared by `Body`, `HdrFtr` and now a table cell) carries the new `Table` model instead of
`Unmodeled`, so a table nests inside a cell for free — the same enum, no depth counter of its own,
bounded by the parse-time nesting limit `mjx-xml` already enforces. `BlockContent` also grows
`Properties(CellProperties)` for `w:tcPr`, mapped (never constructed) by `Body` and `HdrFtr` for the
same exhaustive-match reason `SectionProperties` already is.

**Word's vertical merge is a continuation model, not a span model**, and the grid resolution and
structural edits are built on that distinction (ECMA-376 Part 1 §17.4.84, Annex L.1.5.9): `w:gridSpan`
states a horizontal span in one place with no covered cell created, but `w:vMerge` states a vertical
merge by repeating a bare marker on every covered row's own `w:tc` — so a row's physical cell count is
not its column count, and `(row, column)` addressing (`Table::cell`/`cell_span`/`merge_anchor`) walks
each row accumulating `gridSpan` rather than indexing directly. `Document::cell_span`/
`merged_cell_anchor` mirror `mjx_pptx::Presentation`'s own names, `(row, column)` argument order and
return shape. `insert_row`/`remove_row` rewrite `w:vMerge` markers (a removed anchor row promotes the
cell below it, which takes over the whole removed cell's content); `insert_column`/`remove_column` grow
or shrink a straddled `w:gridSpan` — none of the four needs a `rowSpan` number to keep in step, because
Word's own model never has one. `Table::grid_discrepancies` is the active surface for a malformed grid
(a short row, an orphaned `w:vMerge` continuation, an empty row) real files can carry — exposed, never
panicked on.

Fixture: `tests/fixtures/ragged_table.docx`, a hand-authored, deliberately ragged 4×4 table (no
committed Word fixture carried a `w:tbl` before this) — a `w:gridSpan="2"` in a different place in
three of its four rows and a three-row `w:vMerge`, so cell index and grid column genuinely disagree
in three of four rows.

## [0.0.90] - 2026-09-04

Word headers and footers (MJXOFF-113, Phase C position 11): `CT_HdrFtr`, variant resolution, and the
legacy VML they carry — `crates/mjx-docx/src/document/headers.rs`.

**Header/footer parts reuse MJXOFF-92's block-content addressing rather than duplicating it.**
`body.rs`'s paragraph-vec logic (`paragraph`/`paragraph_mut`/`insert_paragraph`/`append_paragraph`/
`remove_paragraph`) is now five free functions (`block_paragraph[_mut]`, `block_insert_paragraph`,
`block_remove_paragraph`, …) operating on `&[BlockContent]`/`&mut Vec<BlockContent>`; `Body` delegates
to them, and the new `HdrFtr` (`CT_HdrFtr`, reusing `BlockContent` itself — `w:sectPr` is mapped in its
own `#[xml(children, …)]` list purely so the derive macro's exhaustive match compiles, never
constructed) uses the same functions. A header's paragraphs and runs are ordinary
`Paragraph`/`Run` — MJXOFF-94's run properties, MJXOFF-96's paragraph properties and MJXOFF-106's
effective-property ladder already work inside a header with no further wiring.

**Variant resolution — `Document::resolve_header`/`resolve_footer` — implements ECMA-376 Part 1
§17.10.1/.5/.2/.6, not a lookup.** A first/even query whose governing flag (`w:titlePg`/
`w:evenAndOddHeaders`) is off downgrades to the default (odd) query *before* the previous-section
inheritance walk runs — confirmed against the prose directly: *"If \[`titlePg`\] is set to false and a
first page header/footer is specified, then it shall be ignored and only the odd page header/footer
shall be displayed"* (§17.10.6), identically for `evenAndOddHeaders` (§17.10.1) and the even variant.
Inheritance is per-variant, from the nearest preceding section that states that specific type
(§17.10.5/.2, identical prose in both): *"If no headerReference for the \[…\] page header is specified
\[…\] the \[…\] page header shall be inherited from the previous section or, if this is the first
section in the document, a new blank header shall be created."* `w:evenAndOddHeaders` is read directly
from `word/settings.xml` (`Document::even_and_odd_headers`) — MJXOFF-136 models the part; this reads
only the one flag.

**`SectionProperties::remove_header_reference`/`remove_footer_reference` and
`ParagraphProperties::section_properties_mut` are new** — MJXOFF-109 built the field and its structural
push/read but not its removal, since resolution (and therefore "replace" and "remove") was this
child's own scope. `section_properties_mut` (the paragraph-level counterpart of `Body`'s own
`section_properties_mut`) exists so removing a reference never fabricates a `w:sectPr` a section did
not already carry.

**`mjx-vml` is a plain dependency of `mjx-docx` now, ungated** — unlike `mjx-pptx`'s `vml` feature
flag, which exists only to spare PresentationML callers a dependency they may never touch; Word headers
are the primary place VML watermarks and text boxes still appear in the wild.
`Document::header_footer_vml_drawings` resolves a header or footer's `mc:AlternateContent` via
`mjx-mce` (non-mutating) and reads every surviving `w:pict` through `mjx_vml::Drawing` — the first
consumer of MJXOFF-58's model outside PowerPoint.

**Two committed fixtures, both authored through this crate's own public API**
(`Document::blank`/`create_header`/`create_footer`/`edit_header_footer` — never a template):
`header_footer_variants.docx` (two sections; section 1 states all three header and footer variants
with `w:titlePg` absent, section 2 states none at all) and `header_watermark.docx` (one header holding
real, hand-authored `mc:AlternateContent`/`w:pict` VML — the one literal XML fragment in the change,
since this crate has no VML-authoring surface). Mutation-proved: neutralising the `w:titlePg` check,
the `w:evenAndOddHeaders` check, or the previous-section inheritance walk each turns a distinct set of
`crates/mjx-docx/tests/headers.rs` tests red; restored by re-editing.

Fixes a stale `document/mod.rs` module doc that still listed `styles.rs`, `numbering.rs`,
`effective.rs` and `sections.rs` among files "later children are expected to add" — all four already
existed.

## [0.0.89] - 2026-09-04

Word sections (MJXOFF-109, Phase C position 10): `w:sectPr`, page setup, columns, section breaks,
line numbering, header/footer references and `w:printerSettings` — `crates/mjx-docx/src/document/sections.rs`.

**A section's properties live at the END of the range they govern, not the start.** A `w:sectPr`
inside a paragraph's `w:pPr` ends a section *at* that paragraph; the body-level one is always the
document's last section. `SectionProperties::sections` (via the new `sections_in`) walks a body's
paragraphs and returns `SectionSpan`s accordingly. **A single-section fixture cannot catch a reader
that only ever looks at the body-level `w:sectPr`** — `tests/fixtures/three_section_document.docx`
is authored specifically to: section 1 (paragraphs 0–1, landscape A4), section 2 (paragraphs 2–3,
portrait A4, two equal-width columns), section 3 (paragraph 4, the body-level `w:sectPr`, portrait
A4, one column). Mutation-proved: neutralising the paragraph-level scan in `sections_in` turns five
tests red, including the mutation-gate test itself (`paragraph_to_section_assignment_is_correct_on_the_three_section_fixture`,
`left: 0, right: 1`); restored by re-editing.

**All 19 of `EG_SectPrContents` and `EG_HdrFtrReferences`** are modelled on `SectionProperties`:
`w:type`, `w:pgSz`/`w:pgMar` (bridged to the shared `PageSize`/`PageMargins` value types — see
below), `w:paperSrc`, `w:pgBorders` (reusing MJXOFF-94's `Border` model for `w:top`/`w:left`/
`w:bottom`/`w:right` via a `xsd:extension` — `Border::extension_attributes[_mut]`, a small
crate-visible escape hatch, rather than a fourth copy of `CT_Border`'s nine attributes),
`w:lnNumType`, `w:pgNumType`, `w:cols`, `w:formProt`/`w:noEndnote`/`w:titlePg`/`w:bidi`/`w:rtlGutter`
(reusing `Toggle`), `w:vAlign`, `w:textDirection` (reusing `ParagraphTextFlowDirection` directly —
`CT_TextDirection` is the identical type under the identical local name at both `w:pPr` and
`w:sectPr`), `w:docGrid`, `w:printerSettings` (reusing `RelationshipReference`), `w:headerReference`/
`w:footerReference` (the flag and the field are modelled here; *which* header/footer applies is
MJXOFF-113's), and `w:sectPrChange`/`w:footnotePr`/`w:endnotePr` (structure only, kept opaque —
MJXOFF-126/MJXOFF-124 own their semantics).

**`w:equalWidth="true"` wins over an explicit `w:col` list, confirmed against ECMA-376 Part 1
§17.6.4's own prose** ("If `equalWidth` is true, then the columns are defined using the data stored
as attributes of the `cols` element … If `equalWidth` is false, then the columns are defined using
the presence and data on each child `col` element", with a worked example describing the `w:col`
children as "ignored" once `equalWidth="1"`). `Columns` does not resolve this itself (no page-margin
knowledge to compute a width from) — it exposes `is_equal_width` and the explicit `columns()` list
independently, with the ruling written down once in `sections.rs`'s own module doc.

**`w:pgMar/w:header`/`w:footer` are measured from the page edge, not the text body** — confirmed
directly against ECMA-376 Part 1 §17.6.11 ("`header` … Specifies the distance … from the top edge of
the page to the top edge of the header"; "`footer` … from the bottom edge of the page to the bottom
edge of the footer"), restated on `PageMargins`'s own field docs.

**`PageOrientation` de-duplicated** (see Breaking changes): the public API now exposes exactly one
orientation type, the generated `mjx_ooxml_types::wordprocessingml::PageOrientation`, re-exported
from `mjx_docx::page`.

**`blank.rs`'s hand-written minimal `w:sectPr` is replaced by the real modelled writer.** The outer
skeleton (`<w:document>`/`<w:body>`/`<w:p/>`) is still a hand-written template — matching
`mjx_pptx::blank`'s own established convention for a part built from nothing — but the `w:sectPr`
fragment itself now comes from `SectionProperties::new` + its own setters, serialized on its own and
spliced in as bytes, never hand-formatted. Fixing this surfaced a real, previously-latent gap: a
`Document::blank`-authored document never declared `xmlns:r`, so any `r:`-prefixed attribute this
child's own new functionality can now write (`w:printerSettings@r:id`, `w:pgBorders`' corner
relationships, `w:headerReference`/`footerReference@r:id`) would have produced namespace-unbound,
invalid XML. `blank.rs` now declares `xmlns:r` alongside `xmlns:w` on the root, matching every real
Word/LibreOffice-authored document (`tests/fixtures/sample.docx` included).

**`w:printerSettings` never rewrites the binary part it references.** Proved on
`tests/fixtures/printer_settings_reference.docx` (authored — no fixture in the corpus carried a
Printer Settings part): editing an unrelated field of the *same* `w:sectPr` that carries
`w:printerSettings` leaves the referenced part's bytes and the relationship's id/target byte-identical.

**Splitting a document into a new section places the new `w:sectPr` inside the terminating
paragraph's own `w:pPr`, never appended to the body** — `Document::edit_section_properties` (get-or-
insert, unifying "change an existing section" and "create a new one") and
`Document::remove_section_properties`, both addressed by the new `SectionLocation` enum.

## [0.0.88] - 2026-09-04

Word effective-properties ladder (MJXOFF-106, Phase C position 9): `Document::effective_run_properties`
and `Document::effective_paragraph_properties` — every `EG_RPrBase` (38 fields) / `CT_PPrBase`
(32 fields) member resolved across `w:docDefaults` → the numbering level → the paragraph-style
`w:basedOn` chain → the character-style chain → direct formatting, with colours baked to concrete
`RRGGBB` through `mjx-dml`'s own theme model.

**The ladder order the ticket stated was wrong, verified against ECMA-376 Part 1 §17.7.2's own
prose, not assumed.** The ticket ordered the paragraph-style chain above the numbering level;
§17.7.2 states the opposite ("First, the document defaults … Next, … numbered item and paragraph
properties are applied … Next, paragraph and run properties are applied … as defined by the
paragraph style"). `tests/effective.rs`'s discriminating fixture (`w:sz` set to three different
values at docDefaults/numbering/the paragraph-style chain, moved one rung at a time across three
paragraphs) is built to fail under the ticket's own order; mutating the merge fold to that order
turns two tests red, pasted in the PR.

**Toggle properties combine by XOR across ladder tiers, and only twelve of them (ECMA-376 Part 1
§17.7.3), not every `CT_OnOff`-shaped member.** A run whose paragraph style and character style both
state `w:b="true"` renders **not bold** — `true XOR true = false` — the opposite of what a naive
override-based resolver (the same rule every other field correctly uses) would answer. Proved by
mutation: replacing the twelve-field XOR recombination with plain fallback turns the cancellation
test red.

**Theme colour and theme font resolve through `mjx-dml`'s own theme model — no second one.** Word's
`ST_ThemeColor` (17 wire tokens, including the `background1`/`text1`/`background2`/`text2` aliases)
maps onto DrawingML's `a:schemeClr` vocabulary; the `bg1`/`tx1`/`bg2`/`tx2` half of that mapping
reuses `mjx_dml::ColorMap::identity` directly rather than restating it, since its own default
pairing (`bg1→lt1`, `tx1→dk1`, …) is exactly what ECMA-376 Part 1 §17.15.1.20 states for Word's
`w:clrSchemeMapping` when absent (true of every fixture in this workspace — `word/settings.xml` is
not modelled by any child yet). `w:rFonts`'s theme attributes resolve the same way against the font
scheme's major/minor × Latin/East-Asian/complex-script slots.

**Cache design:** a chain, once resolved, is reused for every field of one `effective_*` call rather
than re-walked per field — `ChainCache`, memoized by `styleId`, scoped to a single call. It does not
survive across separate calls (`mjx_ooxml_core::Interner` is not `Clone`, and every `Document`
accessor already re-parses its part fresh); the guide states the caller-side alternative for a loop
over many runs.

**A gap found and fixed while wiring the ladder:** `StyleParagraphProperties` (MJXOFF-101) modelled
`w:spacing`/`w:ind` structurally (both round-tripped) but exposed no `spacing()`/`indentation()`
accessor at all — a caller could not read or write a style's own spacing/indentation. Added with the
same `value_property!` macro every sibling accessor already uses.

`crates/mjx-docx/docs/effective_properties.md` is wired into a real doctest gate the way
`mjx-pptx`'s own page is — `src/effective_properties.rs` is `#![doc = include_str!(...)]` with no
items of its own, so the guide's snippets are compiled by `cargo test --doc`, not merely present;
proved by breaking one assertion and watching the doctest go red before restoring it.

## [0.0.87] - 2026-09-04

Word numbering definitions (MJXOFF-104, Phase C position 8): `word/numbering.xml` in full —
abstract numbering definitions (`w:abstractNum`, up to nine `w:lvl` each), numbering instances
(`w:num`), per-instance level overrides (`w:lvlOverride`), picture bullets (`w:numPicBullet`), and
the two-hop resolution from a paragraph's `w:numPr` to the level it actually uses.

**Two-hop resolution, indexed by real key, never by position.** `w:numPr/w:numId` names a `w:num`;
that instance's own `w:abstractNumId` names a `w:abstractNum`; the abstract definition holds the
levels. `NumberingIndex` (built once from a `&Numbering` snapshot, the same design
`StyleIndex`/MJXOFF-101 already uses) indexes both hops by `numId`/`abstractNumId`, never by
document-order position — `numId` values need not be contiguous or ascending, and two instances may
share one abstract definition. `tests/fixtures/numbering_definitions.docx`, authored for this child
(no fixture in the corpus carried `word/numbering.xml` at all), seeds exactly that trap: `numId` 2
and 5 share one abstract definition, deliberately out of order against `numId` 9, and only `numId` 2
carries a `w:lvlOverride/w:startOverride`. Mutation-proved: neutralising the override handling turns
`numId` 5's own (un-overridden) resolved start wrong too, confirmed red and restored by re-editing.

**`numId = 0` is "no numbering", not a lookup failure; a genuinely dangling `numId` is a typed
error.** Proved against the real, already-committed `tests/fixtures/paragraph_properties.docx`
(MJXOFF-96), which carries a real `w:numPr` (`numId` 5) while relating to no `word/numbering.xml` at
all — not only against a synthetic case.

**`w:numStyleLink` resolves through `StyleIndex` — the seam between two OPC parts.**
`Document::resolve_numbering` follows the redirect (a numbering-type style's own `w:pPr/w:numPr`
substitutes for the numStyleLink-carrying definition's own, typically empty, level list), one part
parse at a time since each OPC part carries its own `Interner` and two cannot be held open on the
same `Package` at once; bounded (`MAX_NUM_STYLE_LINK_DEPTH`), the same design
`MAX_BASED_ON_CHAIN_DEPTH` already uses for `w:basedOn`.

**Displayed list numbers are not computed** — deliberately. Turning a resolved level into "1.2.3"
or a bullet glyph requires counting every preceding paragraph in the same list, `w:lvlRestart`,
restart on entering a higher level, and continuation across sections; a counter correct only for a
flat single-level list would be actively misleading. `numbering.rs`'s own module doc states the
boundary explicitly, in the style the PowerPoint effective-properties page already uses for its own
deliberate absences. Rendering a list's text remains MJXOFF-106's.

**Two ticket corrections, verified directly against `wml.xsd`:** `CT_NumRestart` and
`CT_TrackChangeNumbering` are both unreachable from `CT_Numbering` — the former is
footnote/endnote restart (`EG_FtnEdnNumProps`, MJXOFF-124's scope), the latter is `CT_NumPr`'s own
tracked-change wrapper (already opaque, MJXOFF-96) and `CT_FldChar`'s. Neither belongs to this
child.

`CT_Lvl`'s own `w:pPr`/`w:rPr` are `CT_PPrGeneral`/`CT_RPr` — confirmed against the schema, not
assumed — so `NumberingLevel` reuses `StyleParagraphProperties` (MJXOFF-101) and `RunProperties`
(MJXOFF-94) directly rather than restating either. Picture bullets preserve whichever payload a
real file carries (`w:pict` legacy VML, the common case, or `w:drawing`) as opaque, pending
MJXOFF-113/MJXOFF-131's own typed models. `Document::{numbering, edit_numbering,
attach_paragraph_to_list, detach_paragraph_from_list}` mirror the `styles.xml` authoring surface,
including creating `word/numbering.xml` — relationship and content type — on first use.

## [0.0.86] - 2026-09-04

Word style definitions (MJXOFF-101, Phase C position 7): `word/styles.xml` in full —
`w:docDefaults`, every `CT_Style` member, `w:basedOn` chain resolution with cycle safety, and
`w:latentStyles`.

**`CT_Style/w:pPr` is `CT_PPrGeneral`, not `CT_PPr`.** Verified directly against `wml.xsd`, not
assumed from the ticket's own text (which named `CT_PPrGeneral` as already built — it was not):
`CT_PPr` (a live paragraph's own `w:pPr`, MJXOFF-96) is `CT_PPrBase` plus `rPr` (`CT_ParaRPr`),
`sectPr` and `pPrChange`; `CT_PPrGeneral` — what a style definition, `w:pPrDefault` and
`w:tblStylePr` all actually carry — is `CT_PPrBase` plus `pPrChange` only. A style's own paragraph
properties may not carry a pilcrow's run properties or a section break, so `StyleParagraphProperties`
is its own container rather than `ParagraphProperties` reused; every one of its 33 leaf types
(`Toggle`, `FrameProperties`, `Spacing`, `ParagraphBorders`, …) is still the exact struct
MJXOFF-96 built, reused directly — only the wiring is new. `CT_Style/w:rPr`, by contrast, genuinely
is plain `CT_RPr` and reuses `RunProperties` with no wrapper at all. `w:tblPr`/`w:trPr`/`w:tcPr`
(on both `CT_Style` and `CT_TblStylePr`) stay opaque, the same treatment `w:pPrChange` already
gets — no shipped crate models table properties yet, and inventing a first model of them here would
be scope this child was not given.

**Cycle safety is a bounded depth, not a visited-set — and hitting the bound is a typed error,
never a silently truncated chain.** `StyleIndex::based_on_chain` walks `w:basedOn` from a style
upward, accumulating each ancestor into the `Vec` it must return anyway; that accumulation *is*
the bound (`MAX_BASED_ON_CHAIN_DEPTH = 64`) — a chain that has not terminated by then returns
`Err(DocxError::BasedOnChainTooDeep)`, never a partial `Ok` chain a later caller could resolve
properties against without anything going red. Proved by mutation: turning the bound check into a
silent `break` (an `Ok` chain of 64 repeated entries instead of an error) turns both cycle tests red.
`sample.docx`'s `Normal` style does **not** self-reference — checked directly against the fixture's
own bytes by two independent methods, refuting an earlier dispatch brief's claim — so the corpus has
no cycle to test against; `tests/fixtures/style_based_on_cycle.docx` (a self-reference and a mutual
pair) is the only cycle evidence in the suite, and `based_on_chain` is separately exercised against
`sample.docx`'s own real, non-cyclic chains to prove the depth cap never false-positives.

**The three-deep `basedOn` trap, closed with a discriminating fixture:** `Base → Middle → Leaf`,
where `Middle` overrides `Base`'s font size and `Leaf` overrides nothing, so `Leaf`'s correct
effective font size can only come from walking to `Middle` — reading only the leaf, only the base,
or only direct properties each gives a different wrong answer. Mutation-proved: neutralising the
chain walk (stop after the first push) turns this test red.

`w:styleId` matching is case-sensitive; `w:name` matching is case-insensitive (full Unicode case
fold), matching Word's own "apply style by name" UI — `sample.docx` already shows two producers
disagreeing on capitalisation (`PreformattedText` vs. `"Preformatted Text"`). `w:link` resolves in
both directions through `LinkedStyleResolution`, reporting a missing or wrong-kind target as a
value, never a panic.

`w:count` on `w:latentStyles` is preserved, never silently recomputed — `LatentStyles::sync_count`
is the explicit, opt-in way to keep it consistent with the exception list after an edit.
`tests/fixtures/style_latent_styles.docx` is the only committed coverage: `sample.docx` carries no
`w:latentStyles` at all (checked directly).

`Document::edit_style_sheet` creates `word/styles.xml` — content-type registration and the
`styles` relationship from the main document part — on first use for a document that has none (a
[`Document::blank`] document, among others), then runs the same parse/mutate/write-back shape
every other typed edit in this crate uses; `Document::style_sheet` is its read-only, closure-based
counterpart.

### Added

- **`mjx_docx::{StyleSheet, StyleDefinition, DocumentDefaults, DefaultRunProperties,
  DefaultParagraphProperties, LatentStyles, LatentStyleException, StyleParagraphProperties,
  TableStyleOverride, StyleString, RevisionSaveId}`** and their content enums — the full
  `word/styles.xml` model (`CT_Styles`, `CT_Style`, `CT_DocDefaults`, `CT_RPrDefault`,
  `CT_PPrDefault`, `CT_LatentStyles`, `CT_LsdException`, `CT_PPrGeneral`, `CT_TblStylePr`).
- **`mjx_docx::{StyleIndex, LinkedStyleResolution, MAX_BASED_ON_CHAIN_DEPTH}`** — the style index
  (built once from a `&StyleSheet` snapshot, reused for every lookup), `w:basedOn` chain walking,
  and `w:link` resolution.
- **`mjx_docx::Document::{style_sheet, edit_style_sheet}`** — reading and authoring
  `word/styles.xml`, creating it (with its relationship and content type) on first use.
- **`mjx_docx::DocxError::{UnknownStyleId, BasedOnChainTooDeep}`**.
- New fixtures: `tests/fixtures/style_based_on_chain.docx`, `style_based_on_cycle.docx`,
  `style_latent_styles.docx` — authored for this child; `sample.docx` supplies neither a
  three-deep override chain, a `basedOn` cycle, nor `w:latentStyles`.
- **`mjx_ooxml_types::child_order::{PARAGRAPH_PROPERTIES_GENERAL, DOCUMENT_DEFAULTS,
  DEFAULT_RUN_PROPERTIES, DEFAULT_PARAGRAPH_PROPERTIES, LATENT_STYLES, STYLE_DEFINITION, STYLES,
  TABLE_STYLE_OVERRIDE}`** — generated child-order tables for `CT_PPrGeneral`, `CT_DocDefaults`,
  `CT_RPrDefault`, `CT_PPrDefault`, `CT_LatentStyles`, `CT_Style`, `CT_Styles` and `CT_TblStylePr`
  (`xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`).

## [0.0.85] - 2026-09-04

Word document authoring from nothing (MJXOFF-98, Phase C position 6): `Document::blank` and
`Document::blank_with_properties`, mirroring `mjx_pptx::Presentation::blank`'s shape — one call, no
file, no template. On top of `mjx_opc::Package::empty` (the same OPC primitives `mjx-pptx`'s own
`blank.rs` uses), this writes `word/document.xml` (one empty paragraph and a body-level `w:sectPr`
naming the caller's page) plus `docProps/core.xml`/`docProps/app.xml` (MJXOFF-149's packaging-layer
decision, restated rather than re-derived). `PageSize`/`PageOrientation` give the caller `a4()` or
`us_letter()`, portrait or `landscape()`, refused with a typed `DocxError::InvalidPageSize` before
any byte is written if this crate's fixed "Normal" margins (1 inch, matching Word's own template)
would leave no printable area.

**Which optional parts a blank document gets, and why the answer differs from PowerPoint's own.**
`tests/fixtures/sample.docx` — LibreOffice's own output — ships ten parts, four beyond
`word/document.xml` and the two `docProps`: `styles.xml`, `fontTable.xml`, `settings.xml` and
`theme/theme1.xml`. None is schema-required (`wml.xsd`'s thirteen part-bearing global elements are
all `minOccurs="0"` from wherever they are reached). `mjx_pptx::blank`'s answer to the same "what
beyond the schema minimum" question is to include the master, layout and theme, because without them
a deck is *structurally* unusable — there is no layout to build a slide from. WordprocessingML has no
such dependency: a paragraph with no `w:pStyle` and a run with no `w:rStyle` are both legal, and every
real Word implementation falls back to a built-in appearance when a document names no style to
inherit from, so `Document::blank`'s body is fully usable through MJXOFF-92's `insert_paragraph` /
`append_run` / `set_run_text` with zero related parts. Writing even a throwaway `docDefaults`-only
`styles.xml` — legal under this ticket's own wording — would be work MJXOFF-101 replaces on day one,
so this module writes none of the four, and `crates/mjx-docx/src/blank.rs`'s module doc names every
inclusion and every deliberate absence.

**A ticket correction, caught by checking `wml.xsd` directly rather than trusting the brief's own
claim:** `w:sectPr`, `w:pgSz` and `w:pgMar` are *not* schema-required either — `CT_Body`'s `sectPr`,
and `pgSz`/`pgMar` inside `EG_SectPrContents`, are all `minOccurs="0"`. All three are included for the
same "not required, but what makes the result usable" reasoning `mjx_pptx::blank` uses for its
placeholders, not because the schema demands them. The one attribute-level claim that genuinely is
`use="required"` — `CT_PageMar`'s seven attributes (`top`, `right`, `bottom`, `left`, `header`,
`footer`, `gutter`), if `w:pgMar` is written at all — is proved by mutation, once per attribute, in
`tests/schema_gate.rs`.

**A second, unrelated defect the schema gate caught while building this:** the first draft of
`document_bytes` wrote `w:pgSz`'s and `w:pgMar`'s attributes with no `w:` prefix (`w="11906"` rather
than `w:w="11906"`) — `wml.xsd` is `attributeFormDefault="qualified"`, the same class of defect
MJXOFF-152 fixed for this crate's typed attribute accessors, this time in a hand-written XML
template rather than a codec. `xmllint` rejected it immediately (`the attribute 'w' is not allowed`),
before it ever reached a test file.

The LibreOffice open canary (`tests/office_open.rs`, mirroring `mjx-pptx`'s own) is implemented and
skips cleanly on a machine with no `soffice` installed; `crates/mjx-docx/examples/blank_document.rs`
and the crate's first guide page (`crates/mjx-docx/src/guide.rs`, `building_a_document`) are the
runnable and prose versions of the same story.

## [0.0.84] - 2026-09-04

Word paragraph properties (MJXOFF-96, Phase C position 5): `w:pPr` (`CT_PPr`) and all 33
`CT_PPrBase` children, plus the paragraph mark's own run properties (`w:pPr/w:rPr`, `CT_ParaRPr`).
`CT_PPrBase` is the other half of Word's direct formatting — the base MJXOFF-101 (styles) and
MJXOFF-109 (numbering levels) both build on.

Two traps this child exists to close: the paragraph-mark run properties are not a run's own — `w:b`
set through `Paragraph::paragraph_mark_properties_or_insert` can never touch a run's `w:rPr`, and
setting the paragraph's justification can never touch the pilcrow's — proved on bytes, not just by
type distinctness. And `w:spacing/@line` is meaningless without `@lineRule` (`auto` means 240ths of a
line, `exact`/`atLeast` mean twips): there is no `Spacing::line` accessor, only
`Spacing::line_spacing`, which always returns both together (`LineSpacing`), demonstrated by a
doctest.

`CT_ParaRPr` reuses `run_properties.rs`'s (MJXOFF-94) 39 `EG_RPrBase` leaf types directly —
`Toggle`, `Fonts`, `Color`, `Border`, `Shading`, … — rather than restating them; only `Toggle::new`
and `HalfPointMeasureValue::new` needed widening from private to `pub(crate)` to make that reuse
possible. `CT_PBdr`'s six borders and `w:pPr/w:shd` likewise reuse `CT_Border`/`CT_Shd`
(`super::run_properties::{Border, Shading}`) rather than defining a second border or shading type.

`CT_Ind`'s logical (`w:start`/`w:end`) and physical (`w:left`/`w:right`) spellings are both preserved
independently — nothing is normalised on write — with `Indentation::leading_edge`/`trailing_edge`
resolving between them when a file carries both: the logical spelling wins, since Annex M records it
as the later, Strict-compatible addition (ECMA-376 Part 1's own prose states no explicit precedence
here, unlike the `…Chars`-supersedes-twips rule it does state).

One correction to the ticket's own text: `w:kinsoku` inside `w:pPr` is `CT_OnOff` (a plain toggle),
not the two-attribute `CT_Kinsoku` complex type named in the ticket's "Complex types" list — that
type belongs to `w:noLineBreaksAfter`/`w:noLineBreaksBefore` in document settings, unrelated to
paragraph properties. `CT_DecimalNumberOrPrecent` (`w:summaryLength`) and `CT_ParaRPrOriginal`
(reachable only through `w:pPrChange`, MJXOFF-126's scope) are likewise not `CT_PPrBase` children and
have no home in this child.

New fixture: `tests/fixtures/paragraph_properties.docx` — the only committed `.docx` carrying
`w:line`, `w:tabs`, `w:ind`, `w:pBdr` or `w:framePr` before this child; its two paragraphs cover all
33 `CT_PPrBase` members between them, including the legacy physical indentation spelling and a
`w:tab` with `val="clear"` (which removes an inherited stop rather than adding one, so is preserved
structurally like any other stop).

Reachability: `Paragraph::properties`/`properties_mut`/`properties_or_insert` reach `w:pPr` from
`Paragraph`'s own public surface — MJXOFF-152 found `CT_R`'s legacy leaf types were correct but
unreachable through `Document`/`Body`/`Paragraph`/`Run`; this child does not repeat that gap for
`w:pPr` itself. `ParagraphProperties::section_properties`/`change` similarly reach `w:sectPr`/
`w:pPrChange` structurally (as `Unmodeled`), ahead of MJXOFF-106/MJXOFF-126 giving them real content.

### Added

- **`mjx_docx::ParagraphProperties`** (`CT_PPr`, `w:pPr`) — all 33 `CT_PPrBase` members plus the
  paragraph mark's own properties, the section this paragraph ends, and the tracked-change wrapper.
  Reached off `Paragraph::properties`/`properties_mut`/`properties_or_insert`.
- **`mjx_docx::ParagraphMarkRunProperties`** (`CT_ParaRPr`, `w:pPr/w:rPr`) — the pilcrow's own
  character formatting, distinct from a run's `w:rPr`.
- **`mjx_docx::{Spacing, LineSpacing, Indentation, FrameProperties, TabStops, TabStop,
  ParagraphBorders, NumberingProperties, ConditionalFormatting, ParagraphStyle, ParagraphAlignment,
  ParagraphTextFlowDirection, VerticalCharacterAlignment, TextBoxTightWrapSetting,
  DecimalNumberValue}`** and their content enums — the leaf and container types `CT_PPrBase`'s 33
  members are built from.
- **`mjx_ooxml_types::child_order::{PARAGRAPH_PROPERTIES, PARAGRAPH_MARK_RUN_PROPERTIES,
  PARAGRAPH_BORDERS, NUMBERING_PROPERTIES}`** — generated child-order tables for `CT_PPr`,
  `CT_ParaRPr`, `CT_PBdr` and `CT_NumPr` (`xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`).

## [0.0.83] - 2026-09-04

Fixes the defect 0.0.82's own changelog reported and left open (MJXOFF-152): `crates/mjx-docx/src/
document/body.rs`'s `Break`/`PositionalTab`/`Symbol`/`ProofingError`/`PermissionRangeStart`/
`PermissionRangeEnd` (MJXOFF-92) declared their attributes with no `prefix`, so every accessor
matched only a bare, unprefixed local name — but `wml.xsd` is `attributeFormDefault="qualified"`,
and real markup writes `w:font`, `w:alignment`, `w:type`, never bare. Every accessor on these six
types returned `None` (or `Missing`, for a required attribute) against a file that plainly carries
the value — confirmed against `run_content.docx`'s own `<w:sym w:font="Wingdings" w:char="F0E0"/>`
and `<w:ptab w:alignment="right" w:relativeTo="margin" w:leader="dot"/>`, committed since MJXOFF-92
and never once read correctly. Round-trip fidelity was unaffected throughout — the attribute vector
is retained and re-emitted verbatim regardless, which is why every byte-identity suite and the
schema gate stayed green; only the typed reads were broken, and nothing exercised them.

Audited every `#[xml(attribute(…))]` declaration in the file against `wml.xsd` by hand: 17 of 19
needed `prefix = "w"` added; the other two were already correct (`CT_Text`'s `xml:space`, prefix
`xml`; `CT_Rel`'s `id`, prefix `r` — a relationship reference into a different namespace's own
schema, not `wml`'s). **Do not blanket-add `w`** applied literally: those two stayed untouched.

Second, unrelated defect caught by the same audit: `body.rs`'s two local `AttributeCodec` tag types
(`WhitespacePreservation`, `ShortHex`) were private. That compiles inside the crate — same-module
visibility hides it — but `Text::preserve_whitespace` and `Symbol::character` name the private type
in their return type via `AttributeCodec::Value`, which is a hard compile error for any caller
outside this crate. The same class MJXOFF-94 found and fixed for `run_properties.rs`'s own seven
tag types the release before this one. Made both `pub` and re-exported.

A workspace-wide audit (MJXOFF-152's own scope, not just `mjx-docx`) confirmed `wml.xsd` and
`shared-math.xsd` are the *only* two of the schemas this project models that declare
`attributeFormDefault="qualified"`; `sml.xsd`, `pml.xsd` and `dml-main.xsd` declare no
`attributeFormDefault` at all (XSD's default is `unqualified`), and `dml-chart.xsd`,
`dml-diagram.xsd` and `vml-main.xsd` say `unqualified` explicitly. `mjx-dml`'s 318 attribute
declarations (5 of them correctly `prefix = "r"` for relationship references, the rest correctly
unprefixed) confirm the unqualified reading in practice; `mjx-chart`, `mjx-vml` and `mjx-pptx`
declare no typed attribute accessors yet, so there was nothing there to audit. **`shared-math.xsd`
being qualified is a live warning for MJXOFF-134** (`mjx-omml`, not yet written): its leaf types will
need the same `prefix = "w"`-style treatment `wml.xsd` needed here, from the first declaration,
recorded on that ticket.

### Fixed

- **`mjx_docx::{Break, PositionalTab, Symbol, ProofingError, PermissionRangeStart,
  PermissionRangeEnd}`** — every attribute accessor now reads the value real, `w:`-prefixed markup
  states, proved against `run_content.docx` (already committed) and a new
  `tests/fixtures/leaf_attributes.docx` (for the three elements — `w:br` with real values,
  `w:proofErr`, `w:permStart`/`w:permEnd` — neither existing fixture carries with attribute values
  set, so neither could discriminate this defect).
- **`mjx_docx::{WhitespacePreservation, ShortHex}`** — made `pub` and re-exported; both were private,
  which made `Text::preserve_whitespace` and `Symbol::character` uncallable (a compile error) from
  outside this crate.

## [0.0.82] - 2026-09-04

Run properties (MJXOFF-94): `w:rPr` and the character-formatting vocabulary — `EG_RPrBase`'s **39
members** (the ticket said 38 plus `oMath`; the schema gives the group exactly 39, and `oMath` is
`CT_OnOff`-shaped like nineteen of its siblings, not a fortieth special case). `EG_RPrBase` is the
most-referenced group in `wml.xsd`: `CT_RPr`, `CT_ParaRPr`, `CT_RPrOriginal`, `CT_ParaRPrOriginal`,
`CT_Style` and `CT_RPrDefault` all build on it, so MJXOFF-96, MJXOFF-101, MJXOFF-104, MJXOFF-119 and
MJXOFF-126 all needed this landed first.

### Added

- **`mjx_docx::RunProperties`** (`CT_RPr`), reached off `Run::run_properties`/`run_properties_mut`/
  `run_properties_or_insert` — the last placing a freshly authored `w:rPr` at its schema rank via the
  generated `wml` child-order table.
- **`mjx_docx::Toggle`** — the twenty `CT_OnOff`-shaped members (`b`, `bCs`, `caps`, `cs`, `dstrike`,
  `emboss`, `i`, `iCs`, `imprint`, `noProof`, `oMath`, `outline`, `rtl`, `shadow`, `smallCaps`,
  `snapToGrid`, `specVanish`, `strike`, `vanish`, `webHidden`) share one type, reused exactly as
  `mjx_docx::Text` is reused across four `EG_RunInnerContent` members. `val` is declared with the
  attribute grammar's `default = true` — ECMA-376 Part 1's own prose for every one of these elements
  ("if this element is present without a val attribute, its default value is true") — so
  `RunProperties`'s twenty per-property accessors (`bold`, `italic`, …) return `Option<bool>`: `None`
  for the element absent, `Some(true)`/`Some(false)` for present-and-on/present-and-off, never
  collapsed to a bare `bool`.
- **The other eighteen complex types**: `CharacterStyle`, `Fonts`, `Color`, `Underline`, `TextEffect`,
  `Border`, `Shading`, `VerticalAlignment`, `ManualRunWidth`, `Emphasis`, `Languages`,
  `EastAsianLayout`, `Highlight`, and three measure-value wrappers (`HalfPointMeasureValue`, reused
  across `sz`/`szCs`/`kern`; `SignedHalfPointMeasureValue` for `position`;
  `SignedTwipsMeasureValue` for `spacing`) and `TextScaleValue` for `w`. Colour (`Color`,
  `Underline`'s and `Border`'s and `Shading`'s own colour attributes) is Word's own four-attribute
  model (`val`, `themeColor`, `themeTint`, `themeShade`) — not DrawingML's `a:schemeClr` with child
  transforms.
- **`tests/fixtures/run_properties.docx`** — the three `w:rPr` emptiness states `sample.docx` and
  `run_content.docx` don't between them cover (self-closed, absent, and a separate end tag with no
  children), and a run carrying all 39 properties at once, including `w:b w:val="0"` (explicit off,
  distinct from absent), `w:rFonts` with only a hint and no font name, and `w:u` with `color` and
  `themeColor` alongside `val`.

### Fixed

- Caught while writing this child's own tests: `wml.xsd` is `attributeFormDefault="qualified"`, so
  every WordprocessingML attribute is written `w:val`, not `val` — but nothing in this new
  vocabulary's attribute declarations named a `prefix`, so every single one matched only the
  unprefixed spelling and silently fell through to its schema default. `crates/mjx-docx/src/document/
  body.rs`'s pre-existing `Break`/`PositionalTab`/`Symbol`/`ProofingError`/`PermissionRangeStart`/
  `PermissionRangeEnd` (MJXOFF-92) carry the same latent defect on their own attributes, untested for
  the same reason: their round-trip tests pass the whole attribute vector through verbatim and never
  call the typed accessors. Not fixed here — out of this child's scope — and reported on the ticket.

## [0.0.81] - 2026-09-04

The WordprocessingML block content model (MJXOFF-92): `mjx-docx` could open a `.docx` and name its
parts (MJXOFF-90) but could not read a single word of one. This gives it `w:body`'s block content —
paragraphs, runs, text and the rest of `EG_RunInnerContent`'s 33 members — the content spine every
later Word child hangs off.

### Added

- **`mjx_docx::{Body, Paragraph, Run, Text, Hyperlink}`** and the two content enums that hold them
  together — `BlockContent` (`EG_ContentBlockContent`, plus `w:sectPr`) and `ParagraphContent`
  (`EG_PContent`). `Paragraph`/`Run`/`Hyperlink` are typed for real reach — a `w:hyperlink`'s own
  runs stay reachable; `w:customXml`/`w:smartTag`/`w:sdt`/`w:dir`/`w:bdo`/`w:tbl` stay
  `mjx_docx::Unmodeled` (opaque, unowned) until a later child claims one.
- **`mjx_docx::RunInnerContent`** — all 33 `EG_RunInnerContent` members, every one with a variant now
  (`Break`, `Text` reused for `t`/`delText`/`instrText`/`delInstrText`, `RelationshipReference`,
  `Symbol`, `PositionalTab`, `PhoneticGuide` fully typed; the sixteen `CT_Empty`-based members and
  seven later-child payloads — `w:fldChar` (MJXOFF-121), `w:object`/`w:pict`/`w:drawing`
  (MJXOFF-131), `w:footnoteReference`/`w:endnoteReference`/`w:commentReference` — stay `Unmodeled`).
  Adding a variant later would be a breaking change to an enum fifteen children depend on; a variant
  whose payload is still `Unmodeled` is not.
- **`mjx_docx::{BlockPath, RunPath}`** — the address of a paragraph and of a run, in
  `crates/mjx-docx/src/address.rs`, mirroring `mjx_pptx::ShapePath`'s manners (a bare index for the
  common case, an array/slice/`Vec` to descend a level) for WordprocessingML's own kind of nesting —
  block containers for paragraphs, run containers (`w:hyperlink`, so far) for runs — rather than
  `p:grpSp` groups.
- **`Document::{paragraph_count, run_count, paragraph_text, run_text, set_run_text, insert_paragraph,
  append_paragraph, remove_paragraph, insert_run, append_run, remove_run}`** — reading and editing
  paragraphs and runs, each edit going through `ToXml::write_back` so only the touched subtree
  re-serializes.
- **The `xml:space` rule** for `w:t` (`Text::set_text`): writes `xml:space="preserve"` when the new
  text starts or ends with ASCII whitespace, and removes the attribute otherwise — reading never
  trims, regardless.
- **`tests/fixtures/run_content.docx`** — a fixture carrying `w:br`, `w:tab`, `w:sym`, `w:cr`,
  `w:noBreakHyphen`, `w:ptab`, `w:ruby`, a `w:t` with `xml:space="preserve"`, a `w:hyperlink`
  wrapping two runs, and a `w:fldChar` (a run-inner element whose payload is still `Unmodeled`),
  swept automatically into every byte-identity suite and the schema gate.

### Also in this release: the Word crate spine (MJXOFF-90)

MJXOFF-90 shipped without a version bump of its own, so its work reaches a release here rather than
in a `0.0.81` of its own. Recorded rather than renumbered — the history is linear and a rewrite
would cost more than the misfiled heading does.

- **`mjx_docx::{Document, PartKind, DocumentParts, DocxError}`** — `Document::open`/`save`/
  `save_unchecked`/`validate`, mirroring `Presentation`'s names so the Word method is guessable from
  the deck one, and the part graph over `wml.xsd`'s fourteen global elements. `crates/mjx-docx` was
  thirteen lines and zero public items before it.
- **`xtask`'s child-order generator resolves `xsd:complexContent` and `xsd:simpleContent`.** An
  extension splices the resolved base chain *before* the derived type's own particle; a restriction
  replaces it; `simpleContent` contributes nothing. This is why `wml` can have an ordering table at
  all — its schema uses `complexContent` in 41 derived types — and it is what unblocks MJXOFF-132
  (`sml`, 6 `simpleContent`) and MJXOFF-134 (`shared-math`, 2) without either repeating the work.
- **The `wml` child-order table**, generated and committed, with the ordering audit proved red then
  green on real WordprocessingML.

## [0.0.80] - 2026-09-04

Document properties (MJXOFF-149): the programme held two contradictory positions on `docProps/*` —
"deliberately absent" in `mjx-pptx`'s own blank-deck module doc, and already assumed in two Word/Excel
tickets' part lists. Settled in favour of authoring: every file real Office writes carries
`docProps/core.xml` and `docProps/app.xml`, and `mjx-schema-gate`'s three-category rule was written
anticipating exactly this flip.

### Added

- **`mjx_opc::doc_props`** — `CoreProperties` (`title`, `creator`, `created`, `modified`),
  `ExtendedProperties` (`application`) and `DocumentTimestamp` (built only from explicit calendar
  fields — there is no `now()`), plus the writer, part-name, content-type and relationship-type
  constants for `docProps/core.xml` (ECMA-376 Part 2's `opc-coreProperties.xsd`, Dublin Core) and
  `docProps/app.xml` (`shared-documentPropertiesExtended.xsd`). Packaging-layer, so `mjx-pptx` and
  the Word/Excel `blank()` constructors still to come share one implementation.
- **`mjx_pptx::Presentation::blank_with_properties`** — `blank` with document properties set, rather
  than left absent. `blank` itself now writes both parts on every call, all-`None` by default (a
  schema-valid, childless part, since both are `xs:all` groups with every child optional).

### Fixed

- `mjx-schema-gate`'s `opc-coreProperties` and `shared-documentPropertiesExtended` namespaces move
  from the preserved-foreign allowlist to the modelled-schema table: `docProps/core.xml` and
  `docProps/app.xml`, in every fixture and every authored deck alike, are now genuinely validated
  against ECMA-376 rather than skipped as foreign markup. `opc-coreProperties.xsd`'s Dublin Core
  imports (`dc:`, `dcterms:`, real network `schemaLocation`s, unlike `wml.xsd`'s bare `xml:` import)
  are resolved through a committed local XML catalog rather than a live fetch.

## [0.0.79] - 2026-09-03

The DrawingML diagram (SmartArt) model (MJXOFF-148): `add_diagram` authored `dgm:` markup this
project neither modelled nor ordered, which is exactly the condition MJXOFF-110 exists to make
impossible. Closes the hole.

### Added

- **`mjx_dml::diagram`** — a typed model of `dml-diagram.xsd`, 50 of its 58 complex types down to
  their attributes: the data part as a point-and-connection graph (`DataModel`, `PointList`/`Point`,
  `ConnectionList`/`Connection`), the layout definition's whole algorithm tree (`LayoutDefinition`,
  `LayoutNode`, `Algorithm`, `Constraint`, `NumericRule`, `Choose`), the quick style
  (`StyleDefinition`/`StyleLabel`) and the colour transform
  (`ColorTransform`/`StyleLabelColors`/`ColorList`). A handful of externally-defined DrawingML
  formatting groups (`spPr`, `style`, `txPr`, `bg`, `whole`, `scene3d`, `sp3d`) and the SmartArt
  gallery-catalog header types this project never authors or reads stay unmodelled, by name and
  reason, in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md`. Running a `dgm:layoutDef` to compute
  where a consumer draws each point remains a documented non-goal — a rendering concern.
- **`mjx_ooxml_types::diagram`** — the whole `ST_*` family of `dml-diagram.xsd` (66 simple types),
  comprehensively named; `dml-diagram` joins `CHILD_ORDER_SCHEMAS`, so an authored diagram's four
  parts are ordered by construction rather than emitted from a fixed template with no writer checking
  its sequence.

### Fixed

- `mjx-schema-gate`'s `dml-diagram` row now validates for real: `add_diagram`'s four parts were
  already checked against `dml-diagram.xsd`, and a new case proves the check is live by writing
  markup the schema rejects and asserting it is caught, naming the schema — not merely that markup
  this project already writes happens to pass.

## [0.0.78] - 2026-09-03

A performance baseline and a large-file corpus generator (MJXOFF-147) — the numbers MJXOFF-95 (the
Excel cell store) designs its memory budget against, and the numbers a later regression is compared
to instead of intuition. Not an optimisation pass: nothing here got faster, the point is knowing.

### Added

- **`cargo run -p xtask -- corpus`** — (re)builds a git-ignored large-file corpus into
  `target/corpus/`: a 300-slide `.pptx` (`mjx_pptx::Presentation`'s real edit surface), a
  20,000-paragraph `.docx` and a 300,000-cell `.xlsx` (raw WordprocessingML/SpreadsheetML on
  `mjx_opc::Package` — neither format has a model yet), and prints size/element/cell counts.
  `corpus --mem <pptx|docx|xlsx>` runs its peak-resident-set checkpoints (open / first-mutation
  materialisation / edit / save) in one process via `/proc/self/status`'s `VmHWM`, the kernel's own
  peak-RSS counter — chosen over a counting allocator because it answers the literal question asked
  ("peak resident set") rather than a proxy for it. Not a substitute for MJXOFF-130's Office-authored
  fixtures, and does not claim to be.

- **Criterion benchmarks** — `crates/{mjx-pptx,mjx-docx,mjx-xlsx}/benches/`, six operations per
  format (`open`, `first_mutation_materialisation`, `edit_after_materialised`, and the three save
  paths `save_untouched` / `save_lightly_edited` / `save_fully_materialized`, measured separately
  because the gap between them is the result), plus a seventh for `mjx-pptx` exercising the real
  `Presentation` edit surface rather than only the lower `Package` layer.

- **`docs/BENCHMARKS.md`** — the baseline: the machine, the (existing, previously undocumented)
  release profile, all four operations × three formats' time and peak RSS, the three save paths
  compared, A7d's `mjx248_measure` reproduced on this machine (matches within ~10–35%, one direction,
  explained by MJXOFF-143), the short-list of figures MJXOFF-95 designs against, and two measurement
  bugs this child caught in its own harness before trusting its numbers.

### Findings, filed rather than fixed here

- Materialising the 610,005-element / 300,000-cell worksheet costs **+274 MiB of peak RSS** over an
  8.54 MiB raw-XML part — roughly 32× the source bytes, ≈ 913 B/cell. Filed as **MJXOFF-151** (under
  MJXOFF-88) for MJXOFF-95 to design against (an arena/columnar layout, per `PLAN.md`'s hybrid model),
  not fixed in this child.

## [0.0.77] - 2026-09-03

The untrusted-input paths are fuzzed, and three defects they were hiding are fixed (MJXOFF-146).

`CLAUDE.md` has always said it: *no `unwrap`/`panic`/`expect` on untrusted input — inputs are
untrusted files.* Nothing in the repository proved it. A grep finds the obvious cases and says
nothing about a recursion depth, a slice index, or an allocation an attacker sizes. This adds a
campaign that tries, and it found three things a grep could not.

### Added

- **`cargo run -p xtask -- fuzz`** — a campaign against the three untrusted-input entry points
  (`mjx_xml::fidelity::parse`, `mjx_opc::Package::open`, `mjx_mce::resolve`) plus the round-trip
  oracle, in five targets. Run **on demand, not on every push**; `--list`, `--target`, `--seed`,
  `--iterations` and `--seconds` select and bound a run, and a seed makes one reproducible.

  It is stable Rust with no new dependency. `cargo-fuzz` needs a nightly toolchain for its sanitizer
  flags, and a gate only some machines can run is not a gate. It lives in `xtask`, which is host-only
  and which nothing depends on, so the harness cannot reach the shipped graph.

  It asserts properties rather than the absence of a crash: every input the reader accepts must
  re-serialize **byte-for-byte**; the same corpus is re-run with the document dirtied at its root,
  where a byte range that does not describe its element shows; a package written back and reopened
  must hold the same part bytes. Panics are caught per execution, a counting global allocator
  measures each execution's peak against a ceiling so unbounded allocation is a *finding* rather than
  an OOM kill, and a watchdog turns a hang into an abort that names its input.

- **`mjx_fixtures::adversarial_xml`** and **`adversarial_xml_dirtied_at_the_root`** — the hostile XML
  corpus, moved out of `crates/mjx-xml/tests/subtree_cow.rs` so the hand-written gate and the
  campaign read the same list instead of drifting apart.

- **`mjx_xml::fidelity::MAXIMUM_DEPTH`** and **`XmlError::DepthLimit`** — see below.

- Regression suites for every finding, in the crate that owns the path:
  `crates/mjx-xml/tests/untrusted_input.rs`, `crates/mjx-opc/tests/untrusted_input.rs`,
  `crates/mjx-mce/tests/untrusted_input.rs`, and the minimised container
  `tests/fixtures/declared_size_lie.zip`.

### Fixed

- **A 140 KB document could abort the process.** The reader is iterative and would build a tree of
  any depth; every walk *over* that tree recurses, because the data does — `Drop` and `Clone` are
  compiler-generated, the serializer descends a dirty element, and `mjx_mce::resolve` descends the
  whole document. `resolve` died first, overflowing the stack at a nesting depth reachable in about
  140 KB of `<a>`. Not a catchable panic: an abort. `fidelity::parse` now refuses to build a tree
  deeper than `MAXIMUM_DEPTH` (256), which bounds every walk downstream, including the ones Phase C
  and D have not written yet. The deepest part in the committed corpus is **13**.

- **A 757-byte container could ask for four gigabytes.** A ZIP entry's uncompressed size is a header
  field, attacker-controlled and checked against the data only after the data has arrived.
  `Package::open` reserved exactly that many bytes per part, so a container declaring 4 GiB for a
  four-byte payload allocated 4 GiB before it could return an error. The speculative reservation is
  now capped at 1 MiB and the buffer grows from bytes that actually arrive. **Nothing about what is
  accepted changed.**

- **`<!DoCTYPE a>` lost a byte and changed case.** The writer wraps a doctype in the constant
  `<!DOCTYPE` … `>`, so a source spelling the keyword any other way could not come back —
  sixteen bytes in, fifteen out. `quick-xml` accepts spellings XML 1.0 §2.8 does not, and the reader
  now refuses a doctype it could not reproduce rather than silently rewriting it.

- **An element name that could not be written back is now refused.** `quick-xml` scans an element
  name up to whitespace, so `<a" b"c="1"/>` produced an element literally named `a"`. Untouched it
  round-tripped; *rewritten*, the writer put that name between `<` and `>` and emitted markup that
  will not parse. Names carrying a byte that would end a name or the tag around it are refused; names
  XML would reject but that re-serialize exactly (a leading digit, say) are still preserved, because
  fidelity is the tie-breaker in both directions.

Every one of these fixes **tightens** what the readers accept. None loosens anything: trading a crash
for a corruption is the one thing this project exists to prevent.

## [0.0.76] - 2026-09-03

The SpreadsheetML vocabulary is generated (MJXOFF-145).

`sml.xsd` is the largest schema in the set — 4,439 lines, 367 complex types, 96 simple types — and
nothing in `mjx-ooxml-types` covered any of it. MJXOFF-132 (`mjx-sml`) is built on this vocabulary,
so without it an Excel crate would have invented its own cell-type and error-value enumerations.
This adds it **whole**, not as an allowlist:

- **`mjx_ooxml_types::spreadsheetml`** — all 96 simple types of `sml.xsd`, carrying all 559
  enumeration values of its named types. Cell types, formula kinds, the 18 conditional-format rule
  kinds and 17 icon sets, the 66 PivotTable filters, the 28 table-style elements, border and
  pattern fills, data-validation kinds and IME modes, the MDX cube vocabulary, and the rest.

Every item documents its original `ST_*` symbol and its exact wire token, and 149 of the values are
named from the ECMA-376 prose rather than from their token — `s` is a shared string and `str` a
formula string, `3TrafficLights1` is `ThreeTrafficLights`, `gray125` is 12.5% grey, and `stdDevp`
is the population standard deviation as against `stdDev`'s sample estimate. The `wire` suite grows
26 → 61 → **94** tests: one per overridden `ST_*` pinning its named variants to exact bytes in both
directions, plus an exhaustive pass over all 559 tokens.

### Fixed

- **The simple-type reader lost a type when one nested another.** `xtask`'s XSD reader closed a
  named `xsd:simpleType` on the first `</xsd:simpleType>` it saw, so an `xsd:union` written with
  inline anonymous members — `sml.xsd`'s `ST_TextRotation`, the only one in the emitted set — closed
  its own definition early, swallowed the type declared after it, and attributed the inner
  restrictions' base and facets to the type around them. The reader now tracks nesting depth.
- **A union of one number is now that number.** `ST_TextRotation` (0–180 degrees, or 255) would
  have been a `String` newtype. A union every member of which resolves to the same Rust primitive
  is emitted as that primitive, the way a plain numeric restriction already was.

### Changed

- `assert_every_token_round_trips!` in the `wire` suite is now
  `assert_every_token_round_trips_to_its_own_variant!`, and asserts that the number of distinct
  variants an enumeration reaches equals the number of values its schema declares — the failure two
  colliding naming-override rows would cause. It covers `wml`, `shared-math` and `sml` alike.

## [0.0.75] - 2026-09-03

The WordprocessingML and Office Math vocabularies are generated (MJXOFF-144).

`mjx-ooxml-types` covered the shared common simple types, a curated slice of `dml-main` and a
curated slice of `pml`. Word and equations had nothing, so MJXOFF-90 (`mjx-docx`) and MJXOFF-134
(`mjx-omml`) would each have invented their own enumerations and the naming convention would have
fractured across two crates at once. This adds both vocabularies **whole**, not as an allowlist:

- **`mjx_ooxml_types::wordprocessingml`** — all 110 simple types of `wml.xsd`, carrying all 733
  enumeration values. Justification, underline kinds, the 193 border styles, shading patterns,
  section breaks, the 63 numbering formats, theme colours, text-flow direction, table-style
  overrides, the glossary-document galleries, and the rest.
- **`mjx_ooxml_types::officemath`** — all 14 simple types of `shared-math.xsd`, carrying all 30
  enumeration values.

Every item documents its original `ST_*` symbol and its exact wire token, and 183 of the values are
named from the ECMA-376 prose rather than from their token — `pct12` is 12.5%, `neCell` is the top
**right** table cell, `ideographZodiac` is the zodiac ideograph format, and `--`/`-+`/`+-` would
otherwise have collapsed onto one identifier. The `wire` suite round-trips all 763 tokens and pins
every one of those 183 names to its exact bytes in both directions.

### The naming tables are now per schema

An `ST_*` symbol is scoped to the schema that declares it, and OOXML reuses symbols: `ST_Jc` is
declared by both `wml.xsd` and `shared-math.xsd`, and `ST_Direction` by both `wml.xsd` (`ltr`/`rtl`)
and `pml.xsd` (`horz`/`vert`, already emitted as `Orientation`). One flat override table keyed on the
bare symbol cannot hold two meanings. `xtask`'s naming data is therefore partitioned the way the
symbols are — one `NameEngine` per emitted module — and the engine in `naming.rs` is unchanged: it
already took its tables by reference. Adding a schema still means growing the tables. The existing
`shared`, `drawingml` and `presentationml` output is byte-identical.

### The generator refuses names that would lose a token

Two `ST_*` types that reach one Rust type name, or two values of one enumeration that reach one Rust
variant, are now hard errors in `xtask` rather than Rust that compiles with a wire token nobody can
write back. So is a naming-override row that matched nothing — a misspelled symbol used to do
nothing at all, silently leaving the mechanical name it was written to replace.

### `COVERAGE.md` reports every schema

The generated manifest listed six schemas of the twenty-six in the Transitional set, and printed
`pending` for `wml`, `sml` and `shared-math` from a hard-coded string — so it would have kept saying
`pending` after the work was done. It now has a row for **every** schema in **both** tables, with
each status derived: the simple-type column from the generator's module table, the child-order column
from `CHILD_ORDER_SCHEMAS`. A pending row names the work item that owns it, and
`mjx-schema-gate`'s `the_declared_owners_agree_with_the_generated_coverage_document` fails if the
document and the gate's `OrderingCoverage::Pending` name different owners. A schema that is in
neither table and has no written reason fails the generator.

No `CHILD_ORDER_SCHEMAS` row was added: those belong to the children that start authoring the markup
(MJXOFF-90 for `wml`, MJXOFF-134 for `shared-math`, MJXOFF-132 for `sml`).

## [0.0.74] - 2026-09-03

A typed model's round trip no longer re-flows the part it came from (MJXOFF-143).

0.0.64 gave every parsed element the byte range it came from, so a serializer copies untouched
subtrees rather than rebuilding them. It stopped at the typed layer, and that left one limitation in
`fidelity_and_gaps.md`: **a model is a view**, so a `from_xml` / `to_xml` pass rebuilds every element
it looked at — including the ones nothing changed — and `*slot = value.to_xml(interner)` throws the
range of each of them away. Three surfaces read a whole part that way (`edit_vml_drawing`,
`edit_chart`, and the table-style list), and a dozen more read a single element that way. Editing one
word of a chart title re-flowed the whole chart.

Both halves of the loss were deliberate design rather than oversight, which is why this needed a
decision rather than a patch. `RawElement`'s `Clone` drops the range because a range means nothing
against another document's buffer, and cloning is how a subtree leaves the document that owns one.
`RawElement::new` records none because a newly authored element has no original.

### The design

**`RawElement::replace_preserving_verbatim_source`**, and `ToXml::write_back` over it. Instead of
assigning the rebuild over the original, the two are walked together in one pass, and a range is
moved onto a rebuilt node **only where that node compares equal to the one it replaces**. Two facts
discharge the burden of proof, and both are structural rather than remembered:

- *The bytes still describe the element.* `RawElement`'s `PartialEq` compares name, self-closing
  style, attributes in order with their quoting, and, recursively, children — precisely the
  properties an element's markup determines. So "equal" **is** "these bytes spell this element".
- *The buffer is the right one.* The range comes from the element being overwritten and lands on its
  replacement at that same position, so the destination document is by construction the one that
  measured it. A caller cannot pair an original from one document with a rebuild bound for another,
  because the original *is* the destination.

The three candidates it was chosen over: a `clone_within_document` on `RawElement` would give ranges
back only to the markup a model does *not* understand — everything it does model is rebuilt after the
clone, so a wrapped `v:shape` would still re-flow — and its soundness ("only while the clone stays in
this document") is a convention no type can check. `FromXml` taking its element by value moves the
content instead of cloning it, but breaks every implementor and every call site while giving the same
partial answer, and the read-only surfaces (`with_chart`, `with_vml_drawing`) hold a shared reference
and could not give ownership at all. A retained element plus a dirty flag reaches everything, but the
flag must be cleared by every mutator in three crates, and one missed mutator writes the wrong bytes
— the single failure mode this design exists to make impossible. Here there is no flag to forget:
cleanliness is *computed*, against the element still sitting in the document.

`mjx-xml`'s writer is unchanged and still checks every range before trusting it — it must fit, open
with `<` plus the element's qualified name, and close the way `empty` says it closes — so a range
that reached it wrongly degrades to a re-flow rather than to wrong bytes. `RawElement` does not grow:
the eight-byte budget test is untouched.

### What changed for callers

- `ToXml` gains a **provided** method, `write_back`, so no implementor changes. Every whole-part and
  sub-element edit surface in `mjx-pptx` now goes through it — 19 call sites.
- `mjx_vml::DrawingPart` keeps the `RawDocument` it parsed instead of scattering its pieces, because
  a standalone part has no document to write back into otherwise. It costs the parsed tree alongside
  the typed model; a caller that already owns the part's `RawDocument` (through
  `mjx_opc::Package::part_tree_mut`) should use `Drawing::from_xml` and `write_back` directly and pay
  nothing.
- The *Limitations* table in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` is gone — it had one
  row and this was it. The row is recorded under "What used to be here", so a reader can tell "gone"
  from "quietly dropped".

`crates/mjx-vml/tests/drawing.rs`'s `attributes_wrapped_across_lines_reflow_when_the_part_is_re_serialized`
asserted the re-flow *happened* and instructed its reader to replace it with byte identity the moment
it stopped. It has stopped. Every new case is written against a fixture whose start tags are wrapped
across lines — `vmlDrawing1.vml`'s with CRLF — because a part that is already on one line
reconstructs to its own bytes and would pass with the mechanism deleted.

Tests 1,676 → 1,690 default and 1,690 → 1,705 with `--all-features`.

## [0.0.73] - 2026-09-03

`mjx-dml`'s composite tiers on the attribute grammar — geometry, tables, text and the colour
resolver (MJXOFF-142).

0.0.72 put the seven shared property tiers on the grammar. This completes the crate: **the 107
remaining `attr_*` call sites across `geometry/`, `table/`, `text/` and `resolve.rs` became 0**, and
with them every `dml_attr`, `prefixed_attr`, `push_*`, `set_attr`, `angle_to_wire`,
`parse_percentage` and `parse_angle`. `crates/mjx-dml/src/build.rs` no longer mentions attributes at
all: what is left there builds and finds *elements*.

**There is one path from a wire attribute to a typed value in `mjx-dml`, and one back**, and both go
through `mjx_xml::attribute::{read, write}`. A helper family with two callers left is the
half-migrated family CI's naming check exists to warn about, so the family is gone rather than
reduced.

### Two shapes, one grammar

The tiers here have the same split 0.0.72 found: some types retain an attribute vector and declare
on themselves; most sites are *value projections* over elements the crate has no type for — an
`a:pt`, an `a:arcTo`, an `a:tab`, an `a:buChar`, an `a:hlinkClick`, a colour transform's `@val` —
and declare on a generic attribute face reached through `AsRef<[RawAttribute]>` / `AsMut<Vec<..>>`.
Twenty-two such faces are declared here.

Two attributes are declared as `Text` rather than as an `Enumeration<T>` *deliberately*:
`a:prstGeom@prst` and `a:cell3D@prstMaterial` each expose **both** readings of the same bytes — the
typed one and the raw token — which is what lets a shape kind or a material this build does not know
still be named. The typed reading layers the generated enumeration's own `from_wire` over the one
read, so the token → enum mapping still has one implementation.

### New

- **`mjx_dml::codec`** gains five: `TextFontSize` (`ST_TextFontSize`), `TextPointSize`
  (`ST_TextPoint`), `TextIndentLevel` (`ST_TextIndentLevelType`, whose `0..=8` range is enforced),
  `PercentageWithPercentSign` (the `111%` spelling `a:buSzPct@val` is written in), and
  `EmuOrGuideName` / `AngleOrGuideName` for the two `ST_Adj*` unions custom geometry places points
  with.
- **`crates/mjx-dml/tests/in_context_roundtrip.rs`** grows from 16 cases to 28: a table lifted out
  of `tables.pptx` and `table_extensions.pptx` and asserted **at the outermost container**, so a
  cell's attributes must survive being rebuilt as part of a row rebuilt as part of a table; a
  paragraph-level body and a preset geometry out of `text_levels.pptx`; a transform read and written
  back; and five hand-written literals in forms this project's writer never emits, for the run
  properties, paragraph properties, table, custom geometry and transform tiers.

### Changed behaviour: a boolean a setter writes is spelled `true`

`TableProperties::set_part` and `TableCell::set_merged` wrote `1`; they now write `true`, the one
canonical `ST_OnOff` spelling every other boolean in the workspace is written in, because they go
through the same `OnOff` codec. Reading is unchanged and still accepts all six spellings, and an
attribute **nobody assigns to keeps its own spelling** — a file that says `firstRow="1"` still says
`firstRow="1"` after an unrelated edit. (`mjx_pptx`'s `add_table` builds a fresh `a:tblPr` from a
literal template and still writes `firstRow="1" bandRow="1"`, as PowerPoint does.)

### Breaking changes

As in 0.0.72: an accessor over a declared attribute reports a malformed value instead of silently
reading `None`, so it returns `Result<Option<T>, AttributeError>` — or `Result<T, AttributeError>`
where the attribute is `use="required"` — and a text-valued one returns a `Cow<str>` (entity
references in the file are decoded) where it returned `&str`.

| Was | Is |
|-----|----|
| `AdjustPoint::{x, y}` → `Option<AdjustCoordinate>` | `Result<AdjustCoordinate, AttributeError>` |
| `Path2D::{width, height, fill, stroke, extrusion_ok}` → `Option<T>` | `Result<Option<T>, _>` |
| `GeometryGuide::{name, formula}` → `Option<&str>` | `Result<Cow<str>, _>` |
| `PresetGeometry::preset_token` → `Option<&str>` | `Result<Cow<str>, _>` |
| `TableColumn::width`, `TableRow::height` → `Option<Emu>` | `Result<Option<Emu>, _>` |
| `TableColumn::set_width`, `TableRow::set_height` took `Emu` | take `Option<Emu>` (`None` removes) |
| `TableCellProperties`'s eight accessors → `Option<T>` | `Result<Option<T>, _>` |
| `TableCellProperties::{set_anchor, set_text_direction, set_horizontal_overflow}` took a value | take an `Option` |
| `TableCell::id`, `TextField::{id, field_type}` → `Option<&str>` | `Result<Option<Cow<str>>, _>` |
| `TableStyleList::default_style_id`, `TableStyle::{style_id, style_name}` → `Option<&str>` | `Result<Cow<str>, _>` |
| `FontReference::index` → `Option<FontCollectionIndex>` | `Result<Option<..>, _>` |
| `Cell3D::preset_material` → `Option<&str>` | `Result<Option<Cow<str>>, _>` |
| `CharacterProperties`'s ten attribute accessors → `Option<T>` | `Result<Option<T>, _>` |
| `CharacterProperties::{hyperlink_rel_id, hyperlink_action}`, `TextRun::hyperlink_rel_id` → `Option<&str>` | `Option<String>` |
| `ParagraphProperties`'s eight attribute accessors → `Option<T>` | `Result<Option<T>, _>` |
| `ResolvedGuides::define` took `&'a str` | takes `impl Into<Cow<'a, str>>` |

`Transform2D::read`, `CustomGeometry`'s spec readers, `TableCell::{column_span, row_span,
merged_horizontally, merged_vertically}`, `TableProperties::part`, `TableStyleTextStyle::{bold,
italic}`, `PresetGeometry::preset`, `Cell3D::material` and every `resolve_*` function keep their
shape: each is a **total** projection with a documented answer for "the file does not say", and a
value this model cannot read is the file not saying. That decision is written down in
`resolve.rs`'s module docs, where it is load-bearing — a renderer that refused to draw a shape over
one malformed colour transform would be worse than one that drew it without the transform.

## [0.0.72] - 2026-09-03

`mjx-dml`'s shared property tiers on the attribute grammar — colour, fill, outline, effects, 3-D,
theme and style (MJXOFF-141).

MJXOFF-140 proved the `#[xml(attribute(..))]` grammar on a synthetic type inside `mjx-derive`'s own
tests. This release is the first time anything shipped uses it: the seven files every other
DrawingML tier reaches through no longer parse an attribute by hand. **86 calls to the `attr_*`
family became 0 in those files**, and the four hand-written measure readers and writers they were
the last users of are deleted.

### There is now exactly one path from a wire attribute to a typed value

`mjx_xml::attribute::read` and `mjx_xml::attribute::write` are that path — find, decode, hand to a
codec; encode, escape for the quote in use, set or remove. Every accessor
`#[derive(XmlAttributes)]` generates is one call to one of them, `mjx-dml`'s remaining `attr_*` /
`push_*` helpers (which the tiers MJXOFF-142 owns still use) are one call to one of them, and a model
reading an element it has no type for calls them directly. Two implementations of "attribute to
value" is the duplicate this workstream exists to prevent; there is one.

### A declaration no longer requires a type that owns its attributes

`#[derive(XmlAttributes)]` reaches the vector through `AsRef<[RawAttribute]>` to read and
`AsMut<Vec<RawAttribute>>` to write, so the `attributes` field may be a `Vec`, a `&[RawAttribute]`
view (getters only — the bound that would give it setters is simply not satisfied), a
`&mut Vec<RawAttribute>` cursor, or generic over all of them.

That last form is what `mjx-dml`'s **value projections** use. An effect, a bevel, a camera, a
gradient stop, a line end and a blip are facts read out of an element the crate does not model as a
type; a conduit generic over its attribute container declares them once and serves both directions —
`{ attributes: &element.attributes }` to read, which copies nothing, and `{ attributes: Vec::new() }`
to write the vector the new element will own.

### New

- **`mjx_dml::codec`** — `EmuCoordinate`, `EmuLineWidth`, `SixtyThousandthsOfADegree` and
  `Percentage`, the four measure codecs. A crate that owns a measure owns its codec.
- **`mjx_ooxml_core::Number<T>`** — an alias for `Enumeration<T>`, so a numeric attribute is declared
  `codec = Number<u32>` rather than claiming an integer is an enumeration.
- **`RawElement::rebuilt`** — the single construction point every `ToXml` now goes through. Identical
  to `new` today; it exists so that carrying a source range through a typed round trip (MJXOFF-143)
  is one edit rather than one per `to_xml` in the workspace.
- **`crates/mjx-dml/tests/in_context_roundtrip.rs`** — the generalised in-context harness (was
  `txbody_roundtrip.rs`), now covering seven types out of real parts plus a corpus of hand-written
  literals in forms this project's writer never emits, and both tier-3 isolation cases.

### Breaking changes

An accessor over a declared attribute reports a malformed value instead of silently reading `None`,
so several `mjx-dml` accessors return `Result<Option<T>, AttributeError>` where they returned
`Option<T>`, and the text-valued ones return a `Cow<str>` (entity references in the file are decoded)
where they returned `&str`. The affected methods are `Color::{value, hex}`,
`LineProperties::{width, cap, compound, pen_alignment}`, `GradientFill::{flip, rot_with_shape}`,
`PatternFill::preset` and `Shape3D::{z, extrusion_height, contour_width, material}`.
`PictureFill::{image_rel_id, image_link_id}` return `Option<String>`: they read through the blip's
attribute face, which does not outlive the call, and both callers copied the id anyway. The value
tiers (`LineSpec`, `Shape3DSpec`, every effect) are unchanged — a spec is a value description and
still drops what it cannot represent.

## [0.0.71] - 2026-09-03

An attribute grammar for `mjx-derive` — accessors over the retained attribute vector, not a lifting
form (MJXOFF-140).

`mjx-derive` modeled elements, children and text; **attributes it did not model at all.** Every typed
type in the workspace reached into its own `Vec<RawAttribute>` and parsed the value by hand.
DrawingML survived that because Phase A grew it a tier at a time, but `wml.xsd` has 110 simple types
and `sml.xsd` 96, both far more attribute-dense than `pml`, and hand-parsing each one across two new
format crates is where the `ST_OnOff` spellings would quietly go wrong.

Nothing shipped changes. No emitted byte moves; this release adds a way to declare what a hand-written
accessor already does, and the additions are new items beside the existing ones.

### `#[derive(XmlAttributes)]`

A third derive, independent of `FromXml` / `ToXml` and composing with them, with a hand-written pair
of impls, or with neither. It asks only for the retained `attributes: Vec<RawAttribute>` field and
generates **one getter and one setter per declared attribute** over that vector:

```rust
#[derive(FromXml, ToXml, XmlAttributes)]
#[xml(attribute(local = "val", codec = HexColorRgb, accessor = color, required))]
#[xml(attribute(local = "rtlCol", codec = OnOff, default = false))]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
#[xml(attribute(local = "embed", prefix = "r", codec = Text, accessor = image_relationship))]
struct SolidColor { /* .. */ }
```

`local` and `codec` are required; `prefix` matches and writes a prefixed attribute; `accessor` names
the Rust method (the default is the wire name in snake case, which the naming convention will usually
want overriding); `required` makes an absent attribute a typed error and `default` gives it a schema
default. Writing neither makes it optional — the third case, whose getter returns `Option`.

**The accessor form is the point.** A grammar that lifted attributes into struct fields would make
the writer *reconstruct* the attribute list, and reconstruction is how unknown attributes, their
order, their prefixes and their quote characters get lost. Nothing in the generated code builds an
attribute list: a getter borrows the vector, a setter reaches exactly one element of it.

### Read never normalizes; a write does

A getter takes `&self`, so it cannot change the file: `rtlCol='on'` that nobody assigned to still
writes `on`, single-quoted, in the position it was read from, and `val='50%'` stays `50%`. The one
canonical form is written only by a setter — `set_rtl_col(true)` writes `true` — which rewrites the
attribute **in place**, keeping its position and the quote character the file used, and escaping the
new value for *that* quote. An attribute that was not there is appended, double-quoted.

A grammar that canonicalized on read would rewrite every file it opened, and would do it invisibly,
because our reader and our writer would agree with each other.

### The codecs

`mjx_ooxml_core::AttributeCodec` is the wire ⇄ Rust conversion for one *kind* of value — a type-level
tag, never constructed. `mjx-ooxml-core` ships the XML-generic ones (`Text`, `Enumeration<T>`, which
covers every generated `ST_*` enumeration because they all spell themselves with `FromStr` +
`Display`); `mjx-ooxml-types` ships the OOXML-specific ones (`OnOff`, `TrueFalse`, `TrueFalseBlank`,
`HexColorRgb`), consuming the `support` normalizers rather than re-deriving them. A crate that owns a
measure type owns its codec, in about fifteen lines — which is how `mjx-dml` will carry `Emu` and
`Fraction` across the seam.

A malformed value is `AttributeError`, never a panic: these are attacker-controlled files.

### Also

`mjx_xml::attribute` — `find`, `decoded_value`, `set`, `remove`: the four in-place operations a typed
accessor is made of, usable by hand. `mjx_xml::text::escape_attribute_in` escapes for a given quote
character (`'` → `&apos;`), which is what lets a setter keep a single-quoted attribute single-quoted
without being able to emit `attr='it's'`. `FromXmlError` gains an `Attribute` variant.

## [0.0.70] - 2026-09-03

The gates reach Word and Excel — before a line of `wml` or `sml` model code exists (MJXOFF-110).

`sample.docx` and `sample.xlsx` have been in `tests/fixtures/` since the first phase and **nothing
had ever schema-validated either of them.** A `w:` part with no arm in the schema table was reported
"skipped, foreign namespace"; the suite counted the remaining parts, found four of them valid, and
reported green. The sentence "the schema gate covers Word" was true and empty at the same time. So
were the ordering half (`assert_deck_is_in_schema_order` asserted only that *some* part had been
audited, and `word/theme/theme1.xml` satisfied it) and the byte-identity half (three suites carried
hand-maintained fixture lists that between them omitted six of the fifteen committed fixtures).

Nothing shipped changes. Every byte this library writes is identical before and after; this release
is test and CI infrastructure, and the round-trip suites did not move.

### The harness is a crate

`crates/mjx-schema-gate` is a new **test-only** crate (`publish = false`, a `dev-dependency` of
`mjx-pptx`, `mjx-docx` and `mjx-xlsx` and of nothing else). An integration test compiles only into
its own crate, so the harness that lived in `mjx-pptx/tests/schema_validity.rs` could never be
reached from the two crates Phases C and D will fill. `crates/mjx-fixtures` is a second, entirely
dependency-free test-only crate holding the committed corpus, so `mjx-opc`'s byte-identity suites —
which sit *below* the gate in the layering — can read the same corpus without an upward edge.

### The three-category rule, with no fourth branch

`mjx_schema_gate::categories` is the only place the line is drawn. Markup we model is **validated**
against its XSD; foreign markup we only preserve (VML, InkML, ActiveX, and the two `docProps`
streams) is **skipped with a written reason**; a root element in a namespace on neither list is a
**hard failure naming the namespace and the part**. There is no "skip anything we have no arm for"
fallback, because that fallback is the hole being closed.

`WordprocessingML` joins the table, so `sample.docx`'s `word/document.xml`, `word/styles.xml`,
`word/fontTable.xml` and `word/settings.xml` are validated against `wml.xsd` for the first time.

### `wml.xsd` can now be compiled at all

`wml.xsd:21` and `shared-math.xsd:13` import `http://www.w3.org/XML/1998/namespace` with no
`schemaLocation`, and the Transitional set ships no `xml.xsd`, so libxml2 could not resolve
`xml:space` and both schemas failed to *compile*. A bare import gives libxml2 no URI, so a catalog
has nothing to rewrite. `crates/mjx-schema-gate/schemas/xml.xsd` — hand-written for this repository,
no third-party licence, nothing fetched at build time — is paired with each XSD through a generated
driver schema. Every validation goes through one, so `shared-math.xsd` inherits the fix.

### Markup compatibility is resolved, not skipped

A part carrying `mc:AlternateContent` or `mc:Ignorable` used to be skipped, which is why LibreOffice's
`word/document.xml` could never be reached. The gate now resolves it with the existing `mjx-mce`
crate — the winning `mc:Choice` selected, ignorable markup in namespaces ECMA-376 does not define
dropped — and validates that view. Only parts that actually carry markup compatibility are
re-serialized; every other part is validated as the exact bytes the package holds.

### Two pre-existing divergences in `sample.xlsx`, now recorded

LibreOffice writes `xml:space="preserve"` on every `s:t` (which `sml.xsd` types as a simple type that
can carry no attribute) and `dateCompatibility` on `s:workbookPr` (not in the 5th-edition
Transitional schema). Both are inputs this project preserves verbatim, so both are recorded as
tolerated deviations with their reasons, matched error-by-error: a *new* defect in either part still
fails.

### The corpus is the directory

`crates/mjx-opc/tests/{roundtrip,tree_roundtrip,package_validation}.rs` and the schema gate all read
`tests/fixtures/` instead of a list. All fifteen fixtures are now inside all four contracts; a file
whose extension is on no list fails, naming it.

### CI

A new required `test (--all-features)` job runs `cargo clippy --workspace --all-targets
--all-features` and `cargo test --workspace --all-features --no-fail-fast`; the existing test steps
gain `--no-fail-fast`; the `schema-validity` job runs the Word and Excel gates beside the PowerPoint
one. `.github/scripts/merge-when-checks-pass.sh` makes the merge step a command whose exit status
gates the merge rather than a sentence instructing a person to look at one.

## [0.0.69] - 2026-09-03

The chart `delete_*` family is `suppress_*` — the naming question v0.0.66's API review raised and
left open, settled before Word and Excel copy the shape (MJXOFF-89).

Twelve public identifiers change across `mjx-chart`, `mjx-pptx` and `mjx-ooxml`, and three of them
are re-projected by each binding. Three `is_deleted` accessors (`DataLabel`, `DataLabels`, `Axis`)
and two `deleted` fields (`DataLabelSettings`, `ChartAxisData`) become `is_suppressed` /
`suppressed`; `delete_chart_data_labels` (on both `Presentation` and `Deck`),
`delete_plot_data_labels`, `delete_data_labels`, `delete_point_label` and `delete_label_for_point`
take a `suppress_` prefix; and `auto_title_deleted` becomes `auto_title_suppressed`. The crate-private
`DataLabels::delete_all` and the two private `clear_delete` helpers move with them. Python sees the
same names (the binding is the identity mapping); TypeScript sees `suppressChartDataLabels` and a
`suppressed` getter in place of `deleteChartDataLabels` and `deleted`.

Nothing else changes. This is a rename: the bytes written for any file are identical before and
after, and no test was added, removed or skipped.

The spelling is now enforced rather than remembered. `.github/scripts/check-suppress-naming.sh` — a
new `naming` job in CI, plus a step in `wasm-pack` over the generated `.d.ts` — fails the build if
any identifier under `crates/*/src`, `bindings/*/src`, the committed `.pyi` or the generated
TypeScript declarations spells this concept `delete`. The wire token is untouched and explicitly
permitted: `flag("delete")`, `"autoTitleDeleted"`, `c:delete` in prose, and the generated ordering
tables in `mjx-ooxml-types` all pass, and each item's docs still name the exact element it writes.

## [0.0.68] - 2026-09-02

Python (PyO3) and WebAssembly/TypeScript (wasm-bindgen) bindings — the facade, projected whole
(MJX-210).

`mjx-ooxml` has been "the binding-ready public API" since v0.0.67. Nothing was bound to it. Two
workspace members now are, and both project the **whole** surface rather than a sample of it:

- **`bindings/mjx-python`** — PyO3, module `mjx_ooxml`, abi3-py39 wheels. 253 methods on `Deck`
  (257 with the `vml` feature), 192 classes, a committed `.pyi` plus `py.typed`, and an exception
  hierarchy of eleven classes rooted at `OoxmlError` — each carrying `.code` and the coordinates
  `.surface`, `.shape`, `.row`, `.column`, `.index`, with `IndexOutOfRangeError` also an
  `IndexError`. The mapping is the **identity**: nothing is renamed except the `None` member of nine
  enumerations, which Python's grammar will not permit.
- **`bindings/mjx-wasm`** — wasm-bindgen, one npm package with conditional exports for a bundler
  build and a browser build. The same surface in **camelCase**, `Uint8Array` in and out, and
  failures as real `Error` objects with `name === "OoxmlError"`, a stable `code` and a `detail`
  object. `deck.free()` is mandatory and the documentation says so in every place a reader might
  look.

### The acceptance test

`crates/mjx-ooxml/examples/build_a_deck.rs` — the guide's whole walkthrough — now exists three
times: once in Rust, once as `test_build_a_deck.py`, once as `build_a_deck.mjs`. Each of the two
bindings runs the Rust one and compares its own deck **part by part, byte for byte**. That is what
proves the curated subset is sufficient, and it is what would catch a method wired to the wrong
`Deck` method: nothing about the types would complain, and one part payload would differ.

### Added to `mjx-ooxml`

Ten types were reachable from the re-exported vocabulary but not themselves re-exported, so a
binding could hold a value it could not name: `AdjustHandle`, `ConnectionSite`, `ColorKind`,
`FontSlot`, `TableStyleBorder`, `ThemeFontReference`, `GuideFormulaError`, `AxisKind`, and
`AdjustmentSpec` with `AdjustmentAxis` / `AdjustmentBound`. `tests/vocabulary_closure.rs` uses all
ten through the facade, so the list stays closed.

`PartialEq` was added to `mjx_pptx::TableStyleFormat`, `mjx_pptx::TableStyleDefinition` and
`mjx_chart::ChartData`, whose siblings all had it; and the two terse "Delegates to …" doc summaries
on `Deck::set_cell_run_properties` / `set_cell_text_range_properties` were written out, because the
bindings use those summaries as their docstrings.

### The recorded divergence

`PLAN.md`, `README.md` and `CLAUDE.md` said bindings would live in a **separate cargo project** on a
**UniFFI → wasm → C-ABI** stack targeting Kotlin, Swift, JavaScript and C, deferred to Phase 7. They
do not, and it is not. All three files now say what was built and why — see the "Recorded
divergence" section of `PLAN.md`.

### `unsafe`

The two binding crates are the first in this workspace to carry `#![allow(unsafe_code)]`, and the
first use of the `deny`-not-`forbid` escape hatch the workspace lints were written to permit. The
justification is that no `unsafe` is hand-written: every unsafe block is generated by `#[pyclass]`
or `#[wasm_bindgen]`. CI greps `bindings/*/src` and `bindings/*/tests` for `unsafe` outside a
comment and fails if it finds any, so the justification cannot quietly become false.

### Measured, not gated

The WebAssembly payload is **2,484,641 bytes raw and 848,380 gzipped (828 KiB)** with `lto = true`,
`codegen-units = 1`, `strip = "debuginfo"` and `wasm-opt -Oz`. The specification estimated
400–700 KB and asked for a measurement before a budget; this is the measurement, and CI reports it
on every run rather than failing on a number nobody has justified yet. `panic = "unwind"` is kept
workspace-wide because PyO3 needs it to turn a panic into a Python exception rather than a process
abort.

### CI

Three new jobs — `bindings-build` (the `unsafe` check, both crates built the way they ship),
`wasm-pack` (headless Chrome, Node, both npm targets, the size report) and `python-wheel` (abi3
wheels on Linux, macOS and Windows, installed from the wheel, then `pytest` and `mypy --strict`, and
the same wheel re-checked on a much later interpreter). The cross-build matrix excludes both binding
members: a PyO3 `cdylib` needs a host interpreter and a wasm `cdylib` means nothing off `wasm32`.
The `examples` job now runs every crate's examples, not only `mjx-pptx`'s.

## [0.0.67] - 2026-09-02

The `mjx-ooxml` facade — `detect_format`, `Deck`, FFI-shaped errors, the curated surface (MJX-210).

`mjx-ooxml` had been **62 lines of documentation and no code** since the workspace was laid out,
while the docs called it "the binding-ready public API". It is now that API.

**`detect_format` reads the package, not the filename.** It opens the OPC container, follows the root
`officeDocument` relationship and maps the main part's content type against the fifteen ECMA-376 and
macro-enabled types. That is the only way `.pptm` and `.potx` — the same PresentationML markup under a
different declaration — can be told from `.pptx`, and the only answer that survives a renamed file.
Word and Excel are recognized and refused by name (`ErrorCode::UnsupportedFormat`), so a caller who
hands a `.docx` to a PowerPoint library is told it is a Word document rather than that some part
failed to parse. Detection working before editing does is the whole point.

**`Deck` restates 251 of `Presentation`'s 273 methods in types a foreign function boundary can
express**: `impl Into<Surface>` becomes a concrete `Surface`, `impl Into<ShapePath>` a concrete
`ShapePath`, `usize` becomes `u32` on every parameter and return, `&PartName` becomes `&str`, and a
borrowed `Option<&[u8]>` becomes an owned `Option<Vec<u8>>`. The facade owns its own `Surface` and
`ShapePath` — carrying `u32`, converting at the boundary, allocation-free for the top-level case —
because adding `From<u32>` beside `From<usize>` on `mjx-pptx`'s would have made every bare integer
literal in `deck.shape_fill(0, 2)` ambiguous across the workspace.

Sixteen methods are deliberately absent, each unreachable across FFI or reachable another way:
`Presentation::shape` (returns a cursor borrowing the deck), the five closure-taking table-style and
VML readers, the four surface `*_part` accessors and the six `*_rel_id` accessors (part-graph
identity for content that is already reachable by index or by bytes). `Deck::presentation_mut` is the
Rust-only door to all of them; there is no `Deck::package`, because handing out `&mut Package` would
give a caller the whole part graph and make every invariant `save` enforces unenforceable.

**One exclusion the specification proposed was checked and reversed.** The per-cell formatting
setters were to be dropped as reachable through `format_cells(Cells, &CellFormat)`. They are not:
`format_cells` deliberately skips a cell covered by a merge, so only what renders is touched, while
`set_cell_fill` reaches a covered cell — whose own formatting reappears when the region is unmerged.
Dropping them would have dropped that, so all fifteen are exposed, and a test asserts the two
spellings really are different calls.

**One `Error`, eleven stable codes.** `Error { code, message, detail, source }` collapses all 65
`PptxError` variants and all 9 `OpcError` variants into `Io`, `MalformedDocument`, `InvalidDocument`,
`IndexOutOfRange`, `WrongKind`, `NotFound`, `NothingToRead`, `InvalidArgument`, `StructureConflict`,
`UnsupportedContent` and `UnsupportedFormat`, plus the human message and the `surface` / `shape` /
`row` / `column` / `index` coordinates a binding turns into exception attributes. Rust callers lose
nothing: `source()` downcasts back to the `PptxError`. The classification is an exhaustive `match`
with no wildcard arm — which is why `PptxError` stopped being `#[non_exhaustive]` — so a new variant
fails the build until it is classified.

**`Deck::save` inherits the validation `Presentation::save` performs** rather than routing around it.
A facade that widened what a caller could break would be a regression, so this is tested on a deck
that is genuinely invalid: `save` refuses it and `save_unchecked` writes it.

Also here: `Presentation::{remove_unused_parts, external_links, retarget_external_link}` — package
hygiene as three thin delegates rather than an exposed `package()` — and `SlideSize::{widescreen,
standard, from_emu}`, so a caller building a deck from nothing states a size by name instead of by
struct literal.

`examples/build_a_deck.rs` is the guide's walkthrough written through the facade, **naming no crate
below `mjx-ooxml`**; if the re-export list were insufficient it would not compile.

## [0.0.66] - 2026-09-02

API review and reorganisation — the last iteration before `v0.1` freezes the surface (MJX-37).

`crates/mjx-pptx/src/presentation.rs` had reached **12,771 lines and 266 public methods** in a single
`impl Presentation` block. It is the file the whole PowerPoint surface lives in, and the file
`mjx-docx` and `mjx-xlsx` will copy on day one of Phases C and D, so its shape is worth more than its
size suggests.

**The split changes no path a caller imports.** `presentation/` is sixteen modules along the seams
the guide already reads in — deck addressing, slide lifecycle, the shape tree, notes, text,
hyperlinks, table cells, table structure, bounds, appearance, the effective readers, charts, chart
decoration, pictures, legacy content, and the element builders shared by more than one of them. Every
method stays an inherent method on the one re-exported `Presentation`; the helpers that moved out are
`pub(super)`, visible only inside `presentation`. The public-item lines before and after are identical
as a set, and the workspace suite was unchanged at 1,528 passing tests across the move — which is what
a reorganisation is supposed to look like. It is committed separately from every behaviour change so a
reviewer can see that the move moved nothing.

`tests/public_paths.rs` guards that from outside the crate: one authored deck driven through every
seam using only `mjx_pptx::` paths, because a test that reached into `crate::presentation::text` would
prove nothing a caller can rely on.

**The three named inconsistencies are settled.** `cell_span` answers `(rows, columns)` like
everything else on the table surface. The eight DrawingML effects each take what the schema makes
required in `new` and name the rest with `with_` — so a shadow's distance no longer costs eight
`None`s — while an attribute the builder does not name stays unset, and an unset attribute is not
written. And the three `#[allow(clippy::too_many_arguments)]` sites, re-examined now that `Cells` and
`CellFormat` exist: one was dead and is gone, and the two that remain — eight distinct cell
coordinates apiece — are `#[expect]` with their reason, so the day the list fits, the attribute fails
the build instead of quietly outliving its cause.

**A loop in a doc example is a design defect.** Three remained across the guide, the README-adjacent
pages and the examples, all the same shape: `for i in 0..count()` with a fallible accessor inside,
rebuilding a list the deck already has and re-borrowing the part once per entry.
`Presentation::layouts` answers the layout inventory as `Vec<LayoutInfo>`, `Presentation::shapes`
answers a surface's shapes as `Vec<ShapeInfo>` — index, kind, and the placeholder slot each fills —
in one read, and `Presentation::shape_for_placeholder` answers the search *where did this template put
the title?*. Every loop still standing in a doc example iterates a collection the API handed over,
with no `?` inside it.

**`Presentation::from_package` is public**, as the facade needs: the constructor for a caller who
already holds the package — one `mjx-opc` opened directly, or one a facade opened once and dispatched
on by content type rather than handing the bytes back to each format crate to re-open.
`mjx_opc::Package` is re-exported from `mjx-pptx` alongside it, on the same reasoning the chart types
already are: a caller should not have to name another crate to state a parameter type.

**The naming sweep** covered all 1,561 public identifiers of the eleven merged children. Its nine
breaks are tabulated under **Unreleased — 0.1.0** above; the summary is that `blip` is not a word,
that an abbreviation named after an attribute is still an abbreviation, and that a struct a caller
reads and the struct it writes back should name the same field the same way.

Fidelity is unchanged and was the acceptance criterion throughout: per-part byte identity, modeled
round-trips, and edit isolation all hold, `MJX_REQUIRE_SCHEMA=1` passes 51 (52 with `--features
vml`), and all eight examples verify their own output.

## [0.0.65] - 2026-09-02

Chart decoration — data labels, per-point formatting, trendlines and error bars (MJX-116).

`c:dLbls`, `c:dLbl`, `c:dPt`, `c:trendline` and `c:errBars` were preserved verbatim and had no typed
surface at all. A5 closed the chart *data* half completely — every plot type's series, literal and
multi-level sources, axes, gridlines, titles, legend and series fill/outline — and stopped at the
decoration deliberately rather than half-modelling it. That was right for its scope; leaving it
unowned was not. **Data labels are the part of a chart a reader actually reads**, and until this
release a caller could not ask what one said, could not switch a series from value to percentage, and
could not author a chart that labelled itself.

All four families now **read, author and edit**. `crates/mjx-chart/src/decoration.rs` adds
`DataLabels`, `DataLabel`, `DataPointFormat`, `Trendline` and `ErrorBars`, each with the same
ordered-`content` + `Raw` shape as everything else in the crate, so an element nothing touched still
re-emits byte-for-byte. `c:plus` and `c:minus` are the same `CT_NumDataSource` a series' `c:val` is,
so a custom error bar's lengths read and write through the existing `NumericData`.

**The three tiers of a data label.** ECMA-376 §21.2.2.49 says `c:dLbls` states the settings "for an
entire series **or the entire chart**", and a `c:dLbl` overrides them for one point — so a label
resolves over three tiers, and `DataLabelSettings::inherit` merges them **per setting**, not per
tier: a series that only says `c:showVal` still takes its plot's `c:dLblPos`. A `c:delete`
short-circuits the chain, because `CT_DLbls` puts it in one `xsd:choice` with the settings group and
an element carrying one cannot carry the other. There is deliberately no fourth tier — `CT_Chart`
declares no `c:dLbls` of its own — and the model says so rather than inventing one.
`ChartLabelScope` names the three tiers on the `mjx-pptx` surface, so "label this series" and "label
this point" cannot be the same call, and three verbs separate three intentions: *state settings*,
*draw nothing here* (`c:delete`), and *say nothing here* (remove the element, inherit again).

**A `c:dPt`'s `c:idx` is never renumbered.** Per-point formatting is anchored by index into the
series; renumbering one when a series changes length would move a point's colour silently onto a
different point, which is worse than leaving it dangling. Nothing in this release rewrites an index
except an explicit `set_index`. `Series::decoration_beyond_data` and
`Presentation::chart_dangling_decoration` *report* the anchors an edit left past the end;
`drop_chart_dangling_decoration` removes them, and nothing removes them on a caller's behalf. A
`c:idx` that is not a number — `-1`, or a value past `u32::MAX` — addresses no point, is never
matched by a lookup, is never renumbered, and rides through a round-trip untouched. Writing past the
end is the same rule from the other side: it is refused with a typed error rather than written as an
anchor that names nothing.

**Writing is bound to the owning plot's kind, and both the placement and the refusal come from the
schema.** `SeriesDecoration` carries a `ChartKind` because `CT_BarSer` puts `c:dPt` at rank 6 and
`CT_PieSer` at rank 5, and because `CT_PieSer` declares no `c:trendline` and no `c:errBars` while
`CT_SurfaceSer` declares no decoration at all. Both questions are asked of the generated
`child_order` tables, which gain 29 named constants for this — the five decoration types, the eight
`CT_*Ser` and the sixteen `CT_*Chart` — rather than of a list written by hand. `ChartDataError` gains
seven variants, every one raised **before anything is written**, the way `ChartData::validate`
already refused a shape the schema rejects: a point index past the end of a series, a decoration the
series type does not declare, leader lines on one point's label (only `Group_DLbls` declares them),
an `ST_Order` outside 2–6, an `ST_Period` below 2, a non-finite measure, and custom error bars whose
length nothing determines.

Every name is sourced from the ECMA-376 Part 1 prose, never guessed: §21.2.3.11 for `ST_DLblPos`
(`ctr` → `Center`, `inEnd` → `InsideEnd`, `bestFit` → `BestFit`), §21.2.3.50 for `ST_TrendlineType`
(`movingAvg` → `MovingAverage`, `exp` → `Exponential`), §§21.2.3.12–14 for the error bars
(`cust` → `Custom`, `stdErr` → `StandardError`). The exact wire token appears in each item's docs, and
a token the schema does not admit reads as `None` rather than as a guess. The schema's own defaults
are honoured: a bare `<c:showVal/>` is `true`, `<c:trendlineType/>` is `linear`, `<c:errBarType/>` is
`both`, `<c:order/>` and `<c:period/>` are 2.

`ChartData::data_labels` lets a chart label itself the moment it is authored, refused by `validate`
for the two surface kinds, which declare no `c:dLbls`.

**Preservation gained reach and changed nothing.** The `mjx-opc` round-trip suites and tier-3 edit
isolation are unchanged, and decorating a chart dirties `chart1.xml` and *nothing else* — not even
the embedded workbook, because decoration is not data. With `MJX_REQUIRE_SCHEMA=1`, every authored
decoration validates against `dml-chart.xsd` under `xmllint`, including the two cases the ranks make
distinct: a pie chart, whose `CT_PieSer` places `c:dPt` differently, and a scatter chart with two sets
of error bars, which `CT_ScatterSer` admits and `CT_BarSer` does not.

The *Limitations* row in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` naming these four families
is **removed**, not softened — it moves to "what used to be here", where rows go when they close by
being done.

## [0.0.64] - 2026-09-02

Subtree copy-on-write — the copy-on-write `mjx-opc` does per part, now done per subtree (MJX-248).

A6 found that the fidelity reader records each attribute's name, value and quote but **not the
whitespace separating it from the previous one**, so a start tag Office wrapped across lines
re-emitted on one. It pinned the shortfall in `KNOWN_REFLOWS` and stopped, because the obvious fix —
a whitespace field on `RawAttribute` and a trailing-whitespace field on `RawElement` — costs a value
at every construction site, adds size to the hottest data structure in the library, and buys exactly
one preserved property. The next one (entity spelling, comment placement, self-closing style) would
cost the same again.

**The tree is span-preserving instead.** Every element parsed by `mjx_xml::fidelity` remembers the
byte range it came from; the document keeps the buffer; and the serializer writes an unmodified
element by copying that range rather than rebuilding it. One field subsumes the whole family:
whitespace between attributes, whitespace before `/>`, quote style, the spelling of a character
reference (`&#38;` stays `&#38;`), and the placement of comments and processing instructions inside a
subtree. `KNOWN_REFLOWS` is **deleted**, not emptied — its two-way pin fires when a part starts
round-tripping, so the entry could not have been left behind — and `crates/mjx-opc/tests/roundtrip.rs`
and `tree_roundtrip.rs` now have no exceptions at all.

**The invariant is structural, not remembered.** A range is only sound while the element still is
what was parsed, and `RawElement`'s fields are public, so there is nowhere to hook a "clear the span"
call. `RawElement` therefore keeps its attribute and child lists in a `RawElementContent` it
`Deref`s to: reads are unchanged (`element.children`, `element.attributes` still resolve), and any
*mutable* access goes through `DerefMut`, which drops the range. Because mutable descent into a child
passes through every ancestor's child list, that drops the range along the whole path from the root —
which is exactly "a mutation clears the span on that node and every ancestor", obtained by
construction rather than by discipline. `Clone` drops it too, so a subtree copied into another
document can never be written from the buffer it left behind, and `PartialEq` ignores it, so
`RawElement` equality still means "the same markup".

**The one way this could corrupt a file is namespaces**, and it is pinned first. A verbatim subtree
carries prefixes but not the `xmlns:` declarations that bind them; if a rewritten ancestor pruned a
declaration, every descendant beneath it would come silently unbound. It cannot, and the reason is
structural: the reader keeps `xmlns` declarations as ordinary attributes in document order and the
writer emits every attribute an element holds without inspecting any of them.
`crates/mjx-xml/tests/subtree_cow.rs` opens with the test that fails if that ever stops being true —
it namespace-resolves the *output*, the way a consumer does.

**The range is untrusted on the way out.** It is sliced fallibly, and then checked against the
element it claims to describe: the bytes must open with `<` plus that element's qualified name
followed by a delimiter, and close the way the element says it closes. That is what catches a mutated
`name` or `empty` — the two fields deliberately left outside the `Deref` because navigation reads
them constantly — and it means a wrong range degrades to a re-flow, never to wrong bytes. Adversarial
cases are pinned: out-of-bounds, inverted, pointing at a different element, and the one a naive
`starts_with` gets wrong (`<a>` must not claim `<abbr>`'s range).

Measured on a synthetic 2.3 MiB slide (80,004 elements, `cargo run --release -p mjx-xml --example
mjx248_measure`): `size_of::<RawElement>()` 64 → 72 bytes, **+8 bytes per element** — the span packs
into a `u32` start plus a `NonZeroU32` end, so `Option` needs no discriminant, and moving the lists
behind the `Deref` costs nothing. Serializing that part after editing one attribute of one element:
**4.59 ms → 0.27 ms, 17x faster**; untouched, 0.08 ms. A part read but not edited retains no extra
memory at all — `mjx-opc` now shares one `Arc<[u8]>` between the bytes it re-emits and the tree that
indexes into them — and an edited part holds its source buffer, which
`Package::release_unused_part_sources` reclaims once nothing can be copied from it.

One byte-fidelity defect the new adversarial corpus found is fixed with it: `<!DOCTYPE a>` lost the
space after `<!DOCTYPE`, because quick-xml trims it and the writer rebuilt the wrapper. The doctype's
inner bytes now come out of the source.

`crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` states the stronger guarantee — every subtree you
did not touch is byte-for-byte what it was — and drops the re-flow limitation, which is gone. It
gains a narrower one in its place: three surfaces (`edit_vml_drawing`, `edit_chart`, the table-style
list) read a whole part into a typed model and write the whole part back, so subtree copy-on-write
does not reach inside those; slide edits, which navigate in place, are unaffected.

## [0.0.63] - 2026-09-02

Schema-order emission — children are written in `xsd:sequence` order by construction (MJX-248).
OOXML complex types are overwhelmingly sequences, and **children in the wrong order are invalid even
when every child is present and every child is itself correct** — a repair-dialog defect, not a
cosmetic one. Nothing in the workspace enforced that on write: order was whatever each hand-written
serializer happened to do, so correctness rested on the author having read the XSD for that type and
on a fixture happening to exercise it. Fourteen separate hand-copied rank tables had grown across
`mjx-dml`, `mjx-chart` and `mjx-pptx`, one per type, each added by whoever noticed.

**The order now comes from the schema.** `cargo run -p xtask -- codegen` reads the `xsd:complexType`
content models of `dml-main.xsd`, `pml.xsd` and `dml-chart.xsd`, flattens each one — resolving
`xsd:group` references across schemas — and commits
`mjx-ooxml-types::child_order`: every child of every complex type of those schemas, with the position
it occupies. Alternatives of an `xsd:choice` *share* a position, which is exactly the "either an
`a:solidFill` or an `a:noFill`, and whichever is there is the one to replace" question a writer asks.
A type whose own model is `xsd:choice` or `xsd:all` — `CT_Path2D`'s repeating path commands, for
instance — is recorded as unordered rather than given a false order.

### The boundary this does not cross

Placement is a write-side operation and only ever runs on a child a caller asked to write. **Nothing
reads a document and rewrites it into schema order.** A real file may carry children in an order the
schema permits but this table would not have chosen, and re-ordering it would be corruption of the
caller's document rather than a fix. Existing children are never sorted; a new child is inserted
after the last sibling that must precede it. Markup the table does not name — an unmodelled element,
a foreign namespace, a comment, an `mc:AlternateContent` — is invisible to placement: it never moves,
and it never moves the insertion point, so it keeps its position relative to its known neighbours.

### Added

- `mjx-ooxml-types::child_order` — `ChildOrder`, `ChildSlot`, `ContentModel`, `TypeReference`, the
  placement primitives (`ChildOrder::replace_or_insert`, `insert`, `insert_index`,
  `insert_index_of_names`, `rank_of`, `slot`), the ordering audit (`ChildOrder::first_out_of_order`,
  `audit_tree`, `TreeAudit`, `OutOfOrderChild`), the by-symbol lookups (`find`, `root_element`), the
  three generated tables (`DML_MAIN_TYPES`, `PML_TYPES`, `DML_CHART_TYPES`) and twenty-eight named
  constants for the types this workspace writes.
- A child-order audit inside the schema-validity suite that runs on **every** authored-deck case,
  with or without `References/`: it walks every element of every part whose root the tables name and
  fails on the first child out of its type's sequence. `xmllint` catches an ordering fault only for
  the shape some case happens to author, and only where the schemas are installed.

### Changed

- Every insertion path in `mjx-dml`, `mjx-chart` and `mjx-pptx` now places children through the
  generated table. The fourteen hand-written rank tables are gone.

### Removed

- `TableStylePart::rank` and `CellBorder::rank`. Both existed only to feed a hand-written ordering
  table; the generated one is now the single source, and a second copy of a sequence is the thing
  this release exists to remove. `TableStylePart::all` and `CellBorder::all` are unchanged.

## [0.0.62] - 2026-09-02

Package invariant validation — a deck that would need repair is not written (MJX-248). This library
was very good at *not touching* what it does not understand, and had essentially no defence for what
it *does* write. `Package::save` performed no package-level check of any kind, so markup naming a
relationship its `.rels` never declared, a relationship pointing at a part that was not there, a part
no content-type rule covered, and duplicate identifiers where the format requires uniqueness could all
be written and shipped. Every one of them makes PowerPoint say it "found a problem with the content
and needs to repair", and none of them is visible to the schema gate: A1/A2 validate each part against
its XSD in isolation, and every one of these defects is a property of the package *graph*, perfectly
schema-valid part by part.

**`save` now validates first, and the check is not opt-in.** A check you have to remember is a check
that ships the fault it was meant to catch.

- `mjx-opc`: `Package::validate`, `Package::save_unchecked`, and `PackageDefect` — one variant per
  invariant, each naming the part, relationship and identifier at fault:
  `PartWithoutContentType` (ECMA-376 Part 2 §6.2.3), `RelationshipTargetMissing`,
  `UnresolvableRelationshipTarget`, `DuplicateRelationshipId` (§6.5.3),
  `UndeclaredRelationshipReference` — every attribute in the shared relationship-reference namespace,
  not `r:id` alone, because `shared-relationshipReference.xsd` types all fourteen of them
  `ST_RelationshipId` — and `PartIsNotWellFormedXml`. Reached through `OpcError::Invalid`.
- `mjx-pptx`: `Presentation::validate`, `Presentation::save_unchecked`, and `PresentationDefect`:
  `DuplicateShapeId`, `DuplicateListEntryId`, `DuplicateListEntryReference`,
  `ListEntryTargetHasWrongContentType` and `UnlistedRelationship` — the `p:sldIdLst` /
  `p:sldMasterIdLst` / `p:sldLayoutIdLst` agreement with a part's relationships, in both directions.
  Reached through `PptxError::InvalidPresentation`.
- `mjx-opc`: `Package::authored_xml_parts`, `ZipEntry::provenance`, `ZipEntry::tree` and
  `PartProvenance` — the validation scope, defined once so both layers agree on it.

**The scope is the markup this library will write.** A part still holding the bytes it was opened with
is re-emitted verbatim and is never faulted, so a file that arrives broken can still be written back,
and *reading* a part can never change whether a package saves. The moment an edit makes those bytes
ours, the same defect is refused. `save_unchecked` is the deliberate escape hatch.

**A corrupting bug the validator found on its first run.** `add_ole_object` gave its snapshot picture
a hard-coded `p:cNvPr@id` of `0` while the frame took an allocated id, so two OLE objects on one slide
wrote two shapes with the same non-visual id — a duplicate PowerPoint repairs. Fixed, with a
regression test that asserts the ids rather than only that the save succeeded.

The cost, measured on the largest fixture (`charts.pptx`, 43 entries, 39 relationships): **35.7 µs**
to validate against 3.2 ms to write the container — about 1% of a save. Nothing that arrived as
container bytes is ever tokenised.

## [0.0.61] - 2026-09-02

The remaining model gaps, and an honest gap table (MJX-43). The guide's gap list had accumulated
rows that were no longer true, rows that were real, and rows that were deliberate decisions filed as
though they were oversights. This release closes the real ones, restates the decisions as decisions
with their reasoning, and rewrites the page around the difference.

**A shape's own list style is authorable.** Tier 3 of the text ladder — `a:lstStyle` on a shape's
text body, the tier that says *every paragraph at this indent level, in this shape* — could be read
and resolved through since the ladder was written, and could not be stated. It now can:

- `mjx-pptx`: `Presentation::shape_list_style_level`, `set_shape_list_style_level`,
  `clear_shape_list_style_level`, `shape_list_style_default`, `set_shape_list_style_default`,
  `clear_shape_list_style_default`, and `clear_shape_list_style` for the whole element. The setters
  merge, as every other setter does; a clear that finds nothing changes nothing and does not dirty the
  part.
- `mjx-dml`: `TextListStyle::new`, `set_level`, `set_default_properties`, `remove_level`,
  `remove_default_properties`; `TextBody::set_list_style` and `remove_list_style`. A new level is
  placed by `CT_TextListStyle`'s sequence and a new `a:lstStyle` by `CT_TextBody`'s — between
  `a:bodyPr` and the first `a:p` — because order is validity, not style.

**The gap table is now two lists.** Non-goals, each with the reason it is a decision, and *built but
not yet verified against Office*, each with the work that will verify it. Four rows closed outright:
merge-aware selections (already true in the code and now proven by the cases that discriminate — a
merge anchored outside the selection, and the text and paragraph formatters, not just the cell
formatter), the `a:lstStyle` setter above, `Scene3D::backdrop`, and a font slot the theme does not
define — which was correct behaviour listed as a gap. `extLst` is restated as what it has always
been: the schema's own unknown bucket (`CT_OfficeArtExtension` is a required `uri` plus
`xsd:any processContents="lax"`), preserved verbatim through an edit and pinned there by tests at
both tiers rather than merely asserted.

- New fixture `tests/fixtures/table_extensions.pptx` — a table whose `a:tblPr` and one `a:tcPr` carry
  a vendor extension — registered with the OPC round-trip suites, the fidelity-tree suite and the
  schema gate.

## [0.0.60] - 2026-09-02

Typed surfaces for the content that is not DrawingML (MJX-140, absorbing MJX-139). Five kinds of
content — OLE objects, ActiveX controls, ink, SmartArt diagrams and legacy VML — round-tripped
perfectly and could be *read*, and that was all. There was no authoring, no editing, and no way to
answer the question that makes any of it useful: **which shape is this?** An InkML part was findable
but untraceable; a diagram was `GraphicFrameKind::Diagram` and nothing more; `mjx-vml` had been 69
lines since Phase 0, its own doc comment deferring "rich modeling and shape-level references" to "a
later phase" that had no owner and no date.

Every one of the five now has read, author **and** edit coverage. Nothing about the round-trip
guarantee changes: modelling a type only adds reach, and the edit-isolation tier over each fixture is
the gate this release is measured against.

`mjx-vml` — from 69 lines to a real model:

- `Drawing` (the `<xml>` root of a `vmlDrawingN.vml`, or any element holding VML shapes — a Word
  `w:pict`, an `mc:Fallback` branch), `Shape`, `ShapeTemplate`, `ShapeGroup`, `ImageData`, `TextBox`,
  `Fill`, `Stroke`, `ShapePath`, `DiagramText`; the Office extensions that carry the references —
  `ShapeLayout` / `ShapeIdMap`, `EmbeddedOleObject`, `Ink`, `ShapeProtections`; and
  `AttachedObjectData`, the legacy form control's own record. `DrawingPart` reads and writes a whole
  part.
- The point of it is one hop: `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name a
  VML shape's `id`, and `Drawing::shape_by_identifier` resolves it.
- Names come from the ECMA-376 Part 4 §19 prose, never the wire token — `v:shapetype` is a
  `ShapeTemplate`, `o:idmap` a `ShapeIdMap`, `x:ClientData` an `AttachedObjectData` — and
  `ST_ObjectType`'s nineteen values expand to `PushButton`, `DropdownBox`, `AuditingLine` and the rest.

`mjx-pptx`:

- **Ink.** `ink_references` ties every InkML part to the content part that names it, finding both
  PresentationML's `p:contentPart` and the `p14:contentPart` producers wrap in `mc:AlternateContent`;
  `ink_part_for_shape` and `shape_for_ink_part` walk it either way. `add_ink` writes the part and the
  reference; `set_ink_content` replaces the strokes without touching the slide. Both check the root
  namespace, so a package cannot end up declaring `application/inkml+xml` over something else.
- **SmartArt.** `diagram_relationship_ids` and `diagram_parts` expose the whole graph — the four parts
  a `dgm:relIds` names plus the cached drawing, which hangs off the *data* part rather than the frame.
  `add_diagram` writes all four with their relationships and the frame; `DiagramContent::vertical_list`
  generates a working diagram from a list of labels, `from_parts` takes four documents of your own.
  `set_diagram_part` replaces one of them in place.
- **OLE and ActiveX.** `add_ole_object` (an embedded stream, a whole embedded package, or a link) and
  `add_activex_control` (the `ax:ocx` part, its `.bin` state and the `p:controls` container), plus
  `set_ole_prog_id`, `set_ole_object_data`, `set_ole_snapshot_image`, `set_activex_control_name`,
  `set_activex_state`, `set_activex_snapshot_image` and `remove_activex_control`. Reading gains
  `activex_class_id` and `activex_persistence`. Both kinds can be bound to their legacy fallback with
  `set_ole_legacy_shape_id` / `set_activex_control_shape_id` and read back with
  `ole_legacy_shape_id` / `activex_control_shape_id`.
- **VML** (behind the `vml` feature): `vml_drawing_part`, `with_vml_drawing`, `edit_vml_drawing`,
  `add_vml_drawing`, and the headline `with_vml_shape_for_ole_object` /
  `with_vml_shape_for_activex_control`, which walk from the modern frame to the legacy shape that
  draws it. The feature's boundary is unchanged and now documented: it decides only whether *this*
  crate re-exposes the surface, since `mjx-vml` is a normal crate `mjx-docx` will depend on directly.

Verification: the schema gate gains a DrawingML-diagram arm, because this project now writes those
four parts — `dml-diagram.xsd` joins the markers `harness()` requires, and a new case pins that all
four are *validated* rather than skipped. Seven new schema cases cover the authored diagram, OLE
object, ActiveX control, ink and VML deck plus an edited OLE object. `mjx-opc`'s `tree_roundtrip` now
covers the four legacy fixtures.

One limitation surfaced and is recorded rather than hidden: the fidelity reader does not preserve the
whitespace *between* attributes, so a start tag whose attributes were wrapped across lines re-flows
onto one line when its part is edited. It never touches a part nobody edited — those keep their
original bytes and are never re-serialised — but Office wraps VML start tags far more often than it
wraps a slide's, so it shows there first. `KNOWN_REFLOWS` in `crates/mjx-opc/tests/tree_roundtrip.rs`
pins it, and the fidelity guide states it. Fixing it means adding a field to `RawAttribute` and
`RawElement` in `mjx-ooxml-core` and touching ~140 construction sites across every crate, which is an
architectural decision rather than a fix to take inside this change.

The guide's "preserved but not modelled" table is gone, replaced by a read/author/edit table for the
five and five honest non-goal rows (InkML strokes, the SmartArt layout engine, `ax:ocxPr`, VML path
evaluation, and the re-flow above). A seventh example, `legacy_content`, exercises all five and runs
in both feature modes.

Still open from MJX-140: **producer-authentic validation**. Every fixture here is still hand-crafted,
so what the schema gate proves is that our reader agrees with our writer against markup we wrote.
Obtaining decks Microsoft PowerPoint actually produced needs Office, and belongs with the runtime
verification work.

## [0.0.59] - 2026-07-31

The usage guide and the first runnable examples (MJX-209). The repository documented every *item* —
every public item has rustdoc, `missing_docs` is a lint, a strict rustdoc job gates CI — and one
*concept*, the effective-properties page. It documented no *task*: nothing answered "I have a `.pptx`
and want to change the title", nothing answered "I want to produce a deck", and there was no
`examples/` directory or runnable program anywhere in the workspace.

First of three workstreams to `v0.1`: **documentation → external application surface → validation**.

`mjx-pptx`:

- A five-page guide under `crates/mjx-pptx/docs/guide/`, surfaced as the doc-only `mjx_pptx::guide`
  module tree: *building a deck* (the whole story once, end to end), *shapes and text*, *tables,
  charts and pictures*, *inheritance, layouts and masters*, and *fidelity and the known gaps*. The
  last is a candour page listing every deliberate gap with its issue — no embedded chart workbook, no
  guide-formula evaluator, selections that are not merge-aware, colour transforms implemented from the
  prose but unverified against Office, and the fact that no test in this repository reads a file
  PowerPoint wrote. **All 48 doctests in the guides compile against the real API.**
- Six examples under `crates/mjx-pptx/examples/`, each reopening what it wrote and asserting something
  about it: `build_a_deck`, `read_deck` (which re-saves and proves all 17 parts stayed byte-identical),
  `edit_text` (which reports that retitling a slide dirties exactly one part), `style_shapes`,
  `build_table`, `charts_and_media`. `anyhow` is added as a dev-dependency; examples are the one place
  file I/O belongs, because the library is bytes-in/bytes-out and the caller reads and writes.

CI: a new `examples` job runs all six, and the office-open job now feeds `build_a_deck`'s output
through LibreOffice — so "the guide's headline example produces a deck Office opens" is a merge gate.

Also: the README gains a quickstart, a guide table and the example commands; the `mjx-ooxml` facade
gains the guide ladder; and PLAN.md's Phase 3b, which still described tables as in progress and
speaker notes as open, records what actually shipped and adds Phase 3c.

Two API observations surfaced while writing the examples, recorded for the `v0.1` review (MJX-37):
`cell_span` answers `(columns, rows)` while `table_dimensions` answers `(rows, columns)`, and
`OuterShadowEffect` has no `Default` though `EffectListSpec` does. Both are documented where they
bite rather than worked around silently. No behaviour change in this release.

## [0.0.58] - 2026-07-31

Paragraph-hierarchy audit (MJX-22, closing MJX-38). The seven-tier text ladder passed its tests, but
those tests reached the interesting cases by mutating a deck through the builder API rather than by
reading a file, so two disagreements with ECMA-376 Part 1 had gone unnoticed. Both are fixed here,
each cited to the prose that settles it.

`mjx-pptx`:

- **A list-style tier now contributes its `a:defPPr` beneath its level.** `TextListStyle::default_properties`
  had existed since the text model landed and resolution never called it, so a tier supplying nothing
  but an `a:defPPr` contributed nothing and a paragraph at a level its style does not define came back
  empty. §21.1.2.2.2 defines `a:defPPr` as the properties applied "when no other paragraph properties
  have been specified"; §21.1.2.2.6 says the same of a paragraph. The audit also confirms there is
  **no** fallback to `a:lvl1pPr` — §21.1.2.4.13 keys the nine level elements strictly to `a:pPr@lvl`,
  so the existing level behaviour was already right.
- **A shape that is not a placeholder now takes a master text style.** Tier 5 was gated on `p:ph`;
  §19.3.1.35 instead splits by kind — `p:bodyStyle` for a text box (`p:cNvSpPr@txBox`), `p:otherStyle`
  for any other non-placeholder shape. Tier 4 keeps its gate: without a slot there is nothing to
  match. This changes what effective text a deck containing plain shapes or text boxes reports. Real
  PowerPoint is believed to match the previous behaviour, so it is isolated in one commit and tracked
  for validation against an Office-saved deck.
- New `slide::shape_is_text_box`, the reader counterpart of the `txBox="1"` the text-box builder
  already writes.
- The effective-properties guide records both rungs.

Tests: a new hand-authored `tests/fixtures/text_levels.pptx` in which every tier owns a facet no other
tier touches — nine body levels' worth of structure with `a:lvl5pPr` deliberately absent, a layout
overriding only two levels, a shape-level `a:lstStyle` no public setter can author, a footer, a text
box and a plain autoshape. `crates/mjx-pptx/tests/paragraph_hierarchy.rs` pins fifteen rungs against
it; the fixture is registered in the `mjx-opc` round-trip suites and in the LibreOffice open canary.
`layouts.pptx` is untouched.

## [0.0.57] - 2026-07-31

The effective-properties guide (MJX-23). Ten `effective_*` readers had shipped and nothing explained
the idea behind them: the knowledge was spread across ten per-method doc comments and the frozen
`docs/*_HANDOFF.md` files, which are history rather than user documentation. Documentation only — no
behaviour, no API change.

`mjx-pptx`:

- New guide at `crates/mjx-pptx/docs/effective_properties.md`, pulled in with `include_str!` on a
  documentation-only `effective_properties` module, so it reads as prose on a source host and renders
  as its own page in `cargo doc`. It covers: *what a file states* versus *what a renderer shows*; the
  one candidate walk every shape resolver is built on, and why a shape that is not a placeholder
  inherits nothing; the three-source ladder fill, outline and effects share; why a transform is
  inherited whole while text merges tier by tier; the seven text tiers and the level axis cutting
  across them; the shorter table-cell ladder (MJX-33's `effective_cell_*` trio); why colours bake to
  concrete `RRGGBB`; every stop condition, including why text answers with an empty spec where a fill
  answers `None`; and what one read costs.
- Each of the ten readers gains a link to the guide. Their own doc comments stay authoritative for
  their own ladders and stop conditions.

Also: the workspace README grows a guides list, and the `mjx-ooxml` facade — the crate its own docs
name as the entry point for reading the docs — grows a Guides section.

## [0.0.56] - 2026-07-30

Cell 3-D review and direct-cell authoring (MJX-109, closing the last code follow-up of MJX-38). D4
(MJX-100) left `Cell3D` with two material accessors pending a decision and gave a typed 3-D surface
only to the table-*style* cell3D (`a:tcStyle > a:cell3D`); a direct cell's `a:tcPr > a:cell3D` had
none. Both are settled here.

`mjx-dml`:

- `Cell3D::material` (typed) and `Cell3D::preset_material` (raw wire token) are kept as a deliberate
  pair — the typed accessor is the normal path and mirrors `Shape3D::material`; the raw one is an
  escape hatch for a producer value outside `ST_PresetMaterialType`. Docs rewritten to say so; no API
  change.
- `TableCellProperties` gains typed `cell_3d()` / `set_cell_3d()`, the direct-cell counterpart of
  `TableStyleCellStyle`'s, reusing the same `Cell3D` model and honoring `CT_TableCellProperties`
  schema order (`cell3D` after the borders, before the fill).

`mjx-pptx`:

- `CellFormat` gains `with_cell_material` / `with_cell_bevel` / `with_cell_light_rig`, mirroring
  `TableStyleFormat`. `format_cells` now authors a direct cell's `a:cell3D`; any facet set gives the
  cell a `cell3D` with the schema-required bevel. Additive, non-breaking.

- `docs/CUSTOM_GEOMETRY_HANDOFF.md` records the four shipped atoms (CG1–CG4), the design decisions,
  the verified schema, the known follow-ups (chiefly a guide-formula evaluator), and the 3-D audit
  that found `a:scene3d` / `a:sp3d` already complete — so MJX-44's opaque-geometry gap is closed.

## [0.0.54] - 2026-07-30

Custom geometry, the PowerPoint surface (MJX-44 CG4). The `mjx-dml` custom-geometry model (CG1–CG3)
now reaches `.pptx`: one accessor reads and writes both preset and custom geometry.

`mjx-pptx`:

- New `Geometry` enum — `Preset(ShapeGeometry)` | `Custom(CustomGeometrySpec)` | `Inherited`.
- **Breaking:** `Presentation::shape_geometry` now returns `Geometry` (was `ShapeGeometry`), and
  `set_shape_geometry` / the cursor's `.geometry(..)` now take a `Geometry` (was `ShapeGeometry`).
  Migrate a preset call by wrapping it: `Geometry::Preset(ShapeGeometry::…)`. `shape_geometry` no
  longer errors when a shape declares no geometry — it returns `Geometry::Inherited` — so
  `PptxError::ShapeHasNoGeometry` is no longer produced by these methods.
- `shape_geometry` now reads `a:custGeom` (as `Geometry::Custom`) as well as `a:prstGeom`;
  `set_shape_geometry` writes either, converts between them (the two are mutually exclusive), and for
  `Geometry::Inherited` removes the shape's own geometry element so an inherited one takes over.

Pre-`v0.1`, so the API is still unstable; this is the deliberate unification MJX-44 called for.

## [0.0.53] - 2026-07-30

Custom geometry, the container and auxiliary lists (MJX-44 CG3). Completes the `mjx-dml` model of
`a:custGeom` — the path list (CG2) now sits inside the whole `CT_CustomGeometry2D`, with its guides,
adjust handles, connection sites, and text rectangle.

`mjx-dml`:

- `CustomGeometry` (`a:custGeom`, `CT_CustomGeometry2D`) — a fidelity wrapper reading every child
  typed (`adjust_values`/`guides`/`adjust_handles`/`connection_sites`/`text_rectangle`/`paths`) and
  round-tripping byte-for-byte (an unmodeled child such as `extLst` re-emits verbatim).
- Interner-free value types: `GuideSpec` (`a:gd` name + formula), `AdjustHandle` (`a:ahXY` / `a:ahPolar`
  with their `gdRef*` / min / max bounds), `ConnectionSite` (`a:cxn` angle + position), and
  `Rectangle` (`a:rect` edges).
- `CustomGeometrySpec` + `to_custom_geometry` — the interner-free read/author surface; builds children
  in schema order, omits empty auxiliary lists, always writes the required `a:pathLst`.

Additive and non-breaking.

## [0.0.52] - 2026-07-30

Custom geometry, the path list (MJX-44 CG2). The drawing commands a freeform `a:custGeom` is traced
from — the render-critical core, on top of the CG1 value types.

`mjx-dml`:

- `Path2DList` (`a:pathLst`, `CT_Path2DList`) and `Path2D` (`a:path`, `CT_Path2D`) — fidelity wrappers
  that read their paths / flags typed and round-trip byte-for-byte (an unmodeled child re-emits
  verbatim). `Path2D` exposes `width`/`height`/`fill`/`stroke`/`extrusion_ok` (each `None` when
  unstated, distinct from the schema default) and `commands`.
- `DrawCommand` — the interner-free, ordered instruction a renderer follows: `MoveTo`, `LineTo`,
  `ArcTo { width_radius, height_radius, start_angle, swing_angle }`, `QuadBezierTo`, `CubicBezierTo`,
  `Close` (the `a:path` choice group `close`/`moveTo`/`lnTo`/`arcTo`/`quadBezTo`/`cubicBezTo`).
- `Point` — an interner-free `(x, y)` of `AdjustCoordinate`s; `AdjustPoint::value` resolves one.
- `Path2DSpec` (with `to_path_2d`) and `Path2DList::new` / `paths` / `specs` — the read/author surface.

Additive and non-breaking.

## [0.0.51] - 2026-07-30

Custom geometry, foundation types (MJX-44 CG1). Groundwork for a typed surface over `a:custGeom`
(`CT_CustomGeometry2D`) — the freeform path list a hand-drawn PowerPoint shape uses, until now
preserved only opaquely. This iteration adds the value types every piece of a custom geometry is
expressed in; the path list, guide/handle/connection lists, and the pptx accessor follow.

`mjx-ooxml-types`:

- Generated `PathFillMode` (`ST_PathFillMode`: `none`/`norm`→`Normal`/`lighten`/`lightenLess`/
  `darken`/`darkenLess`) — how a freeform `a:path` is filled (`a:path@fill`). Added to the DrawingML
  codegen allowlist.

`mjx-dml`:

- `AdjustCoordinate` (`ST_AdjCoordinate`) and `AdjustAngle` (`ST_AdjAngle`) — each a union of a
  numeric literal (`Emu` / `Angle`) and a geometry-guide reference by name (`Guide`), the two forms a
  custom-geometry coordinate or angle can take.
- `AdjustPoint` (`a:pt` / `a:pos`, `CT_AdjPoint2D`) — the `(x, y)` a path command, adjust handle, or
  connection site is drawn through; a fidelity leaf that reads its coordinates typed and round-trips
  byte-for-byte. Re-exported alongside `PathFillMode` from the crate root.

Additive and non-breaking.

## [0.0.50] - 2026-07-30

Inaccessible external sources — audio/video media (MJX-201 P4, **completing MJX-201**). A slide can
reference audio or video that lives online/externally and is unreachable on another platform. Every
media carrier — `a:videoFile`/`a:audioFile@r:link`, the `a14:media` fallback, `p:snd`/`p:sndTgt`
timing/transition sounds — resolves through a media-typed relationship in the slide's `.rels`, so a
media reference is neutralized by redirecting that relationship.

`mjx-pptx`:

- `Presentation::replace_media_with_placeholder` inserts a placeholder media part and retargets the
  relationship at it (`mjx_opc::Package::retarget_relationship`), so every carrier that named it
  resolves inside the package; the poster image is untouched. The placeholder is caller-supplied bytes
  or a built-in one matching the kind — `default_placeholder_audio()` (a minimal valid silent WAV) or
  `default_placeholder_video()` (a minimal structurally valid MP4 with an empty video track). A
  non-media relationship yields the new `PptxError::NotAMediaReference`.
- `Presentation::media_references` lists a surface's audio/video/media relationships (by id, with kind,
  target, and whether external) — the discovery surface for what to replace. `MediaKind` and
  `MediaReference` are the reported types.

Additive and non-breaking.

## [0.0.49] - 2026-07-30

Inaccessible external sources — OLE objects (MJX-201 P3). An OLE object can reference embedded (or
linked/external) data that is unreachable on another platform. Unlike a chart, an OLE object has no
cached fallback — but it is displayed via its snapshot image and its data stream is read only on
activation, so it is neutralized by redirecting the reference to an in-package placeholder.

`mjx-pptx`:

- `Presentation::replace_ole_object_with_placeholder` inserts a placeholder object part and retargets
  the OLE frame's data relationship at it (`mjx_opc::Package::retarget_relationship`, this feature's
  first consumer), so the object resolves inside the package. The placeholder is caller-supplied bytes
  or the new `default_placeholder_ole()` — a minimal but structurally valid MS-CFB compound file (an
  empty root storage). The `p:oleObj` markup is untouched; a replaced embedded part is left
  unreferenced and can be swept with `Package::remove_unreferenced_parts`. A non-OLE shape yields the
  new `PptxError::ShapeIsNotAnOleObject`.
- `Presentation::ole_objects` lists the OLE frames on a surface, each with its data target, `progId`,
  and whether the reference is external — the discovery surface for what to replace.

Additive and non-breaking. Next: audio/video media (P4).

## [0.0.48] - 2026-07-30

Inaccessible external sources — chart backing workbook (MJX-201 P2). A chart can reference a workbook
that lives online/externally; that reference can be unreachable on another platform. A chart renders
entirely from its cached data (`c:numCache`/`c:strCache`), so the workbook is only needed to *edit* the
data — which means the reference can simply be detached.

`mjx-pptx`:

- `Presentation::detach_chart_workbook` removes a chart's `c:externalData` reference — the element and
  its relationship — leaving the chart to render from its cache (the same cache-only shape a freshly
  authored chart has). An embedded workbook part is left unreferenced and can be swept with
  `Package::remove_unreferenced_parts`. A non-chart shape yields `PptxError::ShapeIsNotAChart`; a chart
  with no backing workbook yields the new `PptxError::ChartHasNoExternalData`.
- `Presentation::chart_workbooks` lists the charts on a surface that reference a workbook, each with its
  target and whether the reference is external — the discovery surface for what to detach.

Additive and non-breaking. Follow-up phases extend to OLE objects and media, where (unlike charts)
there is no cached fallback and the P1 redirect-to-placeholder is used.

## [0.0.47] - 2026-07-30

Inaccessible external sources — foundation + linked-image placeholder (MJX-201 P1, spun out of MJX-42).
Many element sources can be external/online (linked images, a chart's backing workbook, OLE, media),
and an unreachable target can crash a consumer. This begins the caller-driven capability to neutralize
one by substituting an in-package placeholder of the same kind; the library does no external I/O, so
the caller decides which references are inaccessible.

`mjx-opc` gains the general redirect lever:

- `Package::external_relationships` lists every `TargetMode::External` relationship (with its owning
  part) — the discovery surface for what might be unreachable.
- `Package::retarget_relationship` repoints a relationship at a new target/mode while keeping its id
  and its `.rels` position (editing the control tree and the navigation view in tandem). The recipe:
  `insert_part` a placeholder, then retarget the external relationship at it as `Internal` — so the
  binding element resolves in-package without touching its own markup, which is what the many unmodeled
  element kinds need.

`mjx-pptx` applies it to images (the one modeled kind, via element rewrite):

- `Presentation::replace_linked_image_with_placeholder` embeds a placeholder — caller-supplied bytes or
  the new `DEFAULT_PLACEHOLDER_IMAGE` — into a picture that links an external image, rewriting
  `@r:link` → `@r:embed` and dropping the dangling link relationship. An embedded picture yields
  `PptxError::PictureImageNotLinked`.
- `Presentation::linked_images` lists the linked pictures on a surface (with their targets) so callers
  need not walk the shapes.

Additive and non-breaking. Follow-up phases extend the same redirect to the chart workbook, OLE, and
media.

## [0.0.46] - 2026-07-30

Linked images become addressable (MJX-42, second of two package-gap fixes). A picture that *links* its
image (`p:blipFill > a:blip@r:link`) rather than embedding it was invisible to the API:
`picture_image_rel_id` read only `@r:embed` and returned `None`, so a linked image could not be
reached even though it round-tripped fine.

- `Presentation::picture_image_rel_id` now falls back to the link id, returning whichever relationship
  binds the image (embed preferred when both are present).
- New `Presentation::picture_image_link_target` returns where a linked image points — the relationship
  target string, external path/URL or in-package part alike — so a linked image is fully addressable.
- `picture_image_bytes` consequently reaches linked images: an embedded image or an internal link
  resolves to bytes; an external link reports `PptxError::ExternalTarget` (its bytes live outside the
  package).

Additive and non-breaking — an embedded picture reads exactly as before. Also elides a needless
lifetime flagged by newer stable clippy and unwraps single-literal `concat!`/drops unused imports in
`mjx-dml` tests, keeping the workspace clippy-clean under the current toolchain.

## [0.0.45] - 2026-07-30

Orphaned-part sweep (MJX-42, first of two package-gap fixes). Replacing an image, deleting a slide, or
any edit that unwires a relationship can leave a part with nothing pointing at it — a legal but dead
media blob. Until now nothing removed them; `remove_part_cascading` only walks downward from one named
part.

New `Package::remove_unreferenced_parts` on `mjx-opc` is the package-wide garbage collector. It returns
the swept part names and is conservative by construction: a part survives if it is reachable by
following `Internal` relationships transitively from the package root (`_rels/.rels`), so a media part
reached only through a live slide stays, and OPC-required roots (core properties, thumbnail) stay
because the root relationships name them. Control parts are never removed — `[Content_Types].xml` is
not a part, and every `.rels` part is spared. Reference cycles terminate.

The relationship-resolution logic shared by the reachability walk and the existing reference checks is
unified behind one `resolve_rel` helper (root-vs-part base).

## [0.0.44] - 2026-07-29

Run coalescing (MJX-41, third and last text-model gap — **completing MJX-41**) — formatting a
sub-range with `set_text_range_properties` splits a run, and repeatedly formatting overlapping ranges
leaves a paragraph with more runs than it needs. Nothing merged them back; now an explicit pass does.

New `Presentation::coalesce_paragraph_runs` and `coalesce_shape_runs` merge adjacent runs that would
render identically, returning the number of runs merged away. Two adjacent runs merge only when
**both** hold, so the paragraph reads exactly the same afterwards:

- their **effective** formatting is identical — resolved through the full inheritance ladder, so a run
  that sets a property explicitly merges with a neighbour that inherits the same value (this compares
  meaning, not raw XML); and
- neither carries distinguishing state this model does not describe — a hyperlink, an `rtl`, an
  `a:extLst`, a foreign attribute — so nothing is dropped by the merge (`dirty`/`err`/`smtClean`
  housekeeping is ignored and never blocks a merge).

A line break or field between two runs keeps them apart. When nothing merges, the call changes nothing
and does not dirty the part.

```rust
let merged = pres.coalesce_paragraph_runs(surface, shape, para)?;   // runs removed
let total = pres.coalesce_shape_runs(surface, shape)?;              // across the whole body
```

The supporting pieces are in `mjx-dml`: `CharacterProperties::unmodeled_state_eq` /
`has_only_modeled_state` (the safety gate) and `Paragraph::coalesce_adjacent_runs` (the content-vec
merge). Every part still round-trips byte-for-byte.

## [0.0.43] - 2026-07-29

`a:br` / `a:fld` addressability (MJX-41, second of three text-model gaps) — a line break (`a:br`) and
a text field (`a:fld`) are paragraph children like a run, but until now both fell into the opaque
`Raw` bucket, so a slide-number or date field's text could not be read and a break could not be
located.

Both are now typed: new `TextLineBreak` (`CT_TextLineBreak`, an optional `a:rPr`) and `TextField`
(`CT_TextField` — `@id`/`@type` and optional `a:rPr`/`a:pPr`/`a:t`) fidelity wrappers, added as
`ParagraphContent::LineBreak` / `Field` variants. Following the decision recorded on the issue, they
get **their own accessors** rather than joining the run index space, so this is **non-breaking** —
`runs()`, run indices, and `Paragraph::text()` are unchanged. New `Paragraph::line_breaks()` /
`fields()` (and `_mut`) enumerate them; `TextField::text()` reads the field's cached rendering.

`mjx-pptx` gains a read surface mirroring `run_text`/`run_count` — `paragraph_field_count`,
`paragraph_field_text`, and `paragraph_field_type` on `Presentation` — so a field's cached value and
kind are readable at the format level (a new `PptxError::FieldIndexOutOfRange` reports a bad index).

```rust
let count = pres.paragraph_field_count(surface, shape, para)?;
let text = pres.paragraph_field_text(surface, shape, para, 0)?;   // e.g. "1/27/13"
let kind = pres.paragraph_field_type(surface, shape, para, 0)?;   // e.g. Some("datetimeFigureOut")
```

Every part still round-trips byte-for-byte; reading a field dirties nothing.

## [0.0.42] - 2026-07-29

Underline line/fill groups (MJX-41, first of three text-model gaps) — the underline line group
(`a:uLn` / `a:uLnTx`) and fill group (`a:uFill` / `a:uFillTx`) on a run's `a:rPr` now have a typed
surface, so an underline can be recoloured and restyled independently of the text it sits under.
Previously both were preserved opaquely with no way to read or set them.

Each group is a three-state choice — unset (inherited), *follow text* (the marker element), or an
explicit value — modeled as `UnderlineLine` / `UnderlineFill`. The explicit forms reuse the existing
line and fill models (`LineSpec` for `a:uLn`, `FillSpec` for `a:uFill`), and the two members of a
group are mutually exclusive: writing one replaces the other in place. The groups flow through the
whole run-formatting surface for free — `CharacterPropertiesSpec` builders (`with_underline_line` /
`with_underline_fill`), `merge_under`, `set_text_range_properties`, and `effective_run_properties`
(where the colours are baked like any other fill or outline).

```rust
use mjx_dml::{CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth, UnderlineFill,
    UnderlineLine};

let spec = CharacterPropertiesSpec::new()
    .with_underline_line(UnderlineLine::Explicit(LineSpec::solid(
        LineWidth::from_points(1.0),
        ColorSpec::Srgb("FF0000".into()),
    )))
    .with_underline_fill(UnderlineFill::FollowText);
```

Additive and non-breaking; every untouched part still round-trips byte-for-byte.

## [0.0.41] - 2026-07-29

Ink (MJX-138, third and last tier of MJX-135) — **preserve-first** recognition of legacy ink (InkML)
content parts, **completing MJX-135**. Handwriting ink is carried as an InkML part
(`/ppt/ink/inkN.xml`, `application/inkml+xml`) referenced from the shape tree by a `p14:contentPart`.
Producers wrap that reference in `mc:AlternateContent` — a shape-tree child in the Markup-Compatibility
namespace that the shape index space cannot reach — so, like VML, ink is recognized by its content type
rather than navigated from a shape. The InkML markup is carried through a round-trip verbatim, not
modeled. Unconditional, like the OLE and ActiveX tiers.

```rust
use mjx_pptx::Presentation;

let deck = Presentation::open(&bytes)?;
for part in deck.ink_part_names() {
    let inkml = deck.ink_part_bytes(&part); // raw InkML, verbatim
}
```

### Added

- **`Presentation::ink_part_names`** — every InkML part in the package, recognized by content type.
- **`Presentation::ink_part_bytes`** — an ink part's bytes, verbatim and non-dirtying.
- Constants `REL_INK` (the shared `customXml` relationship type) and `CONTENT_TYPE_INKML`.

### Scope

Recognition + preserve + a read window only — no authoring, and the ink is not modeled (a typed stroke
surface, trace points → paths, is deferred). Per-shape association (`p14:contentPart@r:id`) and the
`mc:Fallback` snapshot are deferred with it. **MJX-135 (OLE / ActiveX / Ink) is now complete**;
producer-authentic fixture validation across all three tiers is a follow-up (MJX-140).

## [0.0.40] - 2026-07-26

ActiveX controls (MJX-137, second tier of MJX-135) — **preserve-first** recognition of legacy ActiveX
form controls. Unlike an OLE object (a graphic frame in the shape tree), a control lives in
`p:cSld > p:controls > p:control` — beside the shape tree — so it is addressed per-slide by a control
index. Its persisted state is a **two-hop** chain: `p:control@r:id` names the control part
(`/ppt/activeX/activeXN.xml`, `ax:ocx` markup), which in turn relates to its binary blob
(`/ppt/activeX/activeXN.bin`). The control markup, its binary, and its fallback snapshot image are each
carried through a round-trip verbatim, none modeled. Unconditional, like OLE.

```rust
use mjx_pptx::Presentation;

let mut deck = Presentation::open(&bytes)?;
for i in 0..deck.activex_control_count(slide)? {
    let name = deck.activex_control_name(slide, i)?;          // e.g. "CommandButton1"
    let ocx = deck.activex_part_bytes(slide, i)?;             // ax:ocx markup, verbatim
    let blob = deck.activex_binary_bytes(slide, i)?;          // persisted state (.bin), two-hop
    let snapshot = deck.activex_snapshot_image_bytes(slide, i)?; // fallback image for rendering
}
```

### Added

- **`Presentation::activex_control_count`** — the number of ActiveX controls on a surface.
- **`Presentation::activex_control_rel_id` / `activex_control_name`** — a control's control-part
  relationship id and its declared `name`.
- **`Presentation::activex_part_bytes`** — the `ax:ocx` control part's verbatim bytes.
- **`Presentation::activex_binary_bytes`** — the control's binary blob, resolved across the two-hop
  `activeXControlBinary` chain.
- **`Presentation::activex_snapshot_rel_id` / `activex_snapshot_image_bytes`** — the fallback snapshot
  image a renderer draws in place of the (never-executed) control.
- Constants `REL_CONTROL`, `REL_ACTIVEX_CONTROL_BINARY`, `CONTENT_TYPE_ACTIVEX`,
  `CONTENT_TYPE_ACTIVEX_BINARY`.

### Scope

Recognition + preserve + a read window only — no authoring, and the control is not modeled (opaque
`ax:ocx` markup + binary state). The last MJX-135 tier is ink (MJX-138); producer-authentic fixture
validation is a follow-up (MJX-140).

## [0.0.39] - 2026-07-26

OLE objects (MJX-136, first tier of MJX-135) — **preserve-first** recognition of legacy embedded OLE
objects. An OLE object is an embedded document (a legacy `.xls`/`.doc` or an OLE `.bin` stream)
referenced from a `p:graphicFrame` via `p:oleObj@r:id`, drawn from a fallback image snapshot. Such a
frame previously surfaced only as the opaque `GraphicFrameKind::Other`; it now reads as `OleObject`,
with accessors for the embedded object's bytes and the snapshot image — both carried through a
round-trip verbatim, neither modeled. Unlike VML this is **not** feature-gated: an OLE frame is ordinary
PresentationML, so it mirrors the (unconditional) chart surface.

```rust
use mjx_pptx::{GraphicFrameKind, Presentation};

let mut deck = Presentation::open(&bytes)?;
if deck.graphic_frame_kind(slide, shape)? == Some(GraphicFrameKind::OleObject) {
    let prog = deck.ole_prog_id(slide, shape)?;               // e.g. "Excel.Sheet.12"
    let object = deck.ole_object_part_bytes(slide, shape)?;   // embedded object, verbatim
    let snapshot = deck.ole_snapshot_image_bytes(slide, shape)?; // fallback image for rendering
}
```

### Added

- **`GraphicFrameKind::OleObject`** — a graphic frame framing a `p:oleObj` (refines the former `Other`).
- **`Presentation::ole_object_rel_id` / `ole_object_part_bytes`** — the embedded object's relationship
  and its verbatim bytes (`/ppt/embeddings/oleObjectN.bin` or an embedded package).
- **`Presentation::ole_snapshot_rel_id` / `ole_snapshot_image_bytes`** — the fallback snapshot image a
  renderer draws in place of the (never-executed) object.
- **`Presentation::ole_prog_id`** — the owning application's `progId`.
- Constants `REL_OLE_OBJECT`, `REL_PACKAGE`, `CONTENT_TYPE_OLE_OBJECT`.

### Scope

Recognition + preserve + a read window only — no authoring, and the embedded object is not modeled
(it is an opaque OLE stream or embedded document). The `p:oleObj` is reached through its
`mc:AlternateContent` wrapper (preferring the `mc:Choice` branch) by a bounded structural descent, not
by running full MCE resolution. The remaining MJX-135 tiers are ActiveX controls (MJX-137) and ink
(MJX-138); producer-authentic fixture validation is a follow-up (MJX-140).

## [0.0.38] - 2026-07-26

VML, tier V1 (MJX-115) — **preserve-first** legacy VML round-trip. VML is the Transitional-only drawing
markup producers still emit for OLE-object fallbacks, comment shapes, ink and legacy controls, carried
as standalone `vmlDrawingN.vml` parts. Such parts already round-trip byte-identically through the
generic part-level copy-on-write; this release adds a **recognition surface** so callers can find and
read them — behind the new `vml` crate feature (opt-in, off by default). The VML XML is **not modeled**.

```rust
// with `mjx-pptx` (or `mjx-ooxml`) built with the `vml` feature
let deck = Presentation::open(&bytes)?;
for part in deck.vml_part_names() {
    let xml = deck.vml_part_bytes(&part); // raw legacy VML, verbatim
}
```

### Added

- **`mjx-vml`** becomes real (was a scaffold stub): the VML vocabulary — `CONTENT_TYPE_VML`,
  `REL_VML_DRAWING`, `VML_DEFAULT_EXTENSION` — and an `is_vml_content_type` recognition predicate.
- **`Presentation::vml_part_names`** / **`vml_part_bytes`** (behind the `vml` feature) — enumerate the
  legacy VML drawing parts a package carries (by content type, so VML referenced from any part is
  found) and read a part's bytes verbatim, without dirtying anything.
- **`vml` Cargo feature** on `mjx-pptx` (the repo's first), re-exposed by the `mjx-ooxml` facade.

### Scope

Preserve-first only: VML is recognized and readable as raw bytes, never parsed or modeled, and never
authored. Recognition is package-level (content type), not yet shape/relationship-level — the OLE /
ActiveX / ink references that cite a specific VML shape are the next tier (MJX-135), and Word-side
legacy VML (`w:pict`, header/footer fallback) is tracked under the Word slice (MJX-139). The fixture is
hand-crafted; validation against genuine producer decks is a follow-up (MJX-140). This completes the
chart + VML arc (MJX-47) except for the chart embedded workbook (MJX-116).

## [0.0.37] - 2026-07-25

Charts, tier C4 (MJX-114) — **authoring** a brand-new chart. C0–C3 recognized, modeled and edited an
existing chart; this release creates one from scratch. A chart is described fluently with `ChartData`
(a kind, shared categories, named series) and added to a slide with `Presentation::add_chart`, which
writes a new chart part (`ppt/charts/chartN.xml`) and a `p:graphicFrame` that references it. All six
kinds are supported: bar, line, area, pie, doughnut and scatter.

```rust
use mjx_pptx::{ChartData, ChartKind, ShapeBounds};

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("Revenue", [10.0, 20.5, 15.0])
    .series("Cost", [5.0, 8.0, 7.25]);
let shape = deck.add_chart(slide, &chart, ShapeBounds::from_inches(1.0, 1.0, 6.0, 4.0))?;
```

### Added

- **`Presentation::add_chart`** — authors a chart on a surface from a `ChartData`, returning its shape
  index. Creates the chart part with its `CONTENT_TYPE_CHART` Override and a `REL_CHART` relationship
  from the slide; every pre-existing part stays byte-identical.
- **`ChartData`** (re-exported from `mjx-pptx`, alongside `ChartKind`) — a fluent builder
  (`new(kind).categories(...).series(name, values)`) that serializes a complete `c:chartSpace` part.
- Error **`InvalidChartData`** — a chart with no series (or only empty series) is refused at creation.

### Scope

Authoring writes **cached data only** (`c:strCache`/`c:numCache`, with synthesized `c:f` formulas so
the references are schema-valid) and **no embedded workbook**: the chart renders everywhere from its
cache, while PowerPoint's "Edit Data" is degraded until the embedded-workbook follow-up (MJX-116).
Scatter's shared categories become numeric X values, falling back to the point position for a
non-numeric label. This completes the chart arc except for VML (V1) and the embedded workbook.

## [0.0.36] - 2026-07-25

Charts, tier C3 (MJX-113) — the first **mutating** chart tier. C1/C2 modeled a chart read-only; this
release rewrites a series' cached values and category labels (`c:numCache` / `c:strCache`) on an
existing chart, through a `mjx-pptx` surface. The cached values are what **render**; a chart's
embedded workbook is **not** rewritten and goes stale (a separate follow-up). Only the edited chart
part is dirtied — every other part, the embedded workbook included, is left byte-identical.

```rust
// read the series, then rewrite the first series' values
for s in deck.chart_series(slide, shape)? {           // name, categories, values per series
    println!("{:?}: {:?} = {:?}", s.name, s.categories, s.values);
}
deck.set_chart_series_values(slide, shape, 0, &[1.0, 2.5, 3.0])?;
deck.set_chart_series_categories(slide, shape, 0, &["Q1", "Q2", "Q3"])?;
```

### Added

- **`Presentation::chart_series`** — each series of a chart as a `ChartSeriesData` (`name`,
  `categories`, `values`; a scatter series' `xVal`/`yVal`), flattened across the chart's plots.
  Non-dirtying.
- **`Presentation::set_chart_series_values` / `set_chart_series_categories`** — rewrite the cached
  data of the `series_idx`-th series (0-based across the plots), dirtying only the chart part.
  `set_chart_series_values` targets `c:val`, or a scatter series' `c:yVal`.
- **`ChartSeriesData`** — the read DTO.
- **`mjx-chart` mutation** — `NumberCache::set_values` / `StringCache::set_labels`, the
  reference/data-source `set_values`/`set_labels`, `Series::set_values`/`set_categories`, and the
  mutable navigation (`series_mut`, `all_series_mut`, `ChartSpace::series_mut`/`series_count`).
- Errors **`ShapeIsNotAChart`**, **`ChartSeriesOutOfRange`**, **`ChartSeriesNotEditable`**.

### Fidelity

A cache edit rebuilds only its `c:pt` points and its `c:ptCount`; the `c:formatCode` and everything
outside the edited cache (the axes, other series, styling) survive verbatim. A rewritten number is
formatted with Rust's shortest round-trip representation (the exact inverse of the read parse); a
non-finite value, which has no valid spelling, is skipped. `mjx-pptx` gains a dependency on
`mjx-chart` (both shared-markup/format tiers, cycle-free).

## [0.0.35] - 2026-07-25

Charts, tier C2 (MJX-112) — the remaining common plot types. C1 modeled the bar plot; this release
extends the same read-only, byte-identical model to **line** (`c:lineChart`), **pie** (`c:pieChart`),
**area** (`c:areaChart`), **scatter** (`c:scatterChart`) and **doughnut** (`c:doughnutChart`), and to
**combo charts** — a `c:plotArea` may legitimately hold more than one plot.

```rust
use mjx_ooxml_core::FromXml;

let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;
let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
for kind in space.chart_kinds() {          // e.g. [Bar, Line] for a combo chart
    println!("{kind:?}");
}
if let Some(scatter) = space.plot_area().and_then(|p| p.scatter_chart()) {
    for series in scatter.series() {
        let xs = series.x_data().map(|x| x.values());   // c:xVal, not c:cat
        let ys = series.y_data().map(|y| y.values());   // c:yVal, not c:val
    }
}
```

### Added

- **Plot types** — `LineChart`, `PieChart`, `AreaChart`, `ScatterChart`, `DoughnutChart` alongside the
  existing `BarChart`, each with `series()`/`series_at()`/`series_count()`/`kind()`. `ChartKind` gains
  `Line`, `Pie`, `Area`, `Scatter`, `Doughnut`.
- **`PlotArea` accessors** — `line_chart()`/`pie_chart()`/`area_chart()`/`scatter_chart()`/
  `doughnut_chart()` beside `bar_chart()`; `chart_kinds()` (one entry per plot, for combo charts) and
  `all_series()` (every plot's series, flattened). `ChartSpace::chart_kinds()` mirrors it.
- **Scatter data** — `Series::x_data()` (`c:xVal`) and `y_data()` (`c:yVal`), plus
  `CategoryData::values()` (the numeric companion to `labels()`), for the one series type that carries
  X/Y data instead of `c:cat`/`c:val`.

### Fidelity

Each plot type is its own struct but they share one `Series` type and one `PlotContent` bucket; every
plot preserves its own element name and buckets its type-specific scalars (`barDir`, `grouping`,
`firstSliceAng`, `holeSize`, `scatterStyle`) and axes into `Raw`, so a chart of any modeled type — or
a combo — round-trips byte-for-byte. Unmodeled plot types (radar, bubble, 3-D, …) ride through `Raw`.

## [0.0.34] - 2026-07-25

Charts, tier C1 (MJX-111) — the chart XML gets a typed home. C0 recognized a chart frame and handed
back the chart part's raw bytes; this release **models** that part in `mjx-chart` (until now a
scaffold stub). It derives the chart-space spine `c:chartSpace → c:chart → c:plotArea` and one plot
type end to end — the bar/column plot (`c:barChart` / `c:ser` / `c:cat` / `c:val`) — with read-only
accessors for a chart's kind, its series, and each series' category labels and values, read down
through the `c:strCache` / `c:numCache`.

```rust
use mjx_ooxml_core::FromXml;

let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;      // the /ppt/charts/chartN.xml bytes
let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
if let Some(bar) = space.bar_chart() {                       // c:chart → c:plotArea → c:barChart
    for series in bar.series() {
        let name = series.name();                            // "Sales", from c:tx
        let labels = series.categories().map(|c| c.labels()); // ["North", "South", "West"]
        let values = series.values().map(|v| v.values());     // [19.2, 21.4, 16.7]
    }
}
```

### Added

- **`mjx-chart` chart model** — `ChartSpace` (`c:chartSpace`), `Chart`, `PlotArea`, `BarChart`,
  `Series`, and the data layer (`NumericData`/`CategoryData`/`SeriesText`,
  `NumberReference`/`StringReference`, `NumberCache`/`StringCache`, `DataPoint`, `Value`, `Formula`),
  each parsed with `FromXml` and re-emitted byte-for-byte with `ToXml`.
- **Read accessors** — `ChartSpace::{chart, plot_area, bar_chart, chart_kind}`;
  `BarChart::{series, series_at, series_count, direction, grouping}`;
  `Series::{name, categories, values, index, order}`; `CategoryData::labels`, `NumericData::values`,
  and the underlying reference/cache/point accessors. Chart kinds are the extensible `ChartKind`
  enum; a bar plot's `BarDirection` and `BarGrouping` are typed.

### Fidelity

Every modeled container keeps an ordered `content` list of typed children plus a `Raw` catch-all
(mirroring the `mjx-dml` table model), so the axes, text properties, an external-data reference, a
literal data source or an `extLst` this tier does not interpret round-trip byte-for-byte. A cached
value is parsed on demand from its point's preserved wire text — never reformatted on write.

### Scope

Read-only, bar plot only. Cached data (`c:numCache` / `c:strCache`) is the read path; a literal
source (`c:numLit` / `c:strLit`) or a multi-level category rides through the `Raw` bucket for now.
Other plot types are tier C2; editing (C3) and authoring (C4) are later tiers.

## [0.0.33] - 2026-07-25

Charts, tier C0 (MJX-47) — the first step of the chart workstream. A `p:graphicFrame` that frames a
chart (its `a:graphicData@uri` is the chart URI and its payload is a `c:chart`) points at a **separate
part** (`/ppt/charts/chartN.xml`) by relationship id, unlike a table, whose `a:tbl` is inline. This
release recognizes such a frame, resolves that relationship, and reads the chart part's bytes — the
chart XML itself is not modeled yet; it and its satellites (an embedded workbook, colour and style
parts) are carried through a round-trip **verbatim**.

```rust
if deck.graphic_frame_kind(slide, shape)? == Some(GraphicFrameKind::Chart) {
    let rel = deck.chart_rel_id(slide, shape)?;        // the slide relationship the frame names
    let xml = deck.chart_part_bytes(slide, shape)?;    // the /ppt/charts/chartN.xml bytes, borrowed
}
```

### Added

- **`Presentation::chart_rel_id`** — the relationship id a chart frame names
  (`p:graphicFrame > a:graphic > a:graphicData > c:chart@r:id`), or `None` for any shape that frames
  no chart. The `c:chart` element is looked for rather than the frame's `uri` trusted — the payload
  decides. Reading is non-dirtying.
- **`Presentation::chart_part_bytes`** — the raw XML of the chart part that frame references, borrowed
  from the package exactly as stored (never re-serialized), or `None` when the shape frames no chart.
  The read window onto a chart until `mjx-chart` models it.
- **`constants::REL_CHART` and `constants::CONTENT_TYPE_CHART`** — the chart relationship type and the
  chart part's content type, for the authoring tiers to come.
- A `tests/fixtures/charts.pptx` fixture (two slides, one clustered-column chart with an embedded
  workbook) and integration tests proving a chart deck round-trips byte-identically, that reading a
  chart dirties nothing, and that editing another slide leaves every chart part untouched.

### Changed

- The private `image_part_for_rel` helper is generalized to `part_for_rel` (it resolves any
  relationship id to its part), now shared by the image and chart read paths.

## [0.0.32] - 2026-07-24

DrawingML 3-D, part 3 (MJX-49 D4) — and with it the 3-D workstream is complete. `Cell3D`
(`CT_Cell3D`, a table cell's 3-D corner), until now a fidelity wrapper that kept its `a:bevel` /
`a:lightRig` opaque, becomes the **first consumer** of the typed model: it reads and authors them
through the same `Bevel` / `LightRig` the shape surface uses.

```rust
// a header row whose cells stand up in metal, bevelled and lit
deck.format_table_style_part(style_id, TableStylePart::FirstRow,
    &TableStyleFormat::new()
        .with_cell_material(PresetMaterial::Metal)
        .with_cell_bevel(Bevel { width: Some(Emu::from_emu(76_200)), ..Bevel::default() })
        .with_cell_light_rig(LightRig { rig: LightRigType::ThreePoint, direction: LightRigDirection::Top, rotation: None }))?;
```

### Added

- **`mjx-dml`: `Cell3D` decomposed** — typed `material()` (a `PresetMaterial`, alongside the retained
  raw `preset_material()`), `bevel()` and `light_rig()` accessors, and authoring via `Cell3D::new`
  (seeded with the schema-required empty bevel) + `set_material` / `set_bevel` / `set_light_rig`, with
  `TableStyleCellStyle::set_cell_3d` placing the child at its schema rank. The `a:bevel` / `a:lightRig`
  wire helpers are shared with the shape surface; `extLst` stays opaque, so a `cell3D` still
  round-trips byte-for-byte.
- **`mjx-pptx`: cell-3-D on the table-style builder** — `TableStyleFormat::with_cell_material`,
  `with_cell_bevel` and `with_cell_light_rig` give a styled part's cells an `a:cell3D`, applied
  through the shared and inline `tableStyles` paths alike.

## [0.0.31] - 2026-07-24

DrawingML 3-D, part 2 (MJX-49 D3) — the `mjx-pptx` shape surface. The typed 3-D model from 0.0.30
gains its `Presentation` accessors, a 1:1 mirror of the shape-effects surface (E3): a shape's 3-D
scene and its own 3-D properties are now readable, writable and clearable, on a group member as on a
top-level shape.

```rust
deck.set_shape_scene_3d(0, shape, &Scene3DSpec { camera, light_rig })?;   // how it is lit and viewed
deck.set_shape_3d_properties(0, shape, &Shape3DSpec { extrusion_height, bevel_top, .. })?;
deck.shape(0, shape)?.scene_3d(scene).shape_3d_properties(props).apply()?; // or fluently, one commit
```

### Added

- **`Presentation::shape_scene_3d` / `set_shape_scene_3d` / `clear_shape_scene_3d`** — a shape's
  `p:spPr > a:scene3d` (`CT_Scene3D`) as an interner-free [`Scene3DSpec`]. Reading is non-dirtying and
  returns `None` when the shape is flat (3-D has no inheritance chain) or the scene omits a
  schema-required camera/light rig. Setting rebuilds the element in `CT_ShapeProperties` order — after
  any fill, outline and effects, before `a:sp3d`. Clearing **removes** the element (an empty
  `a:scene3d` would be schema-invalid), a no-op when absent.
- **`Presentation::shape_3d_properties` / `set_shape_3d_properties` / `clear_shape_3d_properties`** —
  a shape's `p:spPr > a:sp3d` (`CT_Shape3D`: extrusion, contour, bevels, material, edge colors) as a
  [`Shape3DSpec`]. `a:sp3d` is the last visual property, so it lands after everything else and before
  any `a:extLst`. An unstated attribute reads `None`, not the schema default.
- **`ShapeCursor::scene_3d` / `clear_scene_3d` / `shape_3d_properties` / `clear_shape_3d_properties`**
  — the same edits recorded on the fluent cursor, applied in one commit alongside fill/outline/effects.
- All six flat methods and the cursor take `impl Into<ShapePath>`, so a group member is addressed the
  same as a top-level shape.

## [0.0.30] - 2026-07-24

DrawingML 3-D, part 1 of the workstream (MJX-49 D1+D2) — the `a:scene3d` / `a:sp3d` subsystem, until
now round-tripped opaquely, gains a typed model. Mirrors the effects/outline/fill workstreams:
generated preset enums, then the `mjx-dml` value types and fidelity wrappers. The `mjx-pptx` shape
surface (D3) and the `Cell3D` upgrade (D4) follow.

### Added

- **`mjx-ooxml-types::drawingml`** — five generated preset enums: `BevelPreset` (12),
  `LightRigType` (27), `LightRigDirection` (8), `PresetMaterial` (15) and `PresetCamera` (62). Each
  cryptic token is expanded to a self-explanatory name sourced from the ECMA-376 token (the light
  direction's compass abbreviations, `threePt`/`twoPt`, `dkEdge`, `softmetal`).
- **`mjx-dml`: the 3-D model** — value types `Bevel`, `SphereCoordinates`, `Camera` (preset view +
  field of view + zoom + rotation) and `LightRig`; fidelity wrappers `Scene3D` (`CT_Scene3D`) and
  `Shape3D` (`CT_Shape3D`) with typed accessors and interner-free `Scene3DSpec` / `Shape3DSpec`. The
  camera and light rig, and a shape's bevels, extrusion/contour colors and material, are read typed;
  the rarer `a:backdrop` and any `extLst` stay opaque, so an element round-trips byte-for-byte. Every
  measure is `Option`, so an unstated attribute reads `None`, not the schema default.

## [0.0.29] - 2026-07-24

Group descent, part 4 — **group structure**, and with it the group workstream is complete. A
`p:grpSp` is now addressable, measurable, editable *and* something a caller can make, dissolve and
move shapes through.

```rust
let group = deck.group_shapes(0, &[1.into(), 2.into()])?;  // select these, group them
deck.move_shape_into_group(0, 3, &group)?;                 // and take that one too
deck.set_shape_fill(0, group.child(0), &navy)?;
```

### Added

- **`Presentation::group_shapes(surface, members)`** — wraps sibling shapes in a new group, returning
  its [`ShapePath`]. The group's box is the union of the members' own boxes (how ECMA-376 Part 1
  §L.4.7.4 defines a child bounding box) and its child space is set **identical** to it, so the
  mapping is the identity: the members keep their coordinates exactly, with no rounding anywhere. The
  group takes the earliest member's z-order slot, and the members keep their relative order inside it
  whatever order they were named in.
- **`Presentation::ungroup(surface, group)`** — dissolves a group, returning where its members now
  are. Each keeps its absolute placement, the group's mapping unwound into its own transform.
- **`Presentation::move_shape_into_group` / `move_shape_out_of_group`** — move one shape one level,
  in or out. The shape does not move on screen: its transform is restated for its new coordinate
  system, **mirrors and rotation included**, so joining a scaled, turned or flipped group leaves it
  exactly where it was.
- **`ShapeCursor::into_group` / `out_of_group` / `group_with` / `ungroup`** — the same, said mid-chain
  and following the shape. Each is a **commit point**: it writes what has been recorded so far,
  performs the change, then re-anchors, so no recorded edit is ever applied against a tree it was not
  recorded against.
- **`ShapePath::child` / `parent`** — step down to a member or up to the enclosing group, which is
  how the group returned by `group_shapes` is addressed.
- **`ShapeBounds::union`** — the smallest rectangle containing both.
- `PptxError::GroupNeedsTwoShapes`, `ShapesAreNotSiblings`, `ShapeCannotContainItself`,
  `ShapeHasNoBounds`.

### Notes

There is deliberately **no empty-group constructor**. §L.4.7.4 records that a group with no shapes is
degenerate and produces no visible output, and one with a single shape "has no representational power
beyond that of the one shape" — and an empty group has no honest `chOff`/`chExt` to be given. Every
group these create is well-formed by construction.

## [0.0.28] - 2026-07-24

Group descent, part 3 — a group member now says **where it is on the slide**. Addressing it, styling
it and measuring it are finally the same three things they are for a top-level shape.

### Changed

- **`Presentation::shape_bounds` answers in absolute slide EMU for a group member**, composing every
  enclosing group's child coordinate space instead of returning the member's raw `a:off` / `a:ext`.
  `set_shape_bounds` takes the same absolute rectangle and maps it back, so read and write stay in
  one space. This is a **behaviour change** for nested addresses only — a top-level shape reads and
  writes exactly as before, because composing over no ancestors is the identity. `shape_transform` /
  `set_shape_transform` are untouched and remain the accessors for what the file literally states, in
  the shape's own space. `effective_shape_bounds` / `effective_shape_transform` compose too, after
  resolving placeholder inheritance; the latter is where the composed rotation and mirror flags are
  read, since an axis-aligned `ShapeBounds` cannot hold a rotation.
- The shape cursor's `.bounds(…)` is slide-absolute to match, and runs the same conversion;
  `.transform(…)` still writes verbatim in the shape's own space.

### Added

- **`mjx-dml`: `Transform2D::child_scale` / `child_to_parent` / `parent_to_child`** — one rung of the
  mapping between a group's child coordinate space and its parent's, and its exact inverse.
- `PptxError::ShapeCannotBePlaced`, when an enclosing group states no `a:chOff` / `a:chExt`: there is
  then no mapping to invert, so the member reads as unplaced and the write is refused rather than
  putting the shape somewhere wrong.

### Notes

The composition follows **ECMA-376 Part 1 §L.4.7.4**, not the naive "apply each ancestor transform in
turn": a nested shape is scaled and mirrored by the *product* of its ancestors' factors, rotated by
their *sum*, and translated so its **centre** lands where the whole chain — rotations included — puts
it. A mirrored or rotated group therefore places its members correctly, which composing corners would
not. Round-tripping `set_shape_bounds(shape_bounds(…))` is exact whenever the groups' scales are, and
within a few EMU — millionths of an inch — when they are not.

## [0.0.27] - 2026-07-24

Group descent, part 2 — the **shape cursor**: a shape is addressed once and edited fluently, and a
group is restyled in one expression.

```rust
deck.shape(0, 2)?                                  // the group at top-level index 2
    .effects(shadow)
    .member(0)?.fill(navy).outline(rule)           // its first member
    .sibling(1)?.fill(gold).text("Q3").all_run_properties(bold)
    .apply()?;                                     // one write pass, one dirty part
```

### Added

- **`Presentation::shape(surface, path)` → `ShapeCursor`** — the ergonomic layer over the
  `set_shape_*` methods. Edit methods record intent and return the cursor; `.apply()` consumes it,
  writes every edit in the order it was recorded, and marks the part dirty once. A cursor that is
  never applied changes nothing, so it is `#[must_use]`. Every edit it records is executed by the
  code the mirrored flat method calls — a cursor is a way of *saying* the edits, not a second way of
  doing them.
- **Moving through a group** — `.member(i)` descends into a `p:grpSp`, `.sibling(i)` moves to another
  shape in the same container, `.parent()` steps back out; `.kind()`, `.member_count()` and `.path()`
  say where the cursor is. Each move checks the address as it lands, so a bad one fails where it was
  written. Recorded edits stay bound to the address they were recorded at, so one `.apply()` commits
  work spread over a group and its members.
- **What a cursor records** — the `p:spPr` surface (`fill` / `no_fill`, `outline` / `no_outline`,
  `effects` / `no_effects`, `geometry`, `bounds`, `transform`), `text`, the text-formatting specs
  (`run_properties`, `paragraph_run_properties`, `all_run_properties`, `end_run_properties`,
  `paragraph_properties`, `text_range_properties` and its `_by_grapheme` sibling), the shape's own
  `hyperlink` / `clear_hyperlink`, and a picture's `image`. Hyperlinks on a *run* or a text range are
  addressed by paragraph and run and stay on the flat API.
- **`Presentation::set_shape_text_content(surface, shape, text)`** — replaces a shape's whole text
  with one paragraph per line, each holding one run, so `shape_text` reads back exactly what was
  written. Only the paragraphs are swapped: the body's own `a:bodyPr` and `a:lstStyle` survive, so
  restating a placeholder's text does not disturb how it is laid out.
- **`Presentation::shape_member_count(surface, shape)`** — how many members a group holds (`0` for
  anything that is not a group).
- `PptxError::ShapeIsNotAGroup` and `PptxError::ShapeHasNoParent`, the two ways a cursor move is
  refused.

### Changed

- The per-shape element edits (fill, outline, effects, preset geometry, a picture's blip) moved into
  `slide.rs` as primitives taking an already-resolved shape, so the flat setters and the cursor share
  one implementation each. No behaviour change.

## [0.0.26] - 2026-07-22

Group descent, part 1 — shapes inside a `p:grpSp` are now addressable. Every shape API takes an
address as `impl Into<ShapePath>`: a bare index is a top-level shape (unchanged), and an array
`[group, member, …]` descends into nested groups. A group member can be read, edited and removed
exactly like a top-level shape; `shape_count` still counts only the top level, and a member's
`shape_bounds` are its own `a:off`/`a:ext` in the group's child space (the absolute-rectangle mapping
lands in a later atom). `PptxError::ShapeIndexOutOfRange` now carries the requested `ShapePath` and
the shape count of the container where the address ran out of range.

## [0.0.25] - 2026-07-22

Hyperlinks on runs and shapes — set, read, and clear links, external URLs and slide jumps.

### Added

- **`Hyperlink`** — a resolved link: `Hyperlink::Url(String)` (an external target) or
  `Hyperlink::Slide(usize)` (a jump to another slide in the deck). The relationship indirection
  (`r:id` → external URL or internal slide part) stays inside `Presentation`.
- **`Presentation::run_hyperlink` / `set_run_hyperlink` / `clear_run_hyperlink`** — the click
  hyperlink on a run, read back as a `Hyperlink`; setting adds its relationship (creating the run's
  `a:rPr` if absent), clearing removes the relationship once nothing else in the part still names it.
- **`Presentation::set_text_range_hyperlink`** — links a scalar range, splitting runs at the
  boundaries so exactly the selected text carries the link (one shared relationship).
- **`Presentation::shape_hyperlink` / `set_shape_hyperlink` / `clear_shape_hyperlink`** — the same on
  a shape's own `p:cNvPr > a:hlinkClick`.
- `mjx-dml`: `CharacterProperties` and `TextRun` gain `hyperlink_rel_id` / `set_hyperlink` (the raw
  `a:hlinkClick` accessors the packaging layer drives).

## [0.0.24] - 2026-07-22

Speaker notes, part 2 — the ergonomic notes surface: read, set, and clear a slide's notes.

### Added

- **`Presentation::notes_text(slide)`** — the speaker notes of a slide, read from its notes slide's
  `body` placeholder by kind (the caller never needs the shape index); `None` when the slide has no
  notes.
- **`Presentation::set_notes_text(slide, text)`** — sets the notes, **creating the notes slide on
  demand** (and, when the deck has none, **synthesizing a notes master** for it to follow) with its
  relationships and content-type overrides. Creating a notes slide adds exactly that part, its
  `.rels`, the slide → notes-slide relationship and the override — every pre-existing part stays
  byte-identical.
- **`Presentation::clear_notes(slide)`** — removes a slide's notes slide (and its `.rels` and
  override); the shared notes master and the slide survive. A no-op when the slide has no notes.

This completes MJX-34 — the last feature before the `v0.1` PowerPoint milestone.

## [0.0.23] - 2026-07-22

Speaker notes, part 1 — a notes slide and the notes master become addressable surfaces.

### Added

- **`Surface::Notes(slide)`** and **`Surface::NotesMaster`** — a slide's notes slide carries the same
  `p:cSld > p:spTree` a slide does, so every existing shape, text, fill, outline, effect, transform and
  table method now works on it unchanged, addressed by the slide it belongs to. `Surface::NotesMaster`
  addresses the single notes master every notes slide inherits from.
- Notes text inherits from the notes master's **`p:notesStyle`** exactly as slide text inherits from a
  slide master's `p:txStyles`; `color_map` and `theme` resolve through the notes master too.

### Changed

- `Presentation::surface_part` now returns an owned `PartName` (a notes part is resolved lazily by
  relationship, not stored), simplifying every call site that previously cloned the borrow.

## [0.0.22] - 2026-07-22

Author inline table styles — a lean, self-contained styling path.

### Added

- **`Presentation::set_inline_table_style(surface, shape, &TableStyleDefinition)`** — gives a table its
  own **inline** `a:tableStyle`, replacing any inline or referenced style. The whole look is declared
  up front and travels with the table: no shared `tableStyles.xml` part, relationship, content-type or
  referenced GUID. Plus the incremental **`format_inline_table_style_part`**.
- **`TableStyleDefinition`** — a declarative builder (`with_name` / `with_id` / `with_part`) reusing
  `TableStyleFormat`; the vestigial `styleId` / `styleName` default.
- **`TableProperties::set_inline_style`** (`mjx-dml`) — writes the style as `a:tableStyle` at its rank,
  replacing any `a:tableStyle` / `a:tableStyleId`.

### Notes

- The style resolves and renders through the existing `with_table_style` and `effective_cell_*`
  readers exactly as a shared one does — an inline style is the same `CT_TableStyle`, spelled out on
  the table.
- **Flags stay the caller's job**: a styled part renders only when its `a:tblPr` flag is on
  (`set_table_part`; `add_table` sets `firstRow`/`bandRow`).

## [0.0.21] - 2026-07-22

Table gaps closed — merge-aware formatting, inline styles, accessibility headers, and more.

### Added

- **`Presentation::cell_headers` / `set_cell_headers`** — the accessibility header associations of a
  cell (`a:tcPr > a:headers`), plus `TableCellProperties::headers` / `set_headers` and `TableCell::id`
  in `mjx-dml`.
- **`Presentation::visible_cell_text`** — the text that renders at a position: the cell's own, or its
  merge anchor's when it is covered.
- **`Presentation::graphic_frame_kind`** returning **`GraphicFrameKind`** (`Table` / `Chart` /
  `Diagram` / `Other`) — tells "not a table" from "a graphic not modeled yet"; a chart or diagram
  frame still answers `ShapeIsNotATable` to the table methods.
- **`TableProperties::inline_style`** (`mjx-dml`) — reports a style defined **inline** on the table
  (`a:tableStyle`); `with_table_style` and the effective-formatting resolvers now resolve an inline
  style as well as a referenced one.

### Changed

- **Formatting a cell selection is merge-aware**: `format_cells` / `format_cell_text` /
  `format_cell_paragraphs` skip merge-covered cells (which render nothing), so unmerging restores a
  covered cell's own formatting. Merging and unmerging still reach covered cells; single-cell methods
  addressed by `(row, column)` are unchanged.

## [0.0.20] - 2026-07-22

Effective cell formatting — what a table cell actually renders as. Closes the tables workstream.

### Added

- **`applicable_parts`** and **`TableStyleFlags`** (`mjx-dml`) — the style parts that cover a cell,
  most specific first, per the ECMA-376 §17.7.6 layering (corner cells > first/last column >
  first/last row > row bands > column bands > `wholeTbl`), with banding over data cells only.
- **`Presentation::effective_cell_fill` / `effective_cell_border`** — the fill or border a cell
  renders, resolving the cell's own `a:tcPr`, then the applicable style parts (explicit or a theme
  `fillRef`/`lnRef`), then the theme, colours baked to concrete `RRGGBB`. A border takes the outer
  edge for a rim cell and the interior edge (`insideH`/`insideV`) for one within the table.
- **`Presentation::effective_cell_run_properties`** — a cell's text run resolved down a
  table-specific ladder: the run's own `a:rPr`, the paragraph default, the table style's `a:tcTxStyle`
  for each applicable part (bold / italic / colour), then the presentation `p:defaultTextStyle`.

### Notes

- This is what the modeled `tableStyles.xml` exists for: everything before reported what a file
  *states*; this resolves what a renderer would show. An explicit property on the cell always wins.
- Reading resolves nothing into the file — every effective read leaves the package byte-identical.

## [0.0.19] - 2026-07-22

The `tableStyles.xml` part is modeled, and table styles can be authored and resolved.

### Added

- **The table-style model** (`mjx-dml`) — `TableStyleList`, `TableStyle`, the thirteen part slots
  (`TableStylePart`) plus `tblBg`, and the `TablePartStyle` / `TableStyleTextStyle` /
  `TableStyleCellStyle` / `TableCellBorderStyle` / `TableBackgroundStyle` / `FontReference` / `Cell3D`
  leaves. Every accessor reuses the DrawingML already modeled (fills, `LineProperties`, `Color`,
  `EffectList`, theme references via `StyleMatrixReference`). Two new generated types: the tri-state
  `OnOffStyle` (`on`/`off`/`def`) and `FontCollectionIndex`. `Cell3D`'s `a:bevel`/`a:lightRig` are
  preserved opaque pending the 3-D workstream.
- **Authoring the style tree** (`mjx-dml`) — constructors and setters that build a style from parts
  (fill, borders, text emphasis), each merge-not-rebuild and default-dropping.
- **`Presentation` surface** (`mjx-pptx`):
  - the seven `a:tblPr` flags — `table_part` / `set_table_part` (`TablePart`).
  - `table_style_id` / `set_table_style` — read and assign a table's `a:tableStyleId`.
  - `create_table_style`, creating the `tableStyles.xml` part on demand (relationship + content-type
    wired like an image part), and `format_table_style_part` with the new `TableStyleFormat` builder.
  - `with_table_style` — resolve a table's style through the shared part.
  - `PptxError::TableStyleNotFound`.
- **`tests/fixtures/tables.pptx`** — a deck carrying a real `tableStyles.xml` and a table naming its
  style.

### Notes

- A table style is layered formatting keyed by which part of the table a cell is in; modeling the
  part is what makes a `tableStyleId` resolve — the basis for effective cell formatting (next).
- Authoring a style touches exactly the content-types manifest, the presentation's relationships, and
  the new part; every other part stays byte-identical, and reading a styled table dirties nothing.

### Added

- **`Presentation::insert_row`, `remove_row`, `insert_column`, `remove_column`** — an index equal to
  the current count appends; beyond it is `TableCellOutOfRange`. A new row copies the height of the
  row beside it and a new column the width of the column beside it; the frame's own bounds are left
  alone, as PowerPoint leaves them.
- **`Table::insert_row`, `remove_row`, `insert_column`, `remove_column`** (`mjx-dml`) — the
  span-adjustment logic, plus `TableColumn::new`, `TableRow::new`,
  `TableCell::set_body_and_properties`, and grid/row/cell insert-and-remove helpers.

### Notes

- **The grid and every row stay in step.** A column edit changes `a:tblGrid` and one `a:tc` in every
  row together, so the rows never disagree with the width the grid declares.
- **Merges are adjusted, not left dangling.** A merge the new line falls inside grows by one; a merge
  the removed line lies inside shrinks by one; a merge whose **anchor** is removed promotes the next
  cell of the region, which takes over the anchor's `a:txBody` and `a:tcPr` and the reduced span so
  the table looks unchanged — including a region merged in both directions at once.
- **Removing the last row or column is refused** with `InvalidTableSize`: PowerPoint will not open a
  table with no cells.
- **Insert then remove is byte-identical to no change** — a span that falls back to one loses its
  attribute rather than being written as `gridSpan="1"`.
- The structural edit runs on the typed `Table` (parse, mutate, write back), not the raw tree:
  unlike a single-cell text edit it touches every row anyway, so parsing the whole table costs
  nothing extra and the merge logic is expressed in terms of the model.

## [0.0.17] - 2026-07-22

Cells can be merged, and unmerged.

### Added

- **`Presentation::merge_cells`** — takes a `Cells` selection, since every selection is a rectangle
  and a rectangle is the only shape a merged region can take.
- **`Presentation::unmerge_cells`** — given **any** cell of a region, not only its anchor.
- **`TableCell::set_spans`, `set_merged`, `clear_merge`** (`mjx-dml`).
- **`PptxError::TableMergeCrossesSelection`.**

### Notes

- **Merging never removes a cell.** The anchor states how far it reaches; the covered cells stay in
  the table, each stating that something to its left or above owns it. So the grid stays
  rectangular, `(row, column)` addressing keeps working, and a covered cell **keeps its own text** —
  invisible until unmerged, which is what makes unmerging give everything back.
- **A merge then an unmerge is byte-identical to no change at all.** A default is *removed* rather
  than written: `gridSpan="1"` and `hMerge="0"` are what the schema already assumes.
- **A selection that would cut an existing merge in half is refused.** Truncating it would leave the
  table claiming a span that no longer fits, and growing the selection would merge cells the caller
  never named. A region wholly inside the selection is absorbed instead.
- Merging one cell, or none, changes nothing rather than writing a span of one.

## [0.0.16] - 2026-07-21

Say it once. The table surface stops needing loops.

### Added

- **`Cells`** — which cells an operation is about: `one`, `row`, `column`, `rectangle`, `all`.
- **`CellFormat`** — a builder naming the cell properties to write (`with_fill`, `with_border`,
  `with_outline`, `with_margins`, `with_anchor`, `with_text_direction`), plus `without_fill` /
  `without_border` / `without_borders` for removal.
- **`Presentation::format_cells`, `format_cell_text`, `format_cell_paragraphs`** — apply a spec
  across a selection in one call.

### Notes

- Styling a header row took nine calls in a loop and read like nine things rather than the one thing
  it is. In the office-open canary this change turns twenty-two lines and four loops into nine lines
  and none.
- **Neither half is a new pattern.** The crate already builds specs with `with_`-prefixed setters
  (`CharacterPropertiesSpec`, `LineSpec`), and `set_shape_run_properties` already means "every run in
  this much of the shape". Tables simply never got either.
- **A format writes only what it names**, so recolouring a region cannot flatten borders it never
  mentioned. A format naming nothing writes nothing — not even an empty `a:tcPr`.
- `without_fill` is not `with_fill(FillSpec::None)`: removing lets the table style decide again,
  stating "none" stops it. Same for borders.
- The table is located **once** and the selection walked within it, so formatting a whole table is
  one traversal rather than one per cell.
- The per-cell, per-property setters remain for the single-property case; both paths now share one
  get-or-create for `a:tcPr`.
- Selecting nothing (`Cells::rectangle(1..1, ..)`) is well-formed and changes nothing; a selection
  reaching past an edge reports the table's real dimensions.

## [0.0.15] - 2026-07-21

A table can be made to look like something.

### Added

- **Cell formatting on `Presentation`** — `cell_fill` / `set_cell_fill` / `clear_cell_fill`,
  `cell_border` / `set_cell_border` / `clear_cell_border` (all six edges, both diagonals included),
  `cell_margins` / `set_cell_margins`, `cell_anchor` / `set_cell_anchor`, and
  `cell_text_direction` / `set_cell_text_direction`.
- **`CellMargins`** (`mjx-pptx`) — the four insets, each optional.
- **`TableCellProperties` can now be written** (`mjx-dml`): `set_border`, `set_fill`, `set_margins`,
  `set_anchor`, `set_text_direction`, `set_horizontal_overflow`, plus the matching typed reads.
- **`TextAnchoring`, `TextDirection`, `TextHorizontalOverflow`** — generated from
  `ST_TextAnchoringType`, `ST_TextVerticalType` and `ST_TextHorzOverflowType`.

### Notes

- **A border is an `a:ln` under another name** — same `CT_LineProperties` content, different tag —
  which is why one `LineSpec` describes all six edges and no border type was needed.
- **Merge, not rebuild.** `a:tcPr` carries a `cell3D`, a `headers` and an `extLst` this tier does not
  model, so a child is replaced in place or inserted at its rank in the schema's sequence. Setting
  one border cannot disturb the other five.
- **Removing a fill is not writing `FillSpec::None`.** The first lets the table style decide again;
  the second states that the cell is deliberately unfilled and stops the style. Same for borders.
- **An unstated margin is absent, not zero.** The schema defaults are `0.1"` horizontally and
  `0.05"` vertically, so the two are different facts; `CellMargins` keeps every field optional, and
  a `None` on write leaves that inset exactly as it was.
- `ST_TextVerticalType` is named **`TextDirection`** because its own values include `horz`
  (Horizontal) — it selects which way text flows, so a "vertical" name would misdescribe most of its
  range. `wordArtVertRtl` is `VerticalWordArtRightToLeft`, the title ECMA gives it, even though it
  reads oddly beside `WordArtVertical`.
- The seven `a:tblPr` flags are deliberately **not** here: they emphasize nothing on their own, they
  tell a table style which parts to treat specially, and they land with the `tableStyles.xml` part.

## [0.0.14] - 2026-07-21

Tables exist on the deck — created, sized, and filled in.

### Added

- **`Presentation::add_table`** — builds the whole `p:graphicFrame`: the grid, every row and every
  cell, ready for text. A table is a shape on the existing index space, so it is positioned with
  `set_shape_bounds` and dropped with `remove_shape`.
- **`table_dimensions`, `column_width` / `set_column_width`, `row_height` / `set_row_height`,
  `cell_span`, `merged_cell_anchor`** — the table's shape, and which cell renders where.
- **Thirteen `cell_*` text methods** — `cell_text`, `set_cell_text`, the paragraph and run readers,
  and the formatting setters including the run-splitting `set_cell_text_range_properties`. Each is
  the corresponding shape method addressed at a cell instead: same operation, same errors.
- **`PptxError::ShapeIsNotATable`, `TableCellOutOfRange`, `InvalidTableSize`.**

### Changed

- The private text-body locator now takes a *site* — a shape's `p:txBody` or a cell's `a:txBody` —
  and every text operation is a named function both spellings call. `shape_text` and
  `set_shape_text` inlined their own copy of the locate and are folded in. No behaviour change; the
  text suites pass untouched.

### Notes

- **A cell's `a:txBody` is the same `CT_TextBody` as a shape's**, which is why the cell surface is
  delegation rather than a second implementation — a future text feature stays one change.
- Reaching a cell **walks the raw tree** rather than parsing the table, so editing one cell costs
  what editing a shape costs; only the addressed `a:txBody` is parsed and rebuilt.
- The column count comes from `a:tblGrid`, never from counting a row's cells.
- A new table's columns share the frame width evenly with the **last absorbing the rounding**, so
  they sum to exactly the frame rather than leaving it a few EMU short.
- A new table carries `firstRow` and `bandRow`, as PowerPoint's does: they claim nothing about
  appearance on their own, they tell a table style which parts to emphasize.
- `set_column_width` does **not** resize the frame — a table whose columns no longer sum to its
  frame is what PowerPoint itself produces when a column is dragged.
- Creating a table adds no parts and no relationships: only the slide changes.
- Effective (inherited) cell formatting is not here — a cell inherits from the table style, which
  needs the `tableStyles.xml` part, later in this workstream.

## [0.0.13] - 2026-07-21

The table, modeled. The first tier of the tables workstream.

### Added

- **`Table`, `TableProperties`, `TableGrid`, `TableColumn`, `TableRow`, `TableCell`,
  `TableCellProperties`** (`mjx-dml`) — `a:tbl` and everything under it, typed for the first time.
  A `p:graphicFrame` could already be positioned; now what it frames can be read.
- **`TablePart`** — the seven `a:tblPr` flags (`firstRow`, `bandRow`, …), which do not draw anything
  themselves but tell the table style which parts to emphasize.
- **`CellBorder`** — the six `CT_LineProperties` edges of a cell, including the two diagonals.

### Notes

- **How little of this is new.** A cell's content is a `CT_TextBody` — the *same* type a shape's
  `p:txBody` is — so the whole text tree and its formatting model apply inside a cell unchanged.
  Cell borders are `LineProperties`; cell and table fills are the fill model; widths, heights and
  margins are `Emu`. The genuinely new part is the two-dimensional shape.
- **Merging never removes a cell.** A merged region is anchored at its top-left cell, which carries
  `gridSpan`/`rowSpan`; every covered cell remains present carrying `hMerge`/`vMerge`. So a row holds
  as many `a:tc` as the grid has `a:gridCol`, `(row, column)` addressing has no holes, and
  `Table::merge_anchor` answers which cell actually renders at a position by walking left then up.
- The **grid** is the authority on column count: `a:tblGrid` is where a table declares its width.
  A table missing it reports no columns rather than inferring one from the rows.
- A cell's four margins have **non-zero schema defaults** (0.1" horizontal, 0.05" vertical), so an
  unstated margin is not a zero one; the accessors report what the file states and the defaults are
  exposed as constants.
- `a:tableStyleId` is **reported but not resolved** — the `tableStyles.xml` part it names is a later
  tier of this workstream.
- Nothing in `mjx-pptx` uses this yet: creating a table, reaching cell text, and formatting cells
  are the next PRs.

## [0.0.12] - 2026-07-21

Where a shape actually renders. The transform workstream is complete.

### Added

- **`Presentation::effective_shape_bounds`** and **`Presentation::effective_shape_transform`** — the
  position a shape *renders* at, not the one it declares. A placeholder that places itself nowhere
  resolves through the same-slot placeholder on its layout, and failing that its master.

### Changed

- The candidate walk every effective property starts with — the addressed shape, then the same-slot
  placeholder on each part the surface inherits from — is now **one** private helper
  (`placeholder_candidates` + `candidate_shape`) rather than a copy inside `effective_shape_fill`,
  `_outline` and `_effects`. Behaviour is unchanged; those suites pass untouched.

### Notes

- **Inheritance is all-or-nothing at the `a:xfrm` level.** Text formatting merges tier by tier, each
  supplying what the ones above left unset; a transform does not. A shape cannot take its position
  from the layout and its size from the master, so the first tier that states anything wins whole.
- **A present-but-empty `<a:xfrm/>` states nothing**, so resolution steps past it exactly as it steps
  past a tier with no transform element at all — what `Transform2D::is_empty` exists for.
- A shape that is **not a placeholder** has no tier to inherit from, so its effective transform is
  its explicit one.
- A tier that answers with only a rotation yields `effective_shape_bounds == None`: bounds are all
  four numbers, and the all-or-nothing rule means no other tier is consulted.
- `tests/fixtures/layouts.pptx`'s `slideLayout2` title placeholder no longer declares an `a:xfrm`,
  so it defers to the master — ordinary in real decks, and the only way the master tier becomes
  reachable. A slide built from that layout now resolves its title at the master and its body at the
  layout.
- `docs/TRANSFORM_HANDOFF.md` closes the workstream; `PLAN.md` now names **tables** and **speaker
  notes** as what remains before `v0.1`.

## [0.0.11] - 2026-07-21

A shape can be moved. The transform reaches the deck.

### Added

- **`Presentation::shape_bounds` / `set_shape_bounds`** — read, move and resize any shape. Until now
  `ShapeBounds` was written once, at shape creation, and could be neither read back nor changed.
- **`Presentation::shape_transform` / `set_shape_transform`** — the whole `a:xfrm`: position, size,
  rotation, the two mirror flags, and a group's child coordinate space. Rotation and flips had no
  expression at all before this.
- **`ShapeBounds::from_transform` / `to_transform`** — the bridge to `mjx_dml::Transform2D`.
- **`PptxError::ShapeCannotBePositioned`** — names the one shape kind (`p:contentPart`) whose schema
  has nowhere to put a transform, instead of reporting a missing element.

### Notes

- **A transform is not in the same place for every shape kind**, which is what made this its own
  piece of work: `p:spPr > a:xfrm` for a shape, picture or connector; `p:grpSpPr > a:xfrm` for a
  group (a `CT_GroupTransform2D`, carrying `a:chOff`/`a:chExt`); and `p:xfrm` for a graphic frame —
  PresentationML's namespace, a direct child, and required rather than optional. Only the wrapper
  differs; the `a:off`/`a:ext` inside are DrawingML in every case.
- **`None` from `shape_bounds` is not "at the origin"** — it means the shape places itself nowhere,
  and a placeholder's real position is on its layout or master. Resolving that is the next PR.
- **Setting bounds cannot disturb anything else.** `to_transform` names only position and size, and
  `Transform2D::apply` writes only named fields, so moving a shape leaves its rotation alone and
  moving a group keeps the child space its members are laid out in. Resizing a group does rescale
  its members — a group maps its child space onto its own extent, which is what PowerPoint does.
- Shape creation now emits its `a:xfrm` through the same writer as shape editing, so the two cannot
  drift apart. The bytes are unchanged.
- `tests/fixtures/layouts.pptx` gained a `p:grpSp` and a `p:graphicFrame` (holding a real one-cell
  table) on slide 2, appended so existing shape indices keep their meaning — the two exotic locator
  paths now meet a real file, and the tables workstream inherits a fixture.
- Group members are still not addressable, so bounds are always in the parent tree's coordinate
  space. Computing an absolute rectangle for a shape inside a group needs group descent.

## [0.0.10] - 2026-07-21

Where a shape sits, and which way up — the model tier of the transform workstream.

### Added

- **`Transform2D`, `Position` and `Size`** (`mjx-dml`) — `a:xfrm` typed for the first time: an offset
  (`a:off`), an extent (`a:ext`), a rotation (`@rot`) and the two mirror flags (`@flipH` / `@flipV`).
  One type covers both `CT_Transform2D` and a group's `CT_GroupTransform2D`, whose `a:chOff` /
  `a:chExt` child coordinate space is the same sequence with two more members.
- **`Transform2D::apply`** — writes only the fields a caller names, editing the element in place.

### Notes

- **Every field is optional, and absent is not zero.** A placeholder that declares no `a:xfrm` is
  asking its layout where it goes; a transform that read as "origin, zero-sized" could not be told
  from one that means *ask someone else*, and the inheritance walk depends on telling them apart.
- `apply` **merges rather than rebuilds**, because an `a:xfrm` carries content this model does not
  describe — a group's child coordinate space, an `extLst`, unknown attributes on the `a:off` itself.
  Rebuilding it wholesale would move every member of a group whose position was changed. New children
  are inserted at their rank in the schema's sequence (`off` → `ext` → `chOff` → `chExt`).
- A transform reads the same whether its wrapper is DrawingML's `a:xfrm` or the `p:xfrm` a
  `p:graphicFrame` holds — the wrapper's namespace differs, its children do not.
- The measure attribute readers/writers (`attr_emu`, `push_angle`, …) moved from `effect.rs` to
  `build.rs`: a measure-valued attribute is not an effect's idea, and now has one spelling on read
  and one on write rather than one per module.
- Nothing in `mjx-pptx` uses this yet — reading and writing a shape's bounds is the next PR.

## [0.0.9] - 2026-07-21

What the text actually renders as. The text-formatting workstream is complete.

### Added

- **`Presentation::effective_run_properties`** and **`Presentation::effective_paragraph_properties`**
  — the formatting a run and a paragraph *render* with, not the formatting they declare. Seven tiers
  resolve, each contributing only what the tiers above left unset: the run's `a:rPr`, the paragraph's
  `a:defRPr`, the shape's `a:lstStyle`, the same-slot placeholder's on the layout and master, the
  master's `p:txStyles`, `p:defaultTextStyle`, and the theme font scheme.
- **`p:txStyles` and `p:defaultTextStyle` are read** for the first time — the tiers where a
  placeholder's real size, bullet and alignment have always lived.

### Notes

- The paragraph's level is read **once**, before the walk, and selects which `a:lvlNpPr` every tier
  from the third down contributes: a level-2 paragraph that declares nothing answers with the master
  `bodyStyle`'s `a:lvl3pPr`.
- Colors bake to concrete `RRGGBB`, consistent with `effective_shape_fill`.
- A shape that is **not a placeholder** takes no master text style; it falls through to
  `p:defaultTextStyle`, as PowerPoint does. A font slot the theme leaves undefined keeps its
  `+mj-lt` reference rather than inventing a font.
- `tests/fixtures/layouts.pptx` gained three distinct `bodyStyle` levels and a layout-placeholder
  `a:lstStyle`, so the level axis and the placeholder tier are demonstrable on a real deck.

## [0.0.8] - 2026-07-21

What "inherited" means, made explicit — the merge one tier of the text-formatting ladder performs.

### Added

- **`CharacterPropertiesSpec::merge_under`** and **`ParagraphPropertiesSpec::merge_under`**
  (`mjx-dml`) — merge a lower inheritance tier under a spec: the receiver is the higher tier and
  wins, and the argument supplies only what the receiver leaves unset. Folding from the top reads as
  the ladder does: `run.merge_under(&paragraph).merge_under(&shape)`.

### Notes

- Properties merge as **whole values**, so an explicit "off" — `b="0"`, `a:noFill`, `<a:buNone/>` —
  is a present value that blocks the tier below rather than an absence that falls through it.
- Four fields are not a plain field-wise fallback: fonts merge **per script slot**, tab stops as one
  **list** (`a:tabLst` replaces wholesale), `a:defRPr` **recursively**, and each of the four bullet
  groups **as a unit**.
- These are the merge halves of effective text formatting; the inheritance walk that calls them
  follows.

## [0.0.7] - 2026-07-21

The theme's font scheme — where a typeface of `+mj-lt` finally leads.

### Added

- **`FontScheme`** (`mjx-dml`) — `a:fontScheme` modeled as `{ name, major, minor }`, on both `Theme`
  and the interner-free `ThemeInfo` (`Theme::font_scheme` / `ThemeInfo::font_scheme`), so a deck's
  font scheme is reachable through the existing `Presentation::theme`.
- **`FontCollection`** — one collection's latin / East Asian / complex-script fonts, keyed by the
  existing `FontSlot` (`FontSlot::Symbol` is always absent: a collection has no `a:sym`), plus its
  `SupplementalFont` per-script fallbacks, looked up by ISO 15924 script tag.
- **Theme font references** — `TextFont::theme_reference` parses the six spellings the schema
  defines (`+mj-lt`, `+mj-ea`, `+mj-cs`, `+mn-lt`, `+mn-ea`, `+mn-cs`) into a `ThemeFontReference`;
  anything else, including other `+…` strings, is not a reference. `FontScheme::resolve` answers
  what a font is actually drawn with — itself when literal, the scheme's font when a reference.

### Notes

- The theme part stays read-only: the font scheme is a parsed value view, with no write path.
- This is the last piece the effective-text-formatting resolution needs; the inheritance walk that
  consumes it follows.

## [0.0.6] - 2026-07-21

Text formatting reaches the deck. Everything the previous four releases modeled is now callable on a
real `.pptx`, at every scope a user can select.

### Added

- **The paragraph axis** on `Presentation` — `paragraph_count`, `run_count`, `paragraph_text`,
  `run_text`. Run indices are paragraph-local, matching the document tree. The existing flat
  `set_shape_text` is unchanged.
- **Reading formatting** — `paragraph_properties`, `run_properties`, `end_run_properties`. Reading
  never dirties a part.
- **Writing formatting, one call per selection granularity**:
  - `set_run_properties` — one run.
  - `set_paragraph_run_properties` — every run in a paragraph, and its paragraph mark.
  - `set_shape_run_properties` — every run in the shape, and every mark.
  - `set_text_range_properties` — an arbitrary character range, splitting runs where the range cuts
    across them.
  - `set_text_range_properties_by_grapheme` — the same, addressed in grapheme clusters, so an emoji
    and its modifier are one unit.
  - `set_paragraph_properties` — a paragraph's layout (alignment, level, margins, spacing, bullet).
  - `set_end_run_properties` — the format of an **empty** paragraph, which is what a placeholder
    added but not yet typed into holds.
- **`TextRun::split_at` / `Paragraph::split_run_at`** in `mjx-dml` — divide a run's text, giving both
  halves the original's formatting, so splitting alone changes nothing about how the text renders.
- **`Paragraph::set_end_properties`** — the write half of the `a:endParaRPr` surface.

### Notes

- Formatting a paragraph or a shape also formats the paragraph mark, so text typed at the end takes
  the same formatting — what "select and restyle" means to a user.
- Runs are split but never merged, keeping each edit minimal. A range already aligned to run
  boundaries splits nothing, so repeated edits do not accumulate runs.

## [0.0.5] - 2026-07-21

Bullets and numbering — the marks that express a deck's paragraph hierarchy.

### Added

- **`Bullet`** — what marks a paragraph: `None` (an explicit "no bullet", which overrides an
  inherited one), `Character` (a literal glyph), `AutoNumber` (a scheme plus where its sequence
  starts), or `Picture` (an image by relationship id).
- **`BulletColor`, `BulletSize`, `BulletTypeface`** — the bullet's colour, size and font, each with a
  `FollowText` variant for the schema's "match the text" arm. All four groups are set and inherited
  **independently**, as the schema defines them.
- **Builder support** on `ParagraphPropertiesSpec`: `with_bullet`, `with_bullet_color`,
  `with_bullet_size`, `with_bullet_typeface`, plus `with_bullet_character("•")` and
  `without_bullet()` for the common cases.

### Notes

- A bullet percentage is written in the form both schemas specify and ECMA §21.1.2.4.9 illustrates
  (`val="111%"`); the integer spelling found in some files is still read.
- Setting one bullet group never disturbs the others, and a group left unnamed keeps whatever the
  file had.

## [0.0.4] - 2026-07-21

Paragraph formatting: how a paragraph is laid out, and the per-level styles it inherits from.

### Added

- **`ParagraphProperties`** (`CT_TextParagraphProperties`) — indent level, alignment, left/right
  margins, first-line indent, default tab size, reading direction and font alignment, plus line
  spacing, space before/after, tab stops, and the `a:defRPr` a paragraph's runs default to. One type
  serves `a:pPr`, `a:defPPr` and `a:lvl1pPr`…`a:lvl9pPr`; the line-breaking attributes, bullets and
  anything unknown round-trip verbatim.
- **`ParagraphPropertiesSpec`** — the builder, matching the character-properties conventions.
  Margins, indents and tab stops are stated **in points**; EMU is the file's unit and stays reachable
  through `Emu`.
- **`IndentLevel`** — the 0–8 nesting level a paragraph's inherited bullet, size and indent are
  selected by. `IndentLevel::of(2)` for a literal, `::new(raw)` for a value off the wire, `::TOP` for
  the outermost.
- **`TextSpacing`** — a proportion of the line height (`a:spcPct`) or a fixed distance (`a:spcPts`),
  kept apart because they are different measurements. **`TabStop`** — position and alignment.
- **`TextListStyle`** (`a:lstStyle`) — the paragraph properties a container offers at each level, by
  `level(IndentLevel)`. The same type covers a shape's own list style, a placeholder's, and each of a
  master's three text styles.
- **Typed access from the text tree** — `Paragraph::properties` / `set_properties` and
  `TextBody::list_style`, so `a:pPr` and `a:lstStyle` are no longer opaque.

## [0.0.3] - 2026-07-20

Text formatting begins: the vocabulary and the run-level model. A run's appearance — its size, weight,
slant, underline, colour, font — can now be read and written. (Reaching it through a `Presentation`,
and resolving what a run *inherits*, come next.)

### Added

- **Text simple types** — `TextUnderline`, `TextStrike`, `TextCapitalization`, `TextAlignment`,
  `FontAlignment`, `TabAlignment` and `AutonumberScheme` (41 bullet-numbering schemes), generated from
  `dml-main.xsd` and named from the ECMA-376 §20.1.10 enumeration tables.
- **`FontSize` and `TextPoint`** — text measures stated **in points** (`from_points` / `points`), the
  unit every size control uses. The file's hundredths of a point are reachable only through
  `from_wire` / `to_wire`.
- **`CharacterProperties`** (`CT_TextCharacterProperties`) — size, bold, italic, underline, strike,
  capitalization, spacing, kerning, baseline, language, plus the text fill, glyph outline, effects,
  highlight and the four script fonts. One type serves `a:rPr`, `a:defRPr` and `a:endParaRPr`, and
  everything it does not model — hyperlinks, `dirty`/`err`/`smtClean`, unknown children — round-trips
  verbatim.
- **`CharacterPropertiesSpec`** — an interner-free builder:
  `CharacterPropertiesSpec::new().with_size_points(28.0).with_bold(true).with_color(…)`. Naming a
  property sets it; leaving it unnamed means *inherit*, so `with_bold(false)` and
  `with_underline(TextUnderline::None)` are how a caller overrides an inherited value.
- **`TextFont`** — a typeface reference, whether a literal name or a `+mj-lt`-style theme reference.
- **`resolve_character_properties`** — bakes a run's colours (text fill, glyph outline, effects,
  highlight) down to concrete RGB against a theme scheme and colour map.
- **Typed access from the text tree** — `TextRun::properties` / `set_properties` and
  `Paragraph::end_properties`, so `a:rPr` and `a:endParaRPr` are no longer opaque.

### Notes

- Setting a run's properties **merges** onto its existing `a:rPr` rather than replacing it, so the
  state this model does not describe (`lang`, `dirty`, a hyperlink) survives a restyle. An unset
  property means "leave it alone", never "clear it".

## [0.0.2] - 2026-07-20

The PowerPoint slice — Phases 2 and 3. A real `.pptx` can now be opened, read, edited, built up from
its own layouts and pruned back down, and written out so PowerPoint and LibreOffice open it with every
untouched part byte-identical. Phase 3 closes here; Word (Phase 4) is next.

### Added

- **De/serialization (Phase 2)** — `FromXml`/`ToXml` in `mjx-ooxml-core::convert` and the
  `#[derive(FromXml, ToXml)]` proc-macro in `mjx-derive`. Every modeled type keeps an unknown-content
  bucket, so what we do not model survives a round trip.
- **DrawingML text (Phase 2)** — `mjx-dml`'s `TextBody`/`Paragraph`/`TextRun`/`Text`, with a mutation
  surface.
- **PresentationML (Phase 2)** — `mjx-pptx::Presentation`: `open`/`save`, slide inventory, shape
  enumeration, `shape_text`/`set_shape_text`, and construction — `add_text_box`, `add_shape`,
  `add_slide`. The **office-open canary** (LibreOffice headless must render the produced deck to a
  valid PDF) became a CI gate.
- **Preset geometry (Phase 3)** — all 187 `ST_ShapeType` values generated, and the 117 adjustable
  shapes given **named, spec-sourced control parameters** (a rounded rectangle exposes
  `corner_radius`, never `adj1`), with the meaning derived from `presetShapeDefinitions.xml`.
- **Color, theme and the `spPr` visual trilogy (Phase 3)** — theme (`clrScheme`/`fmtScheme`) with
  color resolution to concrete RGB, and **fill**, **outline** (`a:ln`) and **effects**
  (`a:effectLst`), each modeled both *explicitly* and *effectively* — resolved through style
  references and placeholder inheritance to what actually renders.
- **Images (Phase 3)** — `add_image` media parts (de-duplicated by content, format identified by
  magic bytes), `add_picture` `p:pic` shapes, and picture read/replace — on one shape index space
  covering every shape kind.
- **Layouts and masters (Phase 3)** — the layout/master inventory, generated PresentationML simple
  types, **`Surface` addressing** (every shape call works on a slide, a layout or a master, so editing
  a layout reaches every slide inheriting it), and `add_slide_from_layout`, which returns a slide
  carrying the layout's placeholders ready to fill.
- **Removal (Phase 3)** — `remove_shape` on any surface, and `remove_slide`, which unwires
  `p:sldIdLst` → relationship → part and takes with it every part only that slide referenced (its
  notes slide, unshared media) while sparing anything the rest of the deck still uses.
- **Packaging** — `Package::{insert_part, remove_part, remove_part_cascading,
  set_content_type_default/override, add_relationship, remove_relationship}` over a copy-on-write part
  body, plus `PartName::{resolve, resolve_from_root, relative_target}` — the part-name algebra Word
  and Excel will share.

### Fixed

- `add_shape` / `add_text_box` built a paragraph with no run, so the shape they returned could not be
  filled by `set_shape_text`. Every paragraph they create now holds exactly one run, blank lines
  included.
- `add_slide_from_layout` cloned the date, footer and slide-number placeholders. Those render *from
  the layout* for slides that do not declare them, so the clones suppressed the layout's rendering and
  showed as empty boxes; they are now skipped, as PowerPoint does.

### Notes

- The round-trip contract is unchanged and continuously asserted: per-part decompressed-payload byte
  identity plus structural container identity. Reading dirties nothing; an edit re-serializes only its
  own part.
- Public API remains unstable until `v0.1`.

## [0.0.1] - 2026-07-15

First versioned snapshot. Establishes the workspace, the packaging + fidelity + compatibility core,
the schema-type generator, and full documentation. No format models yet.

### Added

- **Packaging (Phase 0)** — `mjx-opc`: load an OOXML package fully into RAM as an ordered part graph,
  parse `[Content_Types].xml` and `_rels/*.rels`, and re-zip with per-part decompressed-byte identity.
  Minimal namespace-resolving reader in `mjx-xml`.
- **Schema codegen (Phase 0)** — `xtask` generates `mjx-ooxml-types` (namespace table +
  `shared-commonSimpleTypes`) with comprehensive, self-explanatory names and exact wire tokens;
  output is deterministic and committed.
- **Fidelity layer (Phase 1)** — `mjx-ooxml-core` string interner + the `RawDocument` preservation
  tree, and `mjx-xml::fidelity`, a byte-preserving reader + hand-written writer. Parsing then
  re-serializing any part reproduces the source **byte-for-byte** (verified on real `.pptx`/`.docx`/
  `.xlsx` fixtures).
- **Markup Compatibility (Phase 1)** — `mjx-mce`: preserve mode (the untouched tree) and a
  non-mutating resolve mode (`AlternateContent` Choice/Fallback, `Ignorable`, `ProcessContent`,
  `MustUnderstand`).
- **Documentation** — comprehensive rustdoc across all crates (crate guides + runnable examples), a
  facade docs hub (`mjx-ooxml`), enforced via `missing_docs` and a strict-rustdoc CI job.
- **Project** — CI (fmt/clippy/test + wasm/Android/iOS/macOS/Windows cross-compile build matrix),
  dual `MIT OR Apache-2.0` license, and the contributor/agent guides.

### Notes

- Cross-platform: pure-Rust dependency graph; the library crates cross-compile to
  `wasm32-unknown-unknown`, `aarch64-linux-android`, and Apple/Windows targets.
- A broader multi-producer sample corpus and fuzzing are planned for later iterations.

[0.0.9]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.9
[0.0.8]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.8
[0.0.7]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.7
[0.0.6]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.6
[0.0.5]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.5
[0.0.4]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.4
[0.0.3]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.3
[0.0.2]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.2
[0.0.1]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.1
