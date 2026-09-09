# The parity ledger

> **Generated. Do not edit.** Produced by `cargo run -p xtask -- ledger` from the suites in this workspace; `cargo run -p xtask -- ledger --check` refuses if this file is not what they produce, and that check runs in `xtask/tests/ledger.rs`. The rows come from
> [`OFFICE_FEATURE_INVENTORY.md`](OFFICE_FEATURE_INVENTORY.md); the **states do not** — every one is derived from what the named suites actually contain.

## ⚠ This is a ledger of what was *checked*, not of what is *true*

A row says which suites cover a capability and what those suites assert. It does not say that the result matches Microsoft Office, because **nothing in this workspace has ever been compared against Microsoft Office**. A reader who takes a green ledger for a parity claim has read it as the opposite of what it says.

The distinction has a name in this project and it is printed on every row: an expectation is `SpecCode`, `DocumentedBehaviour` or `EngineDerived`, and **`EngineDerived` means a change detector, not evidence about Office**. A row whose expectations are entirely `EngineDerived` is a row where this engine agrees with itself.

## The three rules

From `OFFICE_FEATURE_INVENTORY.md` §7, and enforced rather than quoted:

1. **A state is produced by a test, never by assertion.** There is no state field in `xtask/src/ledger/rows.rs` — a row declares evidence, and the generator derives the rest. **Anything nothing tests is `not-started`, regardless of what anyone believes about it.**
2. **`preserved-not-rendered` is a legitimate, permanent state** for markup the core round-trips faithfully but the renderer does not draw. It is how "no holes" stays truthful without pretending every element is drawn on day one.
3. **The exclusions of §2 are ledger rows too**, in a fifth state — `out-of-scope` — with the reason attached, so a later reader can reopen the decision instead of rediscovering the gap.

## The five states

| State | Means |
|---|---|
| `implemented` | suites in a crate that draws cover it, and they assert something. **Not** a claim that the output matches Office |
| `partial` | covered, and a suite declares a limitation it asserts — the limitation is quoted in the row |
| `preserved-not-rendered` | the markup round-trips faithfully and nothing draws it. A legitimate, permanent state |
| `not-started` | nothing tests it, whatever anyone believes about it. This is the default, and it is what an unrecognised row becomes |
| `out-of-scope` | excluded by decision, with the reason in the row |

## Standing facts, read off this tree

**Nobody has run Microsoft Office.** The fidelity oracle holds **5** committed baselines and **0** of them carries a human approval; the rest are stamped `approver = generator`, which is a real approval record — the digest binding is live — and **is not a human review**. Parity is judged against real Office on Windows, and LibreOffice is a preliminary change detector whose export is unreliable for shades and gradients. So the parity count is 0 by construction, not by measurement.

| Baseline | Approver |
|---|---|
| `gradient-panel` | `generator` |
| `nested-groups` | `generator` |
| `preset-star` | `generator` |
| `solid-panels` | `generator` |
| `stroked-frames` | `generator` |

**Word cannot reach pixels at all.** `crates/mjx-scene-docx` does not exist, so a Word `FragmentTree` has nothing that turns it into a display list. PowerPoint and Excel both do, and their rows say so. This is derived from the crate directory rather than stated: the day the crate exists, this sentence changes.

**Not every provenance ledger could be read.** These files declare a `Provenance` enum in a form the generator's scanner does not understand — most often because the tiers are passed through a `use` alias rather than written out — so their expectations are **absent from the provenance columns below**. They are named here rather than skipped silently, which is the difference between a known gap and a quiet one:

* `crates/mjx-layout-xlsx/tests/the_format_language_is_evaluated.rs`

**27 of the rows below are `not-started`**, which is the default and is what a capability becomes when the suites do not reach it. That number going *down* because a row was deleted rather than covered would be the one way this document could lie about progress, so the rows are a fixed partition of the inventory and are removed only when the inventory removes them.

## The counts

| State | Rows |
|---|---:|
| `implemented` | 79 |
| `partial` | 6 |
| `preserved-not-rendered` | 22 |
| `not-started` | 27 |
| `out-of-scope` | 12 |
| **total** | **146** |

## Where the denominators come from

**The row count is not a census.** There are 146 rows because that is how this ledger partitions `OFFICE_FEATURE_INVENTORY.md` — a reading of §2 to §6, written by hand in `xtask/src/ledger/rows.rs`. Dividing anything by it produces a fraction of a reading. It is **not** independent of the numerator: the same file decides which rows exist and which suites each one names, so a coarser partition would raise the implemented share without a line of code changing.

The two figures below **are** independent. Each is summed by the generator from a committed derivation with its own regeneration script, neither of which knows this ledger exists — but neither is a denominator the rows divide into, and no percentage in this document is taken against them. They are here to say how large the subject is.

| Independent census | Figure | Source |
|---|---:|---|
| In-scope controls, Excel | 3,680 | `data/command-surface.tsv` |
| In-scope controls, PowerPoint | 4,255 | `data/command-surface.tsv` |
| In-scope controls, Word | 3,934 | `data/command-surface.tsv` |
| **In-scope controls, all three** | **11,869** | `data/command-surface.tsv` |
| **Declared elements, ECMA-376** | **3,404** | `data/schema-census.txt` |

The workspace holds **35** crates and **374** integration suites, of which the rows below name **257**. A suite no row names is not a defect — most of them are unit-level gates on one crate's own invariants — but the gap between those two numbers is the honest measure of how much of the test estate this ledger actually reads.

## The provenance of the evidence

