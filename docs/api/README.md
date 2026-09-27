# The documentation index

Every prose document this repository holds, what it covers, and which crate owns it. One page, so a
reader arrives once and can reach everything.

**This index is checked, not trusted.** `xtask/tests/doc_gate.rs` requires its row set to equal the
`.md` files of the working tree exactly, in both directions: *writing* a page without adding a row
here fails the build — since MJXOFF-290 that is the moment the page is saved and no longer the
moment it is committed — and a row naming a page that does not exist fails it too. The same gate reads every page
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
| [The user guide's site](../../site/README.md) | — | The Docusaurus site that renders the facade guide in Rust, Python and TypeScript |

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
| [Shared markup reachability](../../crates/mjx-ooxml/docs/shared_markup_reachability.md) | `mjx-ooxml` | Which shared-markup types a facade caller can reach, and by which method |

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

## DrawingML — `mjx-dml`

The shared markup every format draws in, and the largest crate in the workspace. One guide set,
written for a caller who has a shape rather than for a reader touring a thousand types.

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-dml/docs/guide/README.md) | `mjx-dml` | What DrawingML is here, the view/spec pair, the three verbs and the named measures |
| [Reaching the shared types](../../crates/mjx-dml/docs/guide/reaching_the_shared_types.md) | `mjx-dml` | Which format meets which DrawingML, and which host wrapper lives here rather than there |
| [Filling, outlining and colouring](../../crates/mjx-dml/docs/guide/filling_outlining_and_colour.md) | `mjx-dml` | `spPr`, the six fills, the outline, and how a colour carries its transforms |
| [Geometry and placement](../../crates/mjx-dml/docs/guide/geometry_and_placement.md) | `mjx-dml` | The transform, the preset catalogue, custom geometry and the guide-formula language |
| [Text bodies](../../crates/mjx-dml/docs/guide/text_bodies.md) | `mjx-dml` | `a:txBody` down to `a:t`, and merging a format onto a run versus replacing it |
| [The theme](../../crates/mjx-dml/docs/guide/the_theme.md) | `mjx-dml` | Reading a theme, resolving a name into a colour, and the one rule for authoring one |
| [Fidelity and the known gaps](../../crates/mjx-dml/docs/guide/fidelity_and_gaps.md) | `mjx-dml` | The four serialization mechanisms, what backs each, and every gap still open |

## SpreadsheetML — `mjx-sml`

The largest crate in the workspace, and **shared markup rather than Excel's**: an embedded workbook
is SpreadsheetML inside a `.pptx` or a `.docx`, which is why the markup sits at rank 2.1 and the
`Workbook` surface at 3.0. MJXOFF-95's and MJXOFF-97's design notes are two of the six pages rather
than orphans beside them.

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-sml/docs/guide/README.md) | `mjx-sml` | Why this is shared markup, where the crate sits, and the shape of the API in one page |
| [Reaching SpreadsheetML](../../crates/mjx-sml/docs/guide/reaching_spreadsheetml.md) | `mjx-sml` | Who reaches in and for what, the line with `mjx-xlsx`, and writing a package from nothing |
| [The slot frame](../../crates/mjx-sml/docs/guide/the_slot_frame.md) | `mjx-sml` | What *held* means, the derived modelled/held split of every part, and generated placement |
| [The cell store](../../crates/mjx-sml/docs/guide/the_cell_store.md) | `mjx-sml` | The packed representation a worksheet's cells are held in, and its budget |
| [Shared strings](../../crates/mjx-sml/docs/guide/shared_strings.md) | `mjx-sml` | The shared string table, its interner, and the fidelity rules on it |
| [The stylesheet](../../crates/mjx-sml/docs/guide/the_stylesheet.md) | `mjx-sml` | The `xf` indirection, indices as identity, and the four spellings of a colour |
| [Fidelity and the known gaps](../../crates/mjx-sml/docs/guide/fidelity_and_gaps.md) | `mjx-sml` | The four mechanisms, half a schema preserved, and what no gate here can see |

## The upper shared markup — `mjx-chart`, `mjx-omml`, `mjx-vml`

One guide set over the three crates of **rank 2.2**, because they are one thing: the markup that sits
*on top of* DrawingML and SpreadsheetML rather than beside them. Only `mjx-chart` uses the height — it
reaches `mjx-sml` for the workbook a chart embeds and `mjx-dml` for everything a chart draws with —
and none of the three may see the other two, which is why the last two pages are hosted by their own
crates exactly as the packaging tier's MCE page is.

