# The documentation index

Every prose document this repository holds, what it covers, and which crate owns it. One page, so a
reader arrives once and can reach everything.

**This index is checked, not trusted.** `xtask/tests/doc_gate.rs` requires its row set to equal
`git ls-files '*.md'` exactly, in both directions: committing a page without adding a row here fails
the build, and a row naming a page that does not exist fails it too. The same gate reads every page
below and fails when a file path or a crate-qualified symbol it names has stopped resolving. That is
why the list is a table a person writes rather than a file a script emits — the derivation is the
*enforcement*, and the descriptions are the reason to open an index at all.

Rustdoc is the other half. Every public item already carries a doc comment (`missing_docs = "warn"`
plus clippy's `-D warnings`), and intra-doc links are denied on CI, so `cargo doc --workspace
--no-deps` is the reference and these pages are the narrative around it.

---

## Start here

| Page | Crate | What it covers |
|---|---|---|
| [Project README](../../README.md) | — | What the library is, what works today, and how to run every gate |
| [PLAN.md](../../PLAN.md) | — | The roadmap, the milestone definitions, and what is deliberately out of scope |
| [CONTRIBUTING.md](../../CONTRIBUTING.md) | — | How to build, test, lint and fuzz, and what a change has to clear |
| [CHANGELOG.md](../../CHANGELOG.md) | — | Every released version, its ticket and what shipped in it |
| [CLAUDE.md](../../CLAUDE.md) | — | The architecture rules, layering ranks and naming convention this project is held to |
| [AGENTS.md](../../AGENTS.md) | — | The same rules addressed to a coding agent rather than a person |
| [Pull request template](../../.github/pull_request_template.md) | — | The checks a pull request states it has run |

## PowerPoint — `mjx-pptx`

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-pptx/docs/guide/README.md) | `mjx-pptx` | The shape of the PowerPoint surface and where each answer lives |
| [Building a deck](../../crates/mjx-pptx/docs/guide/building_a_deck.md) | `mjx-pptx` | Opening or authoring a presentation, adding slides, saving it back |
| [Shapes and text](../../crates/mjx-pptx/docs/guide/shapes_and_text.md) | `mjx-pptx` | Shape geometry, bounds, text bodies, runs and list styles |
| [Tables, charts and pictures](../../crates/mjx-pptx/docs/guide/tables_charts_pictures.md) | `mjx-pptx` | The three graphic frames a slide can hold, and their editing surfaces |
| [Inheritance and masters](../../crates/mjx-pptx/docs/guide/inheritance_and_masters.md) | `mjx-pptx` | Slide, layout and master, the colour map and the theme behind them |
| [Effective properties](../../crates/mjx-pptx/docs/effective_properties.md) | `mjx-pptx` | The candidate walk that answers what a run actually looks like |
| [Fidelity and gaps](../../crates/mjx-pptx/docs/guide/fidelity_and_gaps.md) | `mjx-pptx` | What round-trips verbatim, what is modelled, and what is preserved raw |

## Word — `mjx-docx`

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-docx/docs/guide/README.md) | `mjx-docx` | The shape of the Word surface and where each answer lives |
| [Building a document](../../crates/mjx-docx/docs/guide/building_a_document.md) | `mjx-docx` | Opening or authoring a document, paragraphs and runs, saving it back |
| [Text and formatting](../../crates/mjx-docx/docs/guide/text_and_formatting.md) | `mjx-docx` | Run content, revisions, bookmarks, hyperlinks and equations |
| [Tables, sections and headers](../../crates/mjx-docx/docs/guide/tables_sections_and_headers.md) | `mjx-docx` | The table grid, section properties, headers, footers and fields |
| [Styles and inheritance](../../crates/mjx-docx/docs/guide/styles_and_inheritance.md) | `mjx-docx` | The style ladder, numbering, and the effective-properties walk |
| [Effective properties](../../crates/mjx-docx/docs/effective_properties.md) | `mjx-docx` | The style-and-numbering ladder that answers what a run looks like |
| [Charts](../../crates/mjx-docx/docs/guide/charts.md) | `mjx-docx` | Charts in a document and the workbook each one carries |
| [Fidelity and gaps](../../crates/mjx-docx/docs/guide/fidelity_and_gaps.md) | `mjx-docx` | What Word markup is modelled, preserved, or deliberately left alone |

## Excel — `mjx-xlsx`

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-xlsx/docs/guide/README.md) | `mjx-xlsx` | The shape of the Excel surface and where each answer lives |
| [Opening and saving](../../crates/mjx-xlsx/docs/guide/opening_and_saving.md) | `mjx-xlsx` | Opening a workbook, what is read eagerly, and what saving rewrites |
| [Reading and editing cells](../../crates/mjx-xlsx/docs/guide/reading_and_editing_cells.md) | `mjx-xlsx` | Cell values, types, shared strings and the batched write path |
| [Authoring a workbook](../../crates/mjx-xlsx/docs/guide/authoring_a_workbook.md) | `mjx-xlsx` | Building a workbook from nothing, sheets, formats and styles |
| [The sheet grid](../../crates/mjx-xlsx/docs/guide/the_sheet_grid.md) | `mjx-xlsx` | Rows, columns, dimensions, merges, freeze panes and views |
| [Formulas and cached values](../../crates/mjx-xlsx/docs/guide/formulas_and_cached_values.md) | `mjx-xlsx` | Why a cached value is never invalidated, and what `calcChain` costs |
| [Conditional formatting](../../crates/mjx-xlsx/docs/guide/conditional_formatting.md) | `mjx-xlsx` | Rule kinds, differential formats and the regions they apply to |
| [Filters and data validation](../../crates/mjx-xlsx/docs/guide/filters_and_data_validation.md) | `mjx-xlsx` | Autofilters, sort state and the validation rules on a range |
| [Worksheet tables](../../crates/mjx-xlsx/docs/guide/worksheet_tables.md) | `mjx-xlsx` | Table parts, their columns, and the uniqueness rules on a display name |
| [Hyperlinks](../../crates/mjx-xlsx/docs/guide/hyperlinks.md) | `mjx-xlsx` | External and internal links on a cell range, and their relationships |
| [Worksheet drawings](../../crates/mjx-xlsx/docs/guide/worksheet_drawings.md) | `mjx-xlsx` | Pictures and shapes anchored to a sheet, and how they are anchored |
| [Charts](../../crates/mjx-xlsx/docs/guide/charts.md) | `mjx-xlsx` | Charts on a sheet and the chart part graph behind them |
| [Cell comments and legacy content](../../crates/mjx-xlsx/docs/guide/cell_comments_and_legacy_content.md) | `mjx-xlsx` | Comments, their VML drawings, and the legacy content beside them |
| [Print setup and sheet kinds](../../crates/mjx-xlsx/docs/guide/print_setup_and_sheet_kinds.md) | `mjx-xlsx` | Page setup, print areas, and the sheet kinds other than a worksheet |
| [Large workbooks](../../crates/mjx-xlsx/docs/guide/large_workbooks.md) | `mjx-xlsx` | The memory model of a big sheet and the costs a caller should expect |
| [Effective properties](../../crates/mjx-xlsx/docs/effective_properties.md) | `mjx-xlsx` | The `xf` indirection that answers what a cell actually looks like |
| [Through the facade](../../crates/mjx-xlsx/docs/guide/through_the_facade.md) | `mjx-xlsx` | The same workbook reached through `mjx-ooxml` and both bindings |
| [Fidelity and the part graph](../../crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md) | `mjx-xlsx` | Which parts are modelled, which are preserved, and what saving touches |
| [Deliberate limitations](../../crates/mjx-xlsx/docs/guide/deliberate_limitations.md) | `mjx-xlsx` | What this library will not do to a workbook, and why each one is refused |

## The facade — `mjx-ooxml`

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-ooxml/docs/guide/README.md) | `mjx-ooxml` | The shape of the binding-ready surface and where each answer lives |
| [Opening and saving](../../crates/mjx-ooxml/docs/guide/opening_and_saving.md) | `mjx-ooxml` | Detecting a format, opening or authoring it, and what saving refuses |
| [Addressing](../../crates/mjx-ooxml/docs/guide/addressing.md) | `mjx-ooxml` | The three addressing vocabularies, and the four calls taking the column first |
| [One vocabulary, three surfaces](../../crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md) | `mjx-ooxml` | What a caller who learned one format already knows about the other two |
| [Errors](../../crates/mjx-ooxml/docs/guide/errors.md) | `mjx-ooxml` | The eleven stable codes, the coordinates beside them, and the typed cause |
| [The curated surface](../../crates/mjx-ooxml/docs/guide/the_curated_surface.md) | `mjx-ooxml` | What stays behind in each format crate, why, and the three escape hatches |
| [Fidelity and the known gaps](../../crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md) | `mjx-ooxml` | The round-trip contract through the facade, and what is absent |

## The packaging tier — `mjx-opc`, `mjx-mce`, `mjx-xml`, `mjx-ooxml-core`

One guide set over four crates, because the fidelity mechanism is spread across all of them and no
one of them can be read alone. It is hosted by `mjx-opc`, the only crate in the tier that can see two
of the other three; the MCE page is hosted by `mjx-mce`, which is the same rank as `mjx-opc` and
therefore unreachable from it.

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-opc/docs/guide/README.md) | `mjx-opc` | The four crates of the tier, who should read them, and where each answer lives |
| [The package](../../crates/mjx-opc/docs/guide/the_package.md) | `mjx-opc` | Parts, part names, content types, relationships, and what saving refuses to write |
| [Laziness and copy-on-write](../../crates/mjx-opc/docs/guide/laziness_and_copy_on_write.md) | `mjx-opc` | What an open package costs, and which of the two meanings of "lazy" is the true one |
| [Removing a part](../../crates/mjx-opc/docs/guide/removing_a_part.md) | `mjx-opc` | The four removals, their blast radii, and the one an edit is allowed to call |
| [The preservation tree](../../crates/mjx-opc/docs/guide/the_preservation_tree.md) | `mjx-opc` | The lossless tree, its byte ranges, and the two readers only one of which preserves |
| [The round-trip contract](../../crates/mjx-opc/docs/guide/the_round_trip_contract.md) | `mjx-opc` | What is promised, what enforces each clause, and what it deliberately leaves out |
| [Markup compatibility](../../crates/mjx-mce/docs/markup_compatibility.md) | `mjx-mce` | `mc:AlternateContent` and friends: preserved by doing nothing, resolved by a borrow |

## Shared markup

| Page | Crate | What it covers |
|---|---|---|
| [Shared markup reachability](../../crates/mjx-ooxml/docs/shared_markup_reachability.md) | `mjx-ooxml` | Which shared-markup types a facade caller can reach, and by which method |
| [The cell store](../../crates/mjx-sml/docs/CELL_STORE.md) | `mjx-sml` | The packed representation a worksheet's cells are held in, and its budget |
| [Shared strings](../../crates/mjx-sml/docs/SHARED_STRINGS.md) | `mjx-sml` | The shared string table, its interner, and the fidelity rules on it |
| [Schema coverage](../../crates/mjx-ooxml-types/COVERAGE.md) | `mjx-ooxml-types` | Which ECMA-376 schemas the generator covers and which it does not |

## The bindings

| Page | Crate | What it covers |
|---|---|---|
| [Python binding](../../bindings/mjx-python/README.md) | `mjx-python` | Installing the wheel, the identity name mapping, and the typed stubs |
| [WebAssembly binding](../../bindings/mjx-wasm/README.md) | `mjx-wasm` | The npm package, its conditional exports and the camelCase surface |

## Measurement and validation

| Page | Crate | What it covers |
|---|---|---|
| [Benchmarks](../BENCHMARKS.md) | — | The large-file corpus, what is measured and how peak memory is read |
| [The validation method](../validation/00-method.md) | — | How the human Office pass is run, and why an agent marks nothing |
| [The validation index](../validation/01-index.md) | — | Every validation entry, its risk level and the artefacts it binds to |
| [Risk order](../validation/02-risk-order.md) | — | The order the areas are worked in, and what makes one riskier |
| [Presentation checks](../validation/03-presentations.md) | — | The per-check pages for every PowerPoint validation area |
| [Document checks](../validation/04-documents.md) | — | The per-check pages for every Word validation area |
| [Workbook checks](../validation/05-workbooks.md) | — | The per-check pages for every Excel validation area |
| [The Office pass](../validation/06-the-office-pass.md) | — | The hand-off a person reads to run the pass against real Office |
| [The Office-authored corpus](../../tests/office-authored/README.md) | — | What may be committed there, its redistribution rule, and why it is empty |

## Design notes still in force

| Page | Crate | What it covers |
|---|---|---|
| [Preset shapes](../DRAWINGML_PRESET_SHAPES.md) | — | The preset geometry catalogue, its guide formulas and the two left out |
| [DrawingML fill](../DRAWINGML_FILL_HANDOFF.md) | — | The fill model, its authoring surface and what it preserves verbatim |
| [Effective cell format](../EFFECTIVE_CELL_FORMAT_HANDOFF.md) | — | How an Excel cell's effective format is resolved through the stylesheet |
| [The Excel facade](../EXCEL_FACADE_HANDOFF.md) | — | How the workbook surface is projected onto `mjx-ooxml` and both bindings |
| [Images](../IMAGES_HANDOFF.md) | — | Pictures on a slide, their blip fills, and the media parts behind them |

## Historical hand-offs — July 2026

These eight predate the Phase A module split and the whole Phase B–F programme. Each carries a dated
banner at its head saying so. They are **kept, not retired**: their descriptions of the layout have
expired, but the design reasoning they record is written down nowhere else, and deleting them would
lose it.

| Page | Crate | What it covers |
|---|---|---|
| [Phase 2 hand-off](../PHASE2_HANDOFF.md) | — | The original PowerPoint vertical slice brief and its guardrails |
| [DrawingML effective fill](../DRAWINGML_EFFECTIVE_FILL_HANDOFF.md) | — | How an effective fill was resolved before the module split |
| [DrawingML outline](../DRAWINGML_OUTLINE_HANDOFF.md) | — | The line and outline model as it was first designed |
| [Layout and master](../LAYOUT_MASTER_HANDOFF.md) | — | The original slide, layout and master inheritance design |
| [Text formatting](../TEXT_FORMATTING_HANDOFF.md) | — | The original text body, paragraph and run formatting design |
| [Transform](../TRANSFORM_HANDOFF.md) | — | The original `a:xfrm` and shape bounds design |
| [Tables](../TABLES_HANDOFF.md) | — | The original `a:tbl` grid, cell and table style design |
| [Custom geometry](../CUSTOM_GEOMETRY_HANDOFF.md) | — | The original custom geometry path and guide formula design |