Every layout child since MJXOFF-172 declares, per expectation, where its expected value came from. This is that declaration summed — first across the suites this ledger names, then across the whole workspace.

| Tier | Cited by a row | In the workspace | Means |
|---|---:|---:|---|
| `SpecCode` | 73 | 73 | ECMA-376, or a schema default |
| `DocumentedBehaviour` | 52 | 52 | an external, checkable definition — **the rows that are actually evidence** |
| `EngineDerived` | 122 | 124 | read off this engine — **a change detector, not evidence about Office** |
| **total** | **247** | **249** | |

A row with `—` in its provenance column names no suite that declares any. That is not the same as a row with no evidence: it means the suites covering it never wrote down where their numbers came from, which is a finding about those suites.

## Declared limitations

Each of these is written in the module documentation of the suite that **asserts** it, behind the marker `MJX-LEDGER-LIMITATION:`. That placement is the point: a limitation nothing asserts is a claim, and a limitation a suite asserts is a fact that goes red when it stops being true. Every one of them demotes its row to `partial`.

| Row | Limitation | Asserted by |
|---|---|---|
| `effects` | a resolved `a:alpha` is destroyed at the `mjx-dml` boundary, so every theme-styled shadow renders at 100 % instead of the standard theme's 63 % — a solid slab under the shape instead of a soft one | `mjx-scene-pptx: the_opacity_is_lost_at_the_spec_boundary` |
| `chart-axes-and-scales` | chart text is *measured* and not shaped, so every rectangle a label or a title occupies is a few percent off and the furniture around it moves with the error | `mjx-layout-chart: the_furniture_is_drawn` |
| `chart-furniture` | chart text is *measured* and not shaped, so every rectangle a label or a title occupies is a few percent off and the furniture around it moves with the error | `mjx-layout-chart: the_furniture_is_drawn` |
| `math-typesetting` | `mjx-text` parses no OpenType `MATH` table, so a stretchy delimiter is *scaled* rather than assembled from glyph variants and its stroke weight grows with its height | `mjx-layout-docx: an_equation_is_typeset` |
| `excel-conditional-formatting` | a conditional-formatting rule whose condition is a formula — an `expression` rule, a `cellIs` with a reference operand, a `cfvo` of `type="formula"` — is reported unevaluated and painted as nothing, because evaluating it needs a calculation engine | `mjx-layout-xlsx: the_conditional_ledger_is_computed` |
| `excel-cell-borders` | a cell border reaches the display list as a filled band, so a dashed or dotted edge draws solid at the right weight and colour; the style survives in the catalogue and nothing consumes it | `mjx-scene-xlsx: the_dash_is_lost_at_the_band` |

## The rows

### §2 · Excluded by decision

| Row | Excluded surface | State | Reason |
|---|---|---|---|
| `excluded-add-in-host` | Add-in host — `TabAddIns`, `GroupOfficeExtension` | `out-of-scope` | an add-in runtime is a host for third-party code, not the application's own document surface |
| `excluded-cloud-intelligence` | Cloud intelligence — `GroupAIAssistance`, `GroupIdeas`, `GroupDesignerOptions` | `out-of-scope` | a service call, not a document feature; the result it inserts is ordinary markup this ledger already covers |
| `excluded-cloud-collaboration` | Cloud collaboration — `GroupCollaborate`, `TabConflicts`, `TabMerge` | `out-of-scope` | co-authoring is a tenant and transport concern; the merge of two documents is not the application's rendering surface |
| `excluded-tenant-licensing` | Tenant, licensing and labelling — `GroupClassifyLabelProtect`, `TabSyntex` | `out-of-scope` | an organisation's policy surface, bound to a tenant this library has no notion of |
| `excluded-external-data-and-bi` | External data and BI — `GroupPowerQuery*` (233 controls in Excel alone), `GroupPowerBI` | `out-of-scope` | a query engine against external sources; the workbook it lands in is SpreadsheetML this ledger already covers |
| `excluded-automation-runtimes` | Automation runtimes — `TabDeveloper`, `GroupMacros`, `GroupOfficeScripts`, `GroupPythonChunk` | `out-of-scope` | executing code embedded in a document is a security surface this project does not open; VBA parts are preserved as opaque bytes and never run |
| `excluded-speech-and-translation` | Speech and translation services — `GroupVoiceTools`, `GroupLiveSubtitles` | `out-of-scope` | a service call with no document markup of its own |
| `excluded-external-publishing` | External publishing — `TabBlogPost`, `GroupInsertBarcode` | `out-of-scope` | publishing to a third-party endpoint, not an editing or rendering surface |
| `excluded-help-and-community` | Help and community — `HelpTab`, `GroupExcelCommunity`, `GroupResearch` | `out-of-scope` | application chrome around the product rather than the product |
| `excluded-mail-merge` | Mail merge — `TabMailings`, 51 controls | `out-of-scope` | **flagged for revisiting.** The merge engine belongs to Word and only its data source is external; excluded from the first pass because it is self-contained enough to add later without disturbing anything |
| `excluded-ink` | Ink — `TabDrawInk`, 32–52 controls per application | `out-of-scope` | **flagged for revisiting.** Ink is a genuine document feature stored in the file and a stylus is first-class on the mobile target; the markup is already preserved (`crates/mjx-pptx/tests/ink.rs`) and nothing lays it out or draws it |