**The three do not carry the same guarantee.** `mjx-chart` and `mjx-omml` are schema-validated and
child-ordered from the XSD; a VML part is neither, and the round trip is the only check it has.

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-chart/docs/guide/README.md) | `mjx-chart` | What rank 2.2 is, which crate reaches which, and where the guarantees differ |
| [Reading a chart](../../crates/mjx-chart/docs/guide/reading_a_chart.md) | `mjx-chart` | The chart-space spine, the sixteen plot types and a series' four data sources |
| [Axes, titles and decoration](../../crates/mjx-chart/docs/guide/axes_titles_and_decoration.md) | `mjx-chart` | The furniture, the three tiers a data label inherits over, and the schema's own refusals |
| [Authoring a chart](../../crates/mjx-chart/docs/guide/authoring_a_chart.md) | `mjx-chart` | Building one from a description, and the two places a chart's data can live |
| [The embedded workbook](../../crates/mjx-chart/docs/guide/the_embedded_workbook.md) | `mjx-chart` | Why a data edit patches rather than regenerates, and the eight references it refuses |
| [Fidelity and the known gaps](../../crates/mjx-chart/docs/guide/fidelity_and_gaps.md) | `mjx-chart` | The two mechanisms, what checks each crate, and what no gate here can see |
| [Office MathML](../../crates/mjx-omml/docs/office_math.md) | `mjx-omml` | What OMML is and is not, and the `w:rPr` inside it that this crate may not name |
| [Legacy VML](../../crates/mjx-vml/docs/legacy_vml.md) | `mjx-vml` | The weaker guarantee, the `spid` hop, and the whitespace a start tag comes back with |

## The generated vocabulary — `mjx-ooxml-types`

**84,128 of this crate's 85,400 lines are written by `xtask/src/codegen/`, and until MJXOFF-224
nothing re-derived them.** `CLAUDE.md` decides that generated output is committed rather than built,
which is right and has a cost: a generator defect is frozen into the repository rather than failing
on the next build, and the committed file is the only artefact anyone reads. Five pages, written for
someone who has to decide how much of a generated table to trust — the last is the one to read first.

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../crates/mjx-ooxml-types/docs/guide/README.md) | `mjx-ooxml-types` | What the crate answers, what is generated, what is hand-written, and the line between them |
| [What is generated](../../crates/mjx-ooxml-types/docs/guide/what_is_generated.md) | `mjx-ooxml-types` | The thirteen artefacts, the nine simple-type modules, and the shape of an emitted item |
| [Child order](../../crates/mjx-ooxml-types/docs/guide/child_order.md) | `mjx-ooxml-types` | The 59,529 lines that say where a child belongs, and the three rules that never reorder a document |
| [The naming convention](../../crates/mjx-ooxml-types/docs/guide/the_naming_convention.md) | `mjx-ooxml-types` | How a cryptic `ST_*` token becomes a self-explanatory name, and which half of that is curated |
| [Regenerating](../../crates/mjx-ooxml-types/docs/guide/regenerating.md) | `mjx-ooxml-types` | What `codegen` needs, why the output is committed, what that costs, and how a change lands |
| [What to distrust](../../crates/mjx-ooxml-types/docs/guide/what_to_distrust.md) | `mjx-ooxml-types` | Which gate catches what, which skips silently, and the four things nothing here checks |
| [Schema coverage](../../crates/mjx-ooxml-types/COVERAGE.md) | `mjx-ooxml-types` | Which ECMA-376 schemas the generator covers and which it does not |

## The bindings — `mjx-python`, `mjx-wasm`

One guide set over the two crates that project `mjx-ooxml` onto another language, because they are
one story told twice: the same 259 `Deck`, 123 `Document` and 138 `Workbook` methods, the same 181
value classes and the same 102 enumerations, differing in five places the target language forces.
Hosted by `mjx-python`, with the one page that is about the npm package alone hosted by `mjx-wasm` —
the arrangement the packaging tier and the upper markup already use, because the two bindings are
siblings and neither may see the other.

