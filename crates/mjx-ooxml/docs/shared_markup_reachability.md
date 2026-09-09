# Shared-markup reachability — the same idea, reachable from all three formats

Five crates in this workspace hold markup that is **not** any one format's: `mjx-dml` (DrawingML),
`mjx-sml` (SpreadsheetML), `mjx-chart` (ChartML), `mjx-vml` (VML) and `mjx-omml` (OMML). A chart is
the clearest case — the same `c:chartSpace` sits inside a `.pptx`, a `.docx` and an `.xlsx`, written
by the same code — but it is not the only one: an embedded workbook is SpreadsheetML inside a
presentation, a watermark is VML inside a Word header, and a theme is DrawingML in all three.

That raises one question this page answers, and a test enforces:

> **Is anything in those five crates reachable from one format's facade surface but not another's —
> and if so, is the difference written down?**

An asymmetry is not a bug. `add_range_chart` belongs to [`Workbook`](crate::Workbook) alone because
only a workbook has a worksheet for a series to point at. What *is* a bug is an asymmetry nobody
decided: a capability that exists on [`Deck`](crate::Deck) because PowerPoint was built first, and is
missing from `Workbook` because nobody looked. This page is the list, with a reason for every
difference, and `crates/mjx-ooxml/tests/shared_markup_reachability.rs` fails the build when the list
and the code disagree.

## 1 · Which format crate models which shared markup

The tier below the facade, read straight out of the three format crates' `Cargo.toml` — a
dependency here means the crate models that markup rather than preserving it as bytes.

| | `mjx-dml` | `mjx-sml` | `mjx-chart` | `mjx-vml` | `mjx-omml` |
|---|---|---|---|---|---|
| `mjx-pptx` | ✓ | ✓ | ✓ | ✓ | — |
| `mjx-docx` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `mjx-xlsx` | ✓ | ✓ | ✓ | ✓ | — |

Two entries are worth reading twice. `mjx-pptx → mjx-vml` is **optional**, behind that crate's `vml`
feature, and it is the only optional edge in the grid — a PresentationML caller may open a thousand
decks and never meet an OLE fallback, where a Word header and an Excel comment both carry VML as a
matter of course. And `mjx-omml` has exactly one consumer: OMML appears in WordprocessingML, and
neither `pml.xsd` nor `sml.xsd` declares it, so there is nothing for the other two to model.

`mjx-pptx → mjx-sml` and `mjx-docx → mjx-sml` are not accidents of layering either: a chart in a
deck or a document carries an **embedded workbook part**, which is SpreadsheetML, and MJXOFF-99
deleted the second writer that used to exist for it so that all three formats emit one.

## 2 · Which shared-markup vocabulary each facade surface names

### How this table is derived — the exact rule the test applies

The test reads the facade's own source, and nothing else:

1. `src/lib.rs` re-exports the vocabulary of each shared-markup crate in its own
   `pub use mjx_dml::{…}` / `pub use mjx_sml::{…}` / `pub use mjx_chart::{…}` block. Those blocks
   are the **shared-markup type set**. (`mjx_vml` and `mjx_omml` have no such block; that is a fact
   the test asserts too — see [vml-omml](#vml-omml).)
2. Every `pub fn` in `src/deck.rs` + `src/deck/*.rs`, `src/document.rs` + `src/document/*.rs` and
   `src/workbook.rs` + `src/workbook/*.rs` is read with its signature.
3. **A method is a shared-markup capability when its signature names one of those types.** That is
   the whole rule, and it is deliberately mechanical: it measures what a caller can *say* in
   shared-markup vocabulary through each surface, which is exactly the claim above.
4. Methods with the same name on more than one surface are the same capability. That is the
   convention this facade already follows — `set_chart_axis_title` is `set_chart_axis_title` on all
   three — and `crates/mjx-ooxml/tests/chart_surface_parity.rs` proves the three answer the same
   thing about the same bytes, which is the half a name comparison cannot see.

The three marks:

| | meaning |
|---|---|
| ✓ | the surface has this method **and** its signature names shared-markup vocabulary |
| ≈ | the surface has a method of this name, but stated in **its own format's** vocabulary — the capability is there, the shared type is not |
| — | the surface has no method of this name |

What the rule does *not* catch, stated so the table is not read as more than it is: a capability
whose signature happens to name no shared type at all. `Document::add_inline_picture` writes a
`pic:pic` and takes `Vec<u8>` and four `i64`s, so it is nowhere below even though DrawingML is what
it produces; the same goes for every `*_part_bytes` accessor that hands over preserved markup
verbatim. **The table is about the reachable vocabulary, not about every byte a method emits.**

| Capability | Shared markup | `Deck` | `Document` | `Workbook` | Reason |
|---|---|---|---|---|---|
| `add_chart` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `add_chart_trendline` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `add_floating_chart` | `mjx-chart` | — | ✓ | — | [chart-wrap](#chart-wrap) |
| `add_range_chart` | `mjx-chart` | — | — | ✓ | [chart-range](#chart-range) |
| `append_border` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `append_cell_format` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `append_font` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `append_pattern_fill` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `cell_anchor` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `cell_border` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `cell_end_run_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `cell_fill` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `cell_paragraph_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `cell_run_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `cell_text_direction` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `chart_axes` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_dangling_decoration` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_data_label_tier` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_data_labels` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_error_bars` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_kinds` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_legend` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_point_formats` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_series` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_series_fill` | `mjx-dml` | ✓ | ✓ | ✓ | — |
| `chart_series_from_cells` | `mjx-chart` | — | — | ✓ | [chart-range](#chart-range) |
| `chart_series_references` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `chart_trendlines` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `clear_cell_border` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `clear_shape_list_style_level` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `color_map` | `mjx-dml` | ✓ | — | — | [dml-theme](#dml-theme) |
| `column_width` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `effective_cell_border` | `mjx-dml` | ✓ | ≈ | — | [dml-table](#dml-table) |
| `effective_cell_fill` | `mjx-dml` | ✓ | ≈ | — | [dml-table](#dml-table) |
| `effective_cell_format` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `effective_cell_run_properties` | `mjx-dml` | ✓ | ≈ | — | [dml-table](#dml-table) |
| `effective_merged_cell_format` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `effective_paragraph_properties` | `mjx-dml` | ✓ | ≈ | — | [dml-text](#dml-text) |
| `effective_run_properties` | `mjx-dml` | ✓ | ≈ | — | [dml-text](#dml-text) |
| `effective_shape_effects` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `effective_shape_fill` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `effective_shape_outline` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `effective_shape_transform` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `end_run_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `format_cell_paragraphs` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `format_cell_text` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `format_inline_table_style_part` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `format_table_style_part` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `paragraph_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `remove_chart_data_labels` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `resolved_scheme_color` | `mjx-dml` | ✓ | — | — | [dml-theme](#dml-theme) |
| `row_height` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `run_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_cell_anchor` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_border` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_end_run_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_fill` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_paragraph_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_paragraph_run_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_run_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_run_properties_all` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_text_direction` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_cell_text_range_properties` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_chart_axis_orientation` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `set_chart_data_labels` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `set_chart_error_bars` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `set_chart_legend` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `set_chart_point_fill` | `mjx-dml` | ✓ | ✓ | ✓ | — |
| `set_chart_point_line` | `mjx-dml` | ✓ | ✓ | ✓ | — |
| `set_chart_series_fill` | `mjx-dml` | ✓ | ✓ | ✓ | — |
| `set_chart_series_line` | `mjx-dml` | ✓ | ✓ | ✓ | — |
| `set_chart_trendline` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `set_column_width` | `mjx-dml` | ✓ | — | ≈ | [dml-table](#dml-table) |
| `set_end_run_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_paragraph_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_paragraph_run_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_row_height` | `mjx-dml` | ✓ | — | ≈ | [dml-table](#dml-table) |
| `set_run_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_shape_3d_properties` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `set_shape_effects` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `set_shape_fill` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `set_shape_list_style_default` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_shape_list_style_level` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_shape_outline` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `set_shape_run_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_shape_scene_3d` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `set_shape_transform` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `set_table_part` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `set_text_range_properties` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `set_text_range_properties_by_grapheme` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `shape_3d_properties` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_adjustments` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_backdrop` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_effects` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_fill` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_list_style_default` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `shape_list_style_level` | `mjx-dml` | ✓ | — | — | [dml-text](#dml-text) |
| `shape_outline` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_scene_3d` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `shape_transform` | `mjx-dml` | ✓ | — | — | [dml-shape](#dml-shape) |
| `suppress_chart_data_labels` | `mjx-chart` | ✓ | ✓ | ✓ | — |
| `table_part` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `table_style_flags` | `mjx-dml` | ✓ | — | — | [dml-table](#dml-table) |
| `table_style_origin` | `mjx-sml` | — | — | ✓ | [sml-styles](#sml-styles) |
| `theme` | `mjx-dml` | ✓ | — | — | [dml-theme](#dml-theme) |

`mjx-chart` is the crate this programme set out to share, and it is the one that came out symmetric:
of the 28 chart capabilities above, **25 are on all three surfaces**, and the three that are not each
have a reason in the file format rather than in this library. Read the other way round that is the
sharper statement — **every capability this table finds on all three surfaces is a chart
capability**, and every other row is an asymmetry with a note beside it.

## 3 · The reasons

Seven notes cover every difference above. Each says *why* the difference is in the file format or in
a recorded decision — not "not implemented yet", which is the sentence this page exists to refuse.

### dml-shape

**DrawingML shape properties (fill, outline, effects, transform, 3-D, adjustments) reach `Deck`
alone.** Not because Word and Excel have no shapes — they do, and `mjx-docx` already models a
`wp:spPr` as `mjx_dml::ShapeProperties` (`crates/mjx-docx/src/document/drawing.rs`) — but because
`mjx_dml::ShapeProperties` is an **interner-bound fidelity model**, and this facade's boundary is
interner-free: `src/lib.rs` says four families are deliberately absent precisely because they cannot
be used without an `Interner`. PowerPoint crosses that boundary because `mjx-pptx` builds an
interner-free *spec* layer over the model — [`FillSpec`](crate::FillSpec),
[`LineSpec`](crate::LineSpec), [`EffectListSpec`](crate::EffectListSpec),
[`Transform2D`](crate::Transform2D) — and reads and writes it through `Presentation`. Neither
`mjx-docx` nor `mjx-xlsx` has such a layer over its own drawings.

So the honest statement is two-part: *the facade cannot expose Word's or Excel's shape properties
today, and the missing piece is a spec layer in those crates rather than a method here.* Of every
row in this table this is the one a future ticket would most plausibly close; it is written here
rather than left as an absence nobody had named.

### dml-text

**DrawingML run, paragraph and list-style properties reach `Deck` alone, and should.** A `.pptx`
states character formatting in `a:rPr` and paragraph formatting in `a:pPr` — DrawingML — because a
slide's text lives inside a shape. Word states the same ideas in `w:rPr`/`w:pPr` and Excel in
`styles.xml`'s font records: different vocabularies for one intent, each with its own facade reader.
That is what the `≈` marks on `effective_run_properties` and `effective_paragraph_properties` are:
`Document` answers both questions, in [`EffectiveCharacterProperties`](crate::EffectiveCharacterProperties)
and [`EffectiveParagraphProperties`](crate::EffectiveParagraphProperties) rather than in DrawingML's
`CharacterPropertiesSpec`. Projecting `CharacterPropertiesSpec` onto a Word run would be naming
Word's formatting in PowerPoint's language, which is the opposite of what this workspace's naming
rule asks for.

This is a *symmetry of intent expressed in three vocabularies*, not a gap — and each format's own
ladder now has its own page, in one shape: [`mjx_pptx::effective_properties`],
[`mjx_docx::effective_properties`] and [`mjx_xlsx::effective_properties`].

### dml-table

**`a:tbl` cell, row, column and table-style-part properties reach `Deck` alone.** A PowerPoint table
*is* DrawingML; a Word table is `w:tbl`, a different complex type with its own facade surface — the
`≈` marks on `effective_cell_border`, `effective_cell_fill` and `effective_cell_run_properties` are
`Document` answering the same three questions through Word's own ladder, table-style rung included.
A worksheet's grid is not a table object at all: its nearest equivalents are a `CT_Table` over a
range ([`Workbook::sheet_tables`](crate::Workbook::sheet_tables)) and the row/column geometry the
`≈` marks on `set_row_height` and `set_column_width` point at. Three different constructs that
happen to be drawn as grids, and this is where they part company.

### dml-theme

**`theme` and `color_map` reach `Deck` alone, and this is the table's clearest open gap.** All three
package kinds relate a `theme1.xml`: `mjx-xlsx` names the part (`parts::CONTENT_TYPE_THEME`,
`parts::REL_THEME`) and `mjx-docx` opens one internally to bake theme colours into
`effective_run_properties` — but neither hands a [`ThemeInfo`](crate::ThemeInfo) or a
[`ColorMap`](crate::ColorMap) to a caller, so neither surface can answer *"what is this file's
accent 1?"*.

The consequence is concrete and worth stating rather than burying: `mjx-sml` resolves
`<color theme="N"/>` only as far as the slot number, and says so in
`crates/mjx-sml/src/styles/stylesheet.rs` — *"resolving that needs the theme part, which is
`mjx-xlsx`'s to fetch"* — and `mjx-xlsx` does not fetch it. An Excel theme colour therefore comes
back as a position where the same colour in a `.pptx` comes back resolved to `RRGGBB`. No ticket
owns this; it is recorded here so the next reader finds a decision to take rather than silence.

### sml-styles

**SpreadsheetML's indexed style records — `append_font`, `append_border`, `append_pattern_fill`,
`append_cell_format`, `effective_cell_format`, `effective_merged_cell_format`, `table_style_origin`
— reach `Workbook` alone.** `styles.xml`'s two-level `xf` indirection has no counterpart in the
other two formats: a PowerPoint table style is a GUID into `tableStyles.xml` and a Word style is a
`w:style` in a `w:basedOn` ladder, and both are reachable through their own format's methods.
Exposing [`CellFormatSpec`](crate::CellFormatSpec) on a `Deck` would claim a `.pptx` has a `cellXfs`
table, which it does not.

### chart-wrap

**`add_floating_chart` reaches `Document` alone.** A floating chart is a `wp:anchor`, and its
argument is a [`ChartWrap`](crate::ChartWrap) — how text flows around the drawing. A slide has no
text flow for a shape to displace, and a worksheet anchors a drawing to cells (a
[`ResizingBehavior`](crate::ResizingBehavior)) rather than to a paragraph. The concept does not exist
in the other two languages, so neither does the method.

### chart-range

**`add_range_chart` and `chart_series_from_cells` reach `Workbook` alone.** These author a chart
whose series are live references into a worksheet of the *same* package — `Sheet1!$B$2:$B$5` — which
requires that worksheet to be there. A PowerPoint or Word chart carries an **embedded** workbook part
instead ([`ChartWorkbook`](crate::ChartWorkbook),
[`DocumentChartWorkbook`](crate::DocumentChartWorkbook)) and its series are literal caches; there is
no sheet in the package for a range to name. The file format's arrangement, not this library's.

### vml-omml

**`mjx-vml` and `mjx-omml` reach no facade surface as *types*, and the reason is the same for both:**
their public models — `mjx_vml::Drawing` and `mjx_omml::Math`/`MathParagraph` — are interner-bound
trees, and this facade's contract is that every type it names can be constructed and destructured
across a foreign-function boundary without an `Interner`. Neither has a flat, binding-friendly
projection the way a chart has [`ChartData`](crate::ChartData) or a fill has
[`FillSpec`](crate::FillSpec). Neither appears in `src/lib.rs`'s re-export list, which is what the
test checks.

What each surface *does* reach is the preserved bytes and the identifiers, and here the three are
**not** symmetric:

| | `Deck` | `Document` | `Workbook` |
|---|---|---|---|
| The VML part's bytes | `vml_part_names`, `vml_part_bytes`, `vml_drawing_part`, `add_vml_drawing` | — | `sheet_vml_part_bytes` |
| An identifier out of the VML | `ole_legacy_shape_id`, `activex_control_shape_id` | — | `vml_shape_id_for_ole_object`, `vml_shape_id_for_form_control` |
| OMML | — (`mjx-pptx` models none) | — | — (`mjx-xlsx` models none) |

**Word's VML is reachable from `mjx-docx` and from nowhere on this facade**, and that is not a
decision anyone recorded before this page. The proximate reason is real:
`mjx_docx::Document::header_footer_vml_drawings` hands back a `Vec<mjx_vml::Drawing>` **without the
interner those values' own accessors take**, where `mjx_pptx::Presentation::with_vml_drawing` and
`mjx_xlsx::Workbook::vml_drawing_markup` both hand the model to a closure together with its
interner. There is therefore nothing on the Word side shaped like the thing the other two project.
That is a defect in `mjx-docx`'s API shape rather than a property of WordprocessingML, it is
reported as such by MJXOFF-118, and it is **not** fixed here: this page and its test are a read and
a rename pass, and a signature change to close it belongs in a commit a reviewer can read on its own.

`crates/mjx-xlsx/docs/guide/cell_comments_and_legacy_content.md` carries the full legacy-content
comparison one layer down, of which the grid above is the facade's own row.

## 4 · When this page and the code disagree

`crates/mjx-ooxml/tests/shared_markup_reachability.rs` re-derives both tables on every `cargo test` —
the first from the three format crates' `Cargo.toml`, the second from the facade's own source — and
compares them to what is written above, row for row and mark for mark. A new facade method that
names a shared-markup type, a method removed, a method added to one surface and not the others, a
format crate that starts or stops modelling a shared markup, or a note that no row cites any more:
each fails that test naming the row it is about.

That is the whole point. **A written reason that no longer matches the code becomes a build failure
rather than stale prose.**

[`mjx_pptx::effective_properties`]: https://docs.rs/mjx-pptx/latest/mjx_pptx/effective_properties/
[`mjx_docx::effective_properties`]: https://docs.rs/mjx-docx/latest/mjx_docx/effective_properties/
[`mjx_xlsx::effective_properties`]: https://docs.rs/mjx-xlsx/latest/mjx_xlsx/effective_properties/