### §3.1 · Text and typography

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `run-formatting` | Run formatting — family, size, colour, bold, italic, underline, strikethrough, caps | `implemented` | `mjx-dml: character_model`<br>`mjx-pptx: text_formatting`<br>`mjx-docx: run_properties`<br>`mjx-layout-pptx: a_slide_becomes_fragments`<br>`mjx-layout-docx: a_document_becomes_fragments` | 73 | 304 | — |
| `text-shaping` | Shaping and script itemisation | `implemented` | `mjx-text: shaping`<br>`mjx-text: itemisation` | 34 | 128 | — |
| `bidirectional-text` | Bidirectional text — the UBA, and a bidi-aware run order | `implemented` | `mjx-text: bidirectional_text` | 11 | 62 | — |
| `line-breaking` | Line breaking — UAX #14 classes and the break opportunities a layout consumes | `implemented` | `mjx-text: line_breaking`<br>`mjx-layout-docx: break_opportunities_that_matter` | 26 | 88 | 1 / 7 / 1 |
| `hyphenation-and-segmentation` | Hyphenation, and grapheme/word segmentation | `implemented` | `mjx-text: segmentation_and_hyphenation` | 18 | 53 | — |
| `font-resolution` | Font resolution — the three tiers, metric-compatible substitution, the per-document manifest | `implemented` | `mjx-text: metric_compatibility`<br>`mjx-text: substitution_manifest`<br>`mjx-text: empty_system_tier` | 19 | 56 | — |
| `glyph-rasterisation` | Glyph rasterisation and the atlas | `implemented` | `mjx-text: glyph_rasterisation`<br>`mjx-text: glyph_atlas` | 22 | 123 | — |
| `glyph-outlines` | Glyph outlines as paths, tessellated for the vector painters | `implemented` | `mjx-scene: glyph_outlines_tessellate_as_paths` | 2 | 14 | — |
| `font-subsetting` | Font subsetting, for an export that embeds only what it uses | `implemented` | `mjx-text: a_subset_is_a_font` | 5 | 15 | — |
| `text-measurement` | Character spacing, scaling, kerning and position — the measures a line is composed from | `implemented` | `mjx-dml: text_measures`<br>`mjx-layout: text_composition` | 13 | 76 | — |
| `text-effects` | Text effects — shadow, glow, reflection, bevel | `implemented` | `mjx-dml: effect_model`<br>`mjx-scene-pptx: every_effect_reaches_the_root` | 15 | 63 | — |
| `east-asian-typography` | East Asian typography — ruby, vertical text, `kinsoku` line-break rules | `not-started` | **none** | 0 | 0 | — |

### §3.2 · Paragraphs

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `paragraph-alignment` | Alignment and justification, including the positions a justified line resolves to | `implemented` | `mjx-layout-docx: a_justified_line_has_positions` | 11 | 31 | 2 / 3 / 2 |
| `paragraph-spacing-and-indents` | Indentation, spacing before and after, line spacing, and five kinds of tab stop | `implemented` | `mjx-layout-docx: spacing_tabs_and_indents`<br>`mjx-docx: paragraph_properties` | 16 | 108 | 7 / 1 / 6 |
| `bullets-and-numbering` | Bullets and multilevel numbered lists, with restart and continuation | `implemented` | `mjx-layout-docx: a_list_composes_its_marker`<br>`mjx-layout-pptx: bullets_and_indent_levels`<br>`mjx-docx: numbering` | 36 | 79 | 5 / 1 / 6 |
| `paragraph-borders-and-rules` | Paragraph borders and shading, at every weight the format allows | `implemented` | `mjx-layout-docx: no_rule_rounds_to_nothing` | 7 | 20 | 2 / 1 / 1 |
| `pagination-controls` | Widow and orphan control, keep-with-next, keep-lines-together, page-break-before | `implemented` | `mjx-layout-docx: each_constraint_moves_a_paragraph` | 8 | 16 | 1 / 0 / 4 |
| `paragraph-hierarchy` | Paragraph and list level hierarchy in a shape's text body | `preserved-not-rendered` | `mjx-pptx: paragraph_hierarchy` | 15 | 30 | — |