| Page | Crate | What it covers |
|---|---|---|
| [Guide index](../../bindings/mjx-python/docs/guide/README.md) | `mjx-python` | The two bindings, the shape of both surfaces in one page, and where each answer lives |
| [Installing](../../bindings/mjx-python/docs/guide/installing.md) | `mjx-python` | The wheel, the npm package, building either from source, and freeing a wasm handle |
| [The mapping rules](../../bindings/mjx-python/docs/guide/the_mapping_rules.md) | `mjx-python` | Identity in Python, camelCase in TypeScript, and the five differences that are forced |
| [What is not projected](../../bindings/mjx-python/docs/guide/what_is_not_projected.md) | `mjx-python` | The nine methods that stay in Rust, the one real gap, and two classes nothing can produce |
| [How much is exercised](../../bindings/mjx-python/docs/guide/how_much_is_exercised.md) | `mjx-python` | What the suites actually call, measured, and the gate that keeps the figure honest |
| [The TypeScript surface](../../bindings/mjx-wasm/docs/guide/the_typescript_surface.md) | `mjx-wasm` | The npm package's exports, the two classes it had to invent, and errors as real `Error`s |
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
| [The reference pack](../validation/07-the-reference-pack.md) | `mjx-reference-pack` | The renderer's half of the Office sitting: four files to export to PDF, and how they come back |
| [The fidelity oracle](../validation/08-the-fidelity-oracle.md) | `mjx-render-oracle` | What the three assertion tiers prove, what they refuse to claim, and the approval only a person gives |
| [The Office exports](../../tests/office-exports/README.md) | — | Where the four PDFs exported from the reference pack land, and why the directory ships empty |

## The client platform

The renderer, the viewport, the session and the chrome catalogue: the plan they were built from, the
inventories that say what complete contains, and the checklists a person works through by hand.

| Page | Crate | What it covers |
|---|---|---|
| [The technical plan](../UI_PLATFORM_PLAN.md) | — | The client platform's architecture: box models, display list, painters, viewport, session and shell |
| [Loop 1](../client-platform/BUILD_PLAN_LOOP_1.md) | — | The first execution loop: the renderer for all three formats and the chrome catalogue in Storybook |
| [The Office feature inventory](../client-platform/OFFICE_FEATURE_INVENTORY.md) | — | What a complete renderer and editor contains, derived rather than recalled, and the ledger's row source |
| [The parity ledger](../client-platform/PARITY_LEDGER.md) | — | Generated: every inventory row's state, derived from what the named suites actually check |
| [The session](../client-platform/SESSION_AND_PERSISTENCE.md) | `mjx-session` | Recording every edit as an operation, coalescing them, and committing on a schedule |
| [HTML in the canvas](../client-platform/HTML_BOX_MODEL.md) | `mjx-layout` | Which of three HTML-in-canvas problems is worth solving, as a second implementation of the box model |
| [Design tokens](../client-platform/DESIGN_TOKENS.md) | `mjx-tokens` | The Allr token system as it actually is, and the three things an editor needs added to it |
| [The in-canvas UI inventory](../client-platform/CANVAS_UI_INVENTORY.md) | `mjx-canvas-harness` | Every element the renderer draws that is not document content, and the harness that exercises it |
| [The in-canvas UI audit](../client-platform/CANVAS_UI_AUDIT.md) | `mjx-canvas-harness` | Generated checklist, one line per in-canvas element, for the person doing the pass by hand |
| [The ribbon review checklist](../client-platform/RIBBON_REVIEW_CHECKLIST.md) | — | The sixty ribbon tabs of the three applications, to be reviewed by hand against real Office |
| [The component catalogue](../../ui/README.md) | — | The Phase U workspace outside the rank graph: custom elements, the token resolver and the audit harness |
| [The bundled substitute faces](../../crates/mjx-text/assets/fonts/README.md) | `mjx-text` | Tier 2 of font resolution: the metric-compatible faces shipped so Office fonts paginate as Office does |

## Render tests

Each directory under `tests/render/` is one render case: an input, the pictures it produces, and a
page saying what the picture must show. The suites that write them live in `mjx-reference-pack`;
RC03's are the corporate fixtures that crate authors.