### §3.3 · Drawing and shapes

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `preset-geometry` | The preset shape geometries and their path tables | `implemented` | `mjx-geometry: a_preset_renders_as_itself`<br>`mjx-geometry: every_preset_is_structurally_sound`<br>`mjx-geometry: every_preset_stands_where_its_box_is`<br>`mjx-pptx: geometry` | 26 | 103 | — |
| `shape-adjustments` | Adjustment handles, and the guide formulas they drive | `implemented` | `mjx-geometry: an_adjustment_moves_the_shape`<br>`mjx-geometry: every_adjustment_moves_its_shape`<br>`mjx-dml: guide_formula`<br>`mjx-pptx: preset_adjustments` | 77 | 185 | — |
| `custom-geometry` | Custom freeform geometry — `a:custGeom` paths | `implemented` | `mjx-dml: custom_geometry_model`<br>`mjx-geometry: the_third_route_is_the_parser`<br>`mjx-pptx: custom_geometry` | 37 | 133 | — |
| `connectors` | Connectors and their connection sites | `implemented` | `mjx-geometry: a_connector_lands_on_the_outline` | 8 | 37 | — |
| `fills` | Fills — solid, gradient with stop and tile semantics, 54 preset patterns, picture, texture | `implemented` | `mjx-dml: fill_model`<br>`mjx-pptx: fill`<br>`mjx-paint: the_tables_are_tables` | 32 | 73 | — |
| `outlines` | Outlines — weight, dash, cap, join, compound, arrowheads | `implemented` | `mjx-dml: line_model`<br>`mjx-paint: the_tables_are_tables` | 16 | 58 | — |
| `effects` | Effects — `outerShdw`, `innerShdw`, `glow`, `softEdge`, `reflection`, `blur`, and the effect DAG | `partial` | `mjx-dml: effect_model`<br>`mjx-scene-pptx: every_effect_reaches_the_root`<br>`mjx-scene-pptx: the_opacity_is_lost_at_the_spec_boundary` | 18 | 67 | — |
| `colour-resolution` | Colour resolution — scheme colours and the transform chain | `implemented` | `mjx-dml: color_model`<br>`mjx-dml: resolve_model`<br>`mjx-scene-xlsx: the_alpha_survives` | 34 | 81 | — |
| `three-dimensional-shapes` | 3-D rotation and extrusion — `a:scene3d` and `a:sp3d` | `preserved-not-rendered` | `mjx-dml: shape3d_model`<br>`mjx-pptx: shape_3d` | 23 | 88 | — |
| `shape-styles` | Shape styles and theme style references | `preserved-not-rendered` | `mjx-dml: style_model`<br>`mjx-pptx: shape_list_style` | 17 | 47 | — |
| `grouping-and-transforms` | Grouping, nested group transforms, flipping and rotation | `implemented` | `mjx-dml: transform_model`<br>`mjx-pptx: groups`<br>`mjx-pptx: grouping`<br>`mjx-layout-pptx: nested_group_transforms_compose` | 57 | 152 | — |
| `z-order-and-placement` | Z-order, size and position, alignment and distribution | `preserved-not-rendered` | `mjx-pptx: placement`<br>`mjx-pptx: transform` | 29 | 64 | — |
| `text-in-a-shape` | The text body inside a shape — insets, anchoring, and the geometry that bounds it | `implemented` | `mjx-dml: text_model`<br>`mjx-geometry: text_goes_inside_the_shape`<br>`mjx-layout-pptx: the_body_geometry_is_honoured` | 55 | 164 | — |
| `wordart` | WordArt — `TabSetWordArtTools`, and the text-warp preset geometries | `not-started` | **none** | 0 | 0 | — |
| `snapping-and-guides` | Snapping, alignment guides and the editing affordances around a shape | `not-started` | **none** | 0 | 0 | — |

### §3.4 · Pictures

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `picture-insertion` | Pictures — insertion, the media part graph, and the relationship that reaches the bytes | `preserved-not-rendered` | `mjx-pptx: pictures`<br>`mjx-pptx: images` | 27 | 60 | — |
| `picture-anchoring` | The anchoring models — inline, floating, one-cell, two-cell, absolute | `implemented` | `mjx-dml: spreadsheet_drawing_model`<br>`mjx-sml: anchor_geometry`<br>`mjx-layout-xlsx: three_anchor_modes_move_differently`<br>`mjx-docx: drawing_placement` | 37 | 149 | — |
| `picture-cropping` | Cropping, including crop-to-shape and aspect fill | `not-started` | **none** | 0 | 0 | — |
| `picture-corrections` | Corrections and colour — brightness, contrast, saturation, recolour, artistic effects | `not-started` | **none** | 0 | 0 | — |

### §3.5 · Charts

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `chart-model` | The chart model — plot types, series, and the parts that carry them | `preserved-not-rendered` | `mjx-chart: plot_types`<br>`mjx-chart: bar_model`<br>`mjx-chart: author`<br>`mjx-chart: edit` | 28 | 103 | — |
| `chart-series-geometry` | Series geometry — every chart family producing marks in the right places | `implemented` | `mjx-layout-chart: series_geometry_is_asserted`<br>`mjx-layout-chart: every_family_produces_marks`<br>`mjx-layout-chart: degenerate_data_is_defined` | 25 | 77 | 8 / 5 / 13 |
| `chart-axes-and-scales` | Axes, scales and tick selection | `partial` | `mjx-layout-chart: ticks_are_arithmetic`<br>`mjx-layout-chart: the_furniture_is_drawn` | 19 | 86 | 6 / 8 / 13 |
| `chart-furniture` | Titles, legends, data labels and gridlines | `partial` | `mjx-chart: furniture`<br>`mjx-layout-chart: the_furniture_is_drawn` | 19 | 115 | 4 / 2 / 8 |
| `chart-decoration` | Series and data-point formatting, and the document palette a series inherits | `implemented` | `mjx-chart: decoration`<br>`mjx-layout-chart: the_document_palette_wins` | 30 | 214 | 1 / 0 / 0 |
| `chart-embedded-workbook` | The embedded workbook that backs the data, and its external-data alternative | `preserved-not-rendered` | `mjx-chart: data_sources`<br>`mjx-chart: external_data`<br>`mjx-pptx: chart_workbook` | 15 | 53 | — |
| `chart-unmodelled-plot-types` | The plot types `mjx-chart` preserves without modelling | `preserved-not-rendered` | `mjx-chart: unmodelled_plot_types` | 5 | 21 | — |
| `chart-in-three-formats` | A chart reached through PowerPoint's, Word's and Excel's box models | `implemented` | `mjx-layout-chart: fragments_reach_the_tree`<br>`mjx-docx: charts`<br>`mjx-xlsx: charts`<br>`mjx-pptx: charts` | 96 | 433 | 0 / 0 / 3 |

### §3.6 · SmartArt and diagrams

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `diagram-parts` | The four-part diagram model — data, layout, style, colours | `preserved-not-rendered` | `mjx-pptx: diagrams`<br>`mjx-pptx: diagram_read_back` | 14 | 40 | — |
| `diagram-layout` | Diagram layout — a hierarchy solved into boxes | `implemented` | `mjx-layout-chart: a_hierarchy_is_laid_out` | 10 | 29 | 4 / 1 / 3 |
| `diagram-layout-catalogue` | The ~200 published layouts across eight categories, and their algorithm evaluation | `not-started` | **none** | 0 | 0 | — |