| Page | Crate | What it covers |
|---|---|---|
| [RC02 01 A shape whose fill cannot be answered](../../tests/render/RC02-losses/01-undecorated-shape/README.md) | `mjx-reference-pack` | Render-loss case: a shape whose fill cannot be answered |
| [RC02 02 A run whose paint is defaulted](../../tests/render/RC02-losses/02-text-colour-default/README.md) | `mjx-reference-pack` | Render-loss case: a run whose paint is defaulted |
| [RC02 03 An unresolved colour](../../tests/render/RC02-losses/03-unresolved-colour/README.md) | `mjx-reference-pack` | Render-loss case: an unresolved colour |
| [RC02 04 A picture with no pixels](../../tests/render/RC02-losses/04-missing-image/README.md) | `mjx-reference-pack` | Render-loss case: a picture with no pixels |
| [RC02 05 A glyph run the PDF cannot embed](../../tests/render/RC02-losses/05-pdf-glyph-run/README.md) | `mjx-reference-pack` | Render-loss case: a glyph run the PDF cannot embed |
| [RC02 06 Effects the SVG cannot express](../../tests/render/RC02-losses/06-svg-effects/README.md) | `mjx-reference-pack` | Render-loss case: effects the SVG cannot express |
| [RC02 07 Line ends that are not drawn](../../tests/render/RC02-losses/07-line-ends/README.md) | `mjx-reference-pack` | Render-loss case: line ends that are not drawn |
| [RC02 08 A chart on a slide](../../tests/render/RC02-losses/08-slide-chart/README.md) | `mjx-reference-pack` | Render-loss case: a chart on a slide |
| [RC02 09 A chart on a worksheet](../../tests/render/RC02-losses/09-sheet-chart/README.md) | `mjx-reference-pack` | Render-loss case: a chart on a worksheet |
| [RC02 10 A SmartArt frame](../../tests/render/RC02-losses/10-diagram-frame/README.md) | `mjx-reference-pack` | Render-loss case: a SmartArt frame |
| [RC02 11 An embedded OLE object](../../tests/render/RC02-losses/11-ole-frame/README.md) | `mjx-reference-pack` | Render-loss case: an embedded OLE object |
| [RC02 12 An ink content part](../../tests/render/RC02-losses/12-ink-frame/README.md) | `mjx-reference-pack` | Render-loss case: an ink content part |
| [RC02 13 Icon-set icons](../../tests/render/RC02-losses/13-excel-icons/README.md) | `mjx-reference-pack` | Render-loss case: icon-set icons |
| [RC02 14 A diagonal border](../../tests/render/RC02-losses/14-excel-diagonal/README.md) | `mjx-reference-pack` | Render-loss case: a diagonal border |
| [RC02 15 Flattened rich text](../../tests/render/RC02-losses/15-excel-rich-text/README.md) | `mjx-reference-pack` | Render-loss case: flattened rich text |
| [RC02 16 The four painters agree](../../tests/render/RC02-losses/16-four-painters/README.md) | `mjx-reference-pack` | Render-loss case: the four painters agree |
| [RC02 17 The real deck journey](../../tests/render/RC02-losses/17-real-deck/README.md) | `mjx-reference-pack` | Render-loss case: the real deck journey |
| [RC02 18 The real worksheet journey](../../tests/render/RC02-losses/18-real-worksheet/README.md) | `mjx-reference-pack` | Render-loss case: the real worksheet journey |
| [RC02 19 An unresolved worksheet outline](../../tests/render/RC02-losses/19-unresolved-outline/README.md) | `mjx-reference-pack` | Render-loss case: an unresolved worksheet outline |
| [RC03 corporate docx](../../tests/render/RC03-corporate/docx/README.md) | `mjx-reference-pack` | The corporate document: themed styles, a header table, tracked changes, an equation and a page border |
| [RC03 corporate pptx](../../tests/render/RC03-corporate/pptx/README.md) | `mjx-reference-pack` | The corporate deck: a master with a logo and gradient, and placeholders sized by the master's styles |
| [RC03 corporate xlsx](../../tests/render/RC03-corporate/xlsx/README.md) | `mjx-reference-pack` | The corporate workbook: a styled table, number formats, three conditional kinds, a chart and frozen panes |
| [RC04 01 Five opacities paint five different greys](../../tests/render/RC04-alpha/01-mid-alpha-fill/README.md) | `mjx-reference-pack` | Opacity case: five opacities paint five different greys |
| [RC04 02 Straight in the display list, premultiplied in the readback](../../tests/render/RC04-alpha/02-premultiplication/README.md) | `mjx-reference-pack` | Opacity case: straight in the display list, premultiplied in the readback |
| [RC04 03 Four painters carry the same opacities](../../tests/render/RC04-alpha/03-four-painters/README.md) | `mjx-reference-pack` | Opacity case: four painters carry the same opacities |
| [RC04 04 A 63 % shadow is not a solid one](../../tests/render/RC04-alpha/04-theme-shadow/README.md) | `mjx-reference-pack` | Opacity case: a 63 % shadow is not a solid one |
| [RC04 05 The corporate overlay lets the row beneath it through](../../tests/render/RC04-alpha/05-corporate-overlay/README.md) | `mjx-reference-pack` | Opacity case: the corporate overlay lets the row beneath it through |

## Design notes still in force

*In force* means the **design** each of these records is the design the code still has. Three of them
were written in July 2026 and two are live checklists for the human Office pass; in all five, a
status line, a "next workstream" pointer or a count is of its own date, and only the reasoning is
maintained. For what is current, the rest of this index is.

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