### §3.7 · Mathematics

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `omml-model` | The OMML model — fractions, radicals, n-ary operators, matrices, delimiters, accents | `preserved-not-rendered` | `mjx-omml: deep_nesting`<br>`mjx-docx: equations` | 6 | 44 | — |
| `math-typesetting` | Typesetting an equation — bar positions, script scale, delimiter growth, array alignment | `partial` | `mjx-layout-docx: an_equation_is_typeset` | 12 | 27 | 1 / 6 / 7 |

### §3.8 · Tables

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `table-model` | Table structure — rows, columns, cells, insertion and deletion | `preserved-not-rendered` | `mjx-dml: table_model`<br>`mjx-pptx: tables`<br>`mjx-pptx: table_structure`<br>`mjx-docx: tables` | 94 | 299 | — |
| `table-styles` | Table styles and the six conditional-formatting bands | `preserved-not-rendered` | `mjx-dml: table_style`<br>`mjx-pptx: table_styles`<br>`mjx-pptx: table_effective` | 42 | 110 | — |
| `table-merging` | Merged and split cells, and the walk that must not be naive | `implemented` | `mjx-pptx: table_merging`<br>`mjx-layout-pptx: merged_cells_are_not_a_naive_walk` | 25 | 54 | — |
| `table-grid-solving` | Column sizing — the fixed and autofit layout algorithms | `implemented` | `mjx-layout-docx: a_table_grid_is_solved` | 10 | 25 | 4 / 2 / 5 |
| `table-splitting` | Tables that split across pages, with repeated header rows | `implemented` | `mjx-layout-docx: a_table_splits_across_a_page` | 5 | 16 | 2 / 0 / 4 |
| `table-borders` | Cell borders and shading, with the resolution precedence between them | `implemented` | `mjx-pptx: table_formatting`<br>`mjx-docx: table_formatting`<br>`mjx-layout-pptx: a_cell_border_is_a_band_that_covers_pixels` | 31 | 88 | — |
| `table-selection` | Formatting a region of cells in one call, and the smaller cell surfaces beside it — accessibility headers, visible cell text | `implemented` | `mjx-pptx: table_selection`<br>`mjx-pptx: table_gaps` | 23 | 41 | — |

### §3.9 · Cross-cutting document surfaces

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `themes` | Themes — colour schemes, font schemes, effect schemes | `preserved-not-rendered` | `mjx-dml: theme_model`<br>`mjx-pptx: theme` | 14 | 75 | — |
| `styles-and-inheritance` | Styles, style sets, and every inheritance ladder a property resolves through | `implemented` | `mjx-docx: styles`<br>`mjx-docx: effective`<br>`mjx-pptx: text_inheritance`<br>`mjx-pptx: transform_inheritance`<br>`mjx-layout-pptx: the_ladder_is_consumed`<br>`mjx-layout-docx: the_ladder_is_consumed`<br>`mjx-layout-xlsx: the_ladder_is_consumed` | 58 | 136 | — |
| `comments` | Comments, both legacy and threaded | `preserved-not-rendered` | `mjx-docx: annotations`<br>`mjx-xlsx: comments` | 30 | 109 | — |
| `hyperlinks` | Hyperlinks, and the relationships that carry their targets | `preserved-not-rendered` | `mjx-pptx: hyperlinks`<br>`mjx-sml: worksheet_hyperlinks`<br>`mjx-xlsx: hyperlinks` | 40 | 183 | — |
| `round-trip-fidelity` | The round-trip contract — untouched parts re-emitted byte for byte | `implemented` | `mjx-opc: roundtrip`<br>`mjx-opc: tree_roundtrip`<br>`mjx-pptx: roundtrip`<br>`mjx-docx: roundtrip`<br>`mjx-xlsx: roundtrip`<br>`mjx-xlsx: preserved_parts` | 51 | 215 | — |
| `markup-compatibility` | MCE — `mc:AlternateContent`, `Ignorable`, `ProcessContent` | `implemented` | `mjx-mce: resolve`<br>`mjx-mce: untrusted_input` | 10 | 15 | — |
| `schema-conformance` | ECMA-376 schema validity and child order, for everything this library writes | `implemented` | `mjx-schema-gate: ordering`<br>`mjx-schema-gate: category_rule`<br>`mjx-docx: schema_gate`<br>`mjx-xlsx: schema_gate` | 52 | 123 | — |
| `untrusted-input` | Untrusted input — a malformed file is a typed error and never a panic | `implemented` | `mjx-xml: untrusted_input`<br>`mjx-opc: untrusted_input`<br>`mjx-text: untrusted_faces`<br>`mjx-text: untrusted_text`<br>`mjx-layout-docx: no_panic_on_a_layout_path`<br>`mjx-layout-pptx: no_panic_on_a_layout_path`<br>`mjx-layout-xlsx: no_panic_on_a_layout_path`<br>`mjx-layout-chart: no_panic_on_a_layout_path` | 55 | 79 | 0 / 0 / 1 |
| `document-properties` | Document properties and metadata | `preserved-not-rendered` | `mjx-opc: package_validation` | 14 | 26 | — |
| `export-pdf-and-svg` | Export — PDF with selectable text, and SVG, both from the display list | `implemented` | `mjx-paint: a_document_is_a_document`<br>`mjx-render-oracle: the_pdf_tiers_work_on_our_own_exports` | 13 | 59 | — |
| `find-and-replace` | Find and replace, including formatting and wildcards | `not-started` | **none** | 0 | 0 | — |
| `spelling-and-grammar` | Spell check, grammar, and the proofing language settings | `not-started` | **none** | 0 | 0 | — |
| `clipboard` | The clipboard, its four flavours, and paste-special | `not-started` | **none** | 0 | 0 | — |
| `protection-and-signatures` | Document protection, encryption and digital signatures | `not-started` | **none** | 0 | 0 | — |
| `print-and-page-setup` | Print and page setup, across the three applications | `implemented` | `mjx-layout-xlsx: print_layout_paginates`<br>`mjx-sml: print_and_sheet_kinds` | 39 | 232 | — |

### §4.1 · Word

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `word-flow-and-pagination` | Flow and pagination — a document becoming pages, consistently | `implemented` | `mjx-layout-docx: a_document_becomes_fragments`<br>`mjx-layout-docx: a_long_document_paginates_consistently`<br>`mjx-layout-docx: the_fragments_match_their_baselines` | 12 | 32 | 0 / 1 / 3 |
| `word-sections` | Sections — page size, orientation, margins, and section-scoped numbering | `implemented` | `mjx-docx: sections`<br>`mjx-layout-docx: a_section_changes_the_page` | 16 | 87 | 5 / 2 / 9 |
| `word-columns` | Multiple columns, and the balance at a continuous break | `implemented` | `mjx-layout-docx: columns_balance_at_a_continuous_break` | 6 | 23 | 3 / 1 / 4 |
| `word-headers-and-footers` | Headers and footers — first page, odd and even, and per section | `implemented` | `mjx-docx: headers`<br>`mjx-layout-docx: the_headers_differ_by_page` | 20 | 51 | 4 / 0 / 3 |
| `word-floating-objects` | Floating objects and text wrapping — `square`, `tight`, `through`, `topAndBottom`, behind, in front | `implemented` | `mjx-layout-docx: text_wraps_around_a_float` | 12 | 35 | 3 / 1 / 6 |
| `word-footnotes-and-endnotes` | Footnotes and endnotes — their own reflow, and the mark on the line | `implemented` | `mjx-layout-docx: a_footnote_moves_the_body`<br>`mjx-layout-docx: endnotes_flow_at_the_end_of_their_scope`<br>`mjx-layout-docx: a_generated_mark_is_measured` | 19 | 47 | 5 / 3 / 13 |
| `word-fields` | Fields — the 90+ types, their computation, and the fixed point an update reaches | `implemented` | `mjx-docx: fields`<br>`mjx-layout-docx: a_field_fixed_point_terminates`<br>`mjx-layout-docx: a_stale_field_is_recomputed` | 33 | 88 | 5 / 5 / 7 |
| `word-track-changes` | Track changes — insertions, deletions, formatting revisions, and their effect on layout | `implemented` | `mjx-docx: revisions`<br>`mjx-layout-docx: a_deletion_changes_the_page` | 20 | 63 | 2 / 1 / 4 |
| `word-line-numbers` | Line numbers, restarting per the four modes | `implemented` | `mjx-layout-docx: line_numbers_restart_per_mode` | 6 | 14 | 2 / 0 / 4 |
| `word-content-controls` | Content controls, bookmarks and structured document tags | `preserved-not-rendered` | `mjx-docx: structured_content`<br>`mjx-docx: content_model` | 23 | 77 | — |
| `word-resumable-layout` | The checkpoint that makes flow layout resumable, across tables and floats | `implemented` | `mjx-layout-docx: a_checkpoint_is_work_not_output`<br>`mjx-layout-docx: a_checkpoint_survives_tables_and_floats`<br>`mjx-layout-docx: termination` | 20 | 40 | 0 / 3 / 0 |
| `word-reaches-pixels` | A Word document reaching a display list, and then pixels | `not-started` | **none** | 0 | 0 | — |
| `word-drop-caps-and-frames` | Drop caps, text frames and watermarks | `not-started` | **none** | 0 | 0 | — |
| `word-toc-and-index` | Tables of contents, indexes, tables of authorities and captions, as generated content | `not-started` | **none** | 0 | 0 | — |
| `word-outline-and-master-documents` | Outline view and master documents — `TabOutlining` | `not-started` | **none** | 0 | 0 | — |

### §4.2 · Excel

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `excel-number-format-engine` | The number-format engine — `numFmt` to display string, and its degradation on a bad code | `implemented` | `mjx-layout-xlsx: the_format_language_is_evaluated`<br>`mjx-layout-xlsx: a_cell_shows_its_formatted_value`<br>`mjx-layout-xlsx: a_broken_format_degrades` | 24 | 58 | — |
| `excel-grid-layout` | Grid layout — row and column sizing, a windowed and sparse read of a large sheet | `implemented` | `mjx-layout-xlsx: a_sheet_becomes_fragments`<br>`mjx-layout-xlsx: the_grid_is_windowed_and_sparse`<br>`mjx-layout-xlsx: sparsity_is_measured` | 21 | 102 | — |
| `excel-merged-regions` | Merged regions, rendered once | `implemented` | `mjx-layout-xlsx: merged_regions_render_once` | 5 | 15 | — |
| `excel-text-overflow` | Text overflowing into empty neighbours, and the rules that clip it | `implemented` | `mjx-layout-xlsx: text_overflows_into_empty_neighbours` | 7 | 22 | — |
| `excel-panes` | Frozen and split panes over one geometry | `implemented` | `mjx-layout-xlsx: frozen_panes_share_one_geometry` | 6 | 29 | — |
| `excel-conditional-formatting` | Conditional formatting — every rule kind, graded interpolation, and Excel's precedence | `partial` | `mjx-sml: conditional_formatting`<br>`mjx-xlsx: conditional_formatting`<br>`mjx-layout-xlsx: every_rule_kind_fires_and_does_not`<br>`mjx-layout-xlsx: graded_rules_interpolate`<br>`mjx-layout-xlsx: precedence_composes_in_excels_order`<br>`mjx-layout-xlsx: the_conditional_ledger_is_computed`<br>`mjx-scene-xlsx: a_conditional_format_changes_a_pixel` | 88 | 299 | — |
| `excel-cell-borders` | Cell borders — every weight reaching a pixel, and the dash that does not | `partial` | `mjx-layout-xlsx: no_border_rounds_to_nothing`<br>`mjx-scene-xlsx: the_dash_is_lost_at_the_band` | 9 | 22 | — |
| `excel-cell-formatting` | The effective cell format — the `xf` chain, fills, fonts and alignment | `implemented` | `mjx-sml: effective_cell_format`<br>`mjx-sml: style_resources`<br>`mjx-xlsx: effective_format`<br>`mjx-scene-xlsx: a_red_negative_reaches_the_paint_table` | 31 | 226 | — |
| `excel-print-layout` | Print layout — page breaks, print areas, repeated rows and scaling | `implemented` | `mjx-layout-xlsx: print_layout_paginates` | 17 | 65 | — |
| `excel-tables-and-filters` | Tables, autofilters and sorting | `preserved-not-rendered` | `mjx-sml: worksheet_tables`<br>`mjx-sml: validation_and_filters`<br>`mjx-xlsx: worksheet_tables`<br>`mjx-xlsx: validation_and_filters` | 73 | 308 | — |
| `excel-drawings` | Cell drawings and worksheet objects | `preserved-not-rendered` | `mjx-sml: worksheet_objects`<br>`mjx-xlsx: worksheet_drawings` | 24 | 126 | — |
| `excel-reaches-pixels` | A worksheet reaching a display list, and then pixels | `implemented` | `mjx-scene-xlsx: a_real_sheet_resolves`<br>`mjx-reference-pack: a_real_worksheet_reaches_pixels` | 9 | 35 | — |
| `excel-cell-store` | The packed cell store — what holding a large sheet costs, measured rather than asserted | `implemented` | `mjx-sml: cell_store_allocation`<br>`mjx-sml: cell_store_fidelity`<br>`mjx-sml: shared_string_allocation` | 18 | 151 | — |
| `excel-gridlines` | Sheet gridlines — the ruled lines a worksheet shows where no border is set | `not-started` | **none** | 0 | 0 | — |
| `excel-sparklines` | Sparklines — `x14:sparklineGroups`, preserved in an `extLst` and unmodelled | `not-started` | **none** | 0 | 0 | — |
| `excel-pivot-tables` | Pivot tables and pivot charts — `TabSetPivotTableTools`, 570 controls across the two | `not-started` | **none** | 0 | 0 | — |
| `excel-data-tools` | Data tools — text to columns, flash fill, remove duplicates, what-if analysis | `not-started` | **none** | 0 | 0 | — |

### §4.3 · PowerPoint

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `pptx-slide-layout` | A slide becoming fragments, against a committed baseline | `implemented` | `mjx-layout-pptx: a_slide_becomes_fragments`<br>`mjx-layout-pptx: the_fragments_match_their_baselines` | 14 | 41 | — |
| `pptx-master-and-layout` | The master, layout, notes and handout hierarchy, and placeholder inheritance | `implemented` | `mjx-pptx: layouts`<br>`mjx-pptx: surfaces`<br>`mjx-layout-pptx: the_ladder_is_consumed` | 30 | 77 | — |
| `pptx-autofit` | Autofit — the search that makes text fit its body | `implemented` | `mjx-layout-pptx: autofit_is_a_search` | 9 | 28 | — |
| `pptx-notes` | Notes pages, laid out as slides by another name | `implemented` | `mjx-pptx: notes`<br>`mjx-layout-pptx: a_notes_page_is_a_slide_by_another_name` | 18 | 48 | — |
| `pptx-slide-authoring` | Creating, removing and reordering slides | `implemented` | `mjx-pptx: slide_creation`<br>`mjx-pptx: removal`<br>`mjx-pptx: blank_document` | 36 | 125 | — |
| `pptx-reaches-pixels` | A deck reaching a display list, and then pixels, through two independent painters | `implemented` | `mjx-scene-pptx: every_effect_reaches_the_root`<br>`mjx-paint: a_page_becomes_pixels`<br>`mjx-paint: two_painters_agree`<br>`mjx-reference-pack: a_real_deck_reaches_pixels` | 25 | 97 | — |
| `pptx-hit-testing` | A point finding its run, and the addressing agreeing with the session's | `implemented` | `mjx-layout-pptx: a_point_finds_its_run`<br>`mjx-layout-pptx: the_addressing_agrees_with_the_session` | 13 | 47 | — |
| `pptx-media` | Audio and video — the parts, the timing markup, and playback | `preserved-not-rendered` | `mjx-pptx: media` | 8 | 25 | — |
| `pptx-ole-and-activex` | OLE objects and ActiveX controls | `preserved-not-rendered` | `mjx-pptx: ole`<br>`mjx-pptx: activex` | 42 | 128 | — |
| `vml-legacy` | VML — the legacy drawing markup, with shape-level references | `preserved-not-rendered` | `mjx-vml: drawing`<br>`mjx-pptx: vml` | 33 | 117 | — |
| `pptx-animation-and-timing` | Animation and timing — `p:timing`, the trigger tree, and the runtime that evaluates it | `not-started` | **none** | 0 | 0 | — |
| `pptx-transitions` | Transitions — ~48 types with their effect options and advance rules | `not-started` | **none** | 0 | 0 | — |
| `pptx-slide-show` | Slide show — presenter view, rehearsed timings, custom shows, hidden slides | `not-started` | **none** | 0 | 0 | — |
| `pptx-sections-and-morph` | Sections, slide sorter, zoom links and morph | `not-started` | **none** | 0 | 0 | — |

### §5 · The calculation engine

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `calculation-engine` | The calculation engine — the dependency graph, ~500 functions, dynamic arrays, the error lattice — *excluded:* `PLAN.md`'s *Explicitly out of scope for v1* names it, and `crates/mjx-xlsx/docs/guide/deliberate_limitations.md` gathers what follows. The inventory §5 treats it as its own programme rather than a feature; it gates Excel **editing** only, since rendering an unedited workbook uses the cached `<v>` already in the file | `out-of-scope` | **none** | 0 | 0 | — |
| `cached-values-are-rendered` | A formula's cached `<v>` reaching the screen, which is what makes a viewer possible without an engine | `implemented` | `mjx-sml: formulas`<br>`mjx-xlsx: formulas`<br>`mjx-layout-xlsx: a_cell_shows_its_formatted_value` | 31 | 140 | — |

### §6 · The third axis — behaviour with neither markup nor a button

| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |
|---|---|---|---|---:|---:|---|
| `hit-testing` | Hit testing — the spatial index that makes it a query rather than a walk | `implemented` | `mjx-layout: spatial_index`<br>`mjx-layout: fragment_tree` | 15 | 126 | — |
| `selection-and-caret` | Selection and caret — grapheme granularity, and a point resolving to a document position | `implemented` | `mjx-layout: text_composition`<br>`mjx-layout-pptx: a_point_finds_its_run` | 15 | 77 | — |
| `undo-and-redo` | Undo and redo — command granularity, and the coalescing of consecutive keystrokes | `implemented` | `mjx-session: undo_granularity`<br>`mjx-session: batching`<br>`mjx-session: journal_allocation` | 28 | 101 | — |
| `document-lifecycle` | Open, autosave, crash recovery, dirty tracking, and the residency budget a session holds | `implemented` | `mjx-session: recovery`<br>`mjx-session: fidelity`<br>`mjx-session: residency_budget` | 17 | 53 | — |
| `viewport-and-scrolling` | The viewport — windowing, invalidation, scroll stability and the frame schedule | `implemented` | `mjx-view: windowing`<br>`mjx-view: invalidation`<br>`mjx-view: scroll_stability`<br>`mjx-view: frame_schedule` | 25 | 120 | — |
| `memory-budgets` | The declared memory budgets — resident documents, the glyph atlas, the mesh and texture pools | `implemented` | `mjx-view: resident_memory`<br>`mjx-view: the_declared_budgets_add_up`<br>`mjx-text: glyph_atlas_allocation`<br>`mjx-scene: the_mesh_cache_holds_its_budget`<br>`mjx-paint: the_texture_pool_holds_its_budget` | 3 | 33 | — |
| `the-fidelity-oracle` | The oracle — three assertion tiers, failing independently, over approved baselines | `implemented` | `mjx-render-oracle: the_tiers_fail_independently`<br>`mjx-render-oracle: an_unapproved_baseline_fails`<br>`mjx-render-oracle: a_regression_arrives_with_its_picture`<br>`mjx-render-oracle: regenerating_a_baseline_is_explicit` | 17 | 77 | — |
| `the-reference-pack` | The reference pack — the artefacts one Windows sitting needs, and the ingest that reads them back | `implemented` | `mjx-reference-pack: the_instructions_are_complete`<br>`mjx-reference-pack: the_decks_generate_reproducibly`<br>`mjx-reference-pack: the_office_corpus_ships_empty`<br>`mjx-reference-pack: the_readers_answer_from_a_real_export` | 19 | 74 | — |
| `the-box-model-seam` | The seam — above a `FragmentTree`, nothing has heard of OOXML, and a foreign box model works | `implemented` | `mjx-layout: the_seam_holds`<br>`mjx-layout: foreign_box_model`<br>`mjx-scene: fragments_alone_drive_the_builder`<br>`mjx-paint: the_seam_holds` | 28 | 109 | — |
| `design-tokens` | One design-token source reaching three generated consumers, which agree with each other | `implemented` | `mjx-tokens: artefacts_agree` | 4 | 12 | — |
| `input-and-ime` | Input — pointer, touch, pen with pressure, the shortcut map, and IME composition | `not-started` | **none** | 0 | 0 | — |
| `editing-feel` | Editing feel — drag with snapping, live resize previews, marching ants, fling scrolling | `not-started` | **none** | 0 | 0 | — |
| `accessibility-tree` | An accessibility tree derived from the fragment tree, and keyboard-only operation | `not-started` | **none** | 0 | 0 | — |
| `responsive-chrome` | Desktop, tablet and phone presentations of the same commands, and command demotion | `not-started` | **none** | 0 | 0 | — |
| `localisation` | UI strings, RTL mirroring of the chrome, and locale-aware numbers, dates and typography | `not-started` | **none** | 0 | 0 | — |
