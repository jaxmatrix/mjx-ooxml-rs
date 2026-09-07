# Documents — the checks

Six areas, `V-DOCX-01` … `V-DOCX-06`. `docs/validation/00-method.md` states the id scheme, the risk
levels and the result convention; `docs/validation/01-index.md` binds each area to its artefact;
`docs/validation/02-risk-order.md` says which of these to do first and why.

**Nothing on this page is marked.** Every `Result:` line ships unfilled.

## How these six areas were derived

Not from the twenty Word tickets, whose status text is a claim rather than evidence. From two things
that shipped:

* **The facade's own module structure.** `crates/mjx-ooxml/src/document/` is fifteen files, and six
  of them are what a person can *see* in Word: `text.rs` + `effective.rs` + `styles.rs` +
  `numbering.rs` (`V-DOCX-01`), `sections.rs` + `headers.rs` (`V-DOCX-02`), `tables.rs`
  (`V-DOCX-03`), `charts.rs` (`V-DOCX-04`), `drawings.rs` (`V-DOCX-05`), and `notes.rs` +
  `comments.rs` + `hyperlinks.rs` + `fields.rs` (`V-DOCX-06`). `paths.rs` and `document.rs` are
  addressing and the package itself, which every area exercises and none owns.
* **The gaps page's own *Built, not yet verified against Office* list.**
  `crates/mjx-docx/docs/guide/fidelity_and_gaps.md` has eight rows there and names this child as
  their reader. Every one of them appears below.

Where the two disagree the surface wins, because a page can be stale and a `pub fn` cannot.

## Coverage, against the gaps page

| Area | Risk | The gaps page says | What that means here |
|---|---|---|---|
| `V-DOCX-01` | high | *Built, not yet verified* — **the effective-properties ladder order** and **the toggle-property XOR rule** | `V-DOCX-01.2` and `V-DOCX-01.3`. The XOR rule is called out on that page as *the single most surprising answer this library gives, and the one most worth checking against the real renderer first* |
| `V-DOCX-01` | high | *Non-goal* — **a list's displayed number is not computed** | Non-goal. `V-DOCX-01.6` checks the definition resolves, never the rendered "3.2.1" |
| `V-DOCX-01` | high | *Non-goal* — **tracked changes are read, never applied** | Non-goal. `V-DOCX-01.7` checks the *answer*, not an accept-all |
| `V-DOCX-02` | medium | *Built, not yet verified* — **header and footer resolution** | `V-DOCX-02.2`, and the `None`-rather-than-fabricate decision is the interesting half |
| `V-DOCX-03` | high | *Non-goal* — **a table inside a block-level content control is not a top-level table** | Non-goal, and `V-DOCX-03.4` is where a reviewer would otherwise report it as a defect |
| `V-DOCX-04` | medium | *Non-goal* — **charts and SmartArt inside a `w:drawing` are preserved through `a:graphicData`'s unknown bucket** in `mjx-docx` | Reached through the facade instead, which is `MJXOFF-103`'s doing; the rendering question is the same one `V-PPTX-04` asks |
| `V-DOCX-05` | low | neither list | Plain modelled markup |
| `V-DOCX-06` | medium | *Built, not yet verified* — **field instruction editing** and **the `w:altChunk` import** | `V-DOCX-06.4`; the `altChunk` half is **blocked**, because the facade has no `add_alt_chunk` |
| `V-DOCX-06` | medium | *Non-goal* — **a field is never evaluated or refreshed** | Non-goal. The check is that the instruction and the cached result are never confused for one another |
| every area | — | *Built, not yet verified* — **every fixture is hand-crafted**, **`Document::blank`'s part set**, **everything this crate authors** | R2, and `V-DOCX-01.5` for the part set |

## `V-DOCX-01` · `text-and-inheritance` — paragraphs, runs, and the character properties they inherit

Risk **high**. Shipped by `MJXOFF-92` (the block content model), `MJXOFF-94` (run properties),
`MJXOFF-96` (paragraph properties), `MJXOFF-101` (styles), `MJXOFF-104` (numbering) and
`MJXOFF-106` (the ladder).

#### V-DOCX-01.1 — the four paragraphs render, and the file opens

- **Risk** low.
- **Shipped by** `MJXOFF-92`, on `MJXOFF-98`'s `Document::blank`.
- **Artefact** `v-docx-01-authored.docx`
- **Object** the four paragraphs: *Quarterly Review*, *This paragraph states nothing…*, *North America: +12%*, *EMEA: +8%*.
- **Action** open it in Word.
- **Expect** **no "unreadable content" prompt**, four paragraphs, in that order, with that text.
  Calls: `Document::append_paragraph` · `Document.append_paragraph` · `Document.appendParagraph`
  Calls: `Document::append_run` · `Document.append_run` · `Document.appendRun`
  Result: — · — · — · —

#### V-DOCX-01.2 — the effective-properties ladder, in Word's own order

- **Risk** high — the first of Word's five risk areas.
- **Shipped by** `MJXOFF-106`, from ECMA-376 Part 1 §17.7.2's prose — **and against a ticket that asserted the opposite order**.
- **Artefact** `tests/fixtures/effective_properties.docx`
- **Object** a run whose value for one property is contributed by a different rung at each of the six tiers: `docDefaults` → table style → numbering → paragraph style → character style → direct.
- **Action** click into the run and read Home → Font, and the Styles pane's *Reveal Formatting*.
- **Expect** the rung `effective_run_properties` names is the rung Word says the value came from, at every tier. The spec is unambiguous here; **Word's agreement with it is assumed and has never been watched happen.**
  Calls: `Document::effective_run_properties` · `Document.effective_run_properties` · `Document.effectiveRunProperties`
  Calls: `Document::effective_paragraph_properties` · `Document.effective_paragraph_properties` · `Document.effectiveParagraphProperties`
  Result: — · — · — · —

#### V-DOCX-01.3 — the toggle-property XOR rule

- **Risk** high — *"the single most surprising answer this library gives"*, in the gaps page's own words.
- **Shipped by** `MJXOFF-106`, from ECMA-376 Part 1 §17.7.3.
- **Artefact** `tests/fixtures/style_based_on_chain.docx`
- **Object** a run whose paragraph style and character style **both** say bold.
- **Action** read what Word renders.
- **Expect** §17.7.3's twelve toggle properties combine by **XOR**, so two rungs that both say bold render **not bold**. This is implemented from the prose and unit-tested against it. If Word renders it **bold**, this is the highest-value finding the Word half of the pass can make, and the decision that follows — follow §17.7.3 or follow Word — is the user's.
  Calls: `Document::effective_run_properties` · `Document.effective_run_properties` · `Document.effectiveRunProperties`
  Calls: `Document::style_ids` · `Document.style_ids` · `Document.styleIds`
  Result: — · — · — · —

#### V-DOCX-01.4 — a `basedOn` chain, and a `basedOn` cycle

- **Risk** medium.
- **Shipped by** `MJXOFF-101`.
- **Artefact** `tests/fixtures/style_based_on_chain.docx`, `tests/fixtures/style_based_on_cycle.docx`, `tests/fixtures/style_latent_styles.docx`
- **Object** the chain, the deliberate cycle, and the latent-style block.
- **Action** open all three in Word.
- **Expect** the chain resolves to the same values this library reports. The **cycle** file opens without a repair — a cycle is the file's own statement, and nothing here repairs it. The latent styles appear in Word's Styles pane as latent, not as defined styles.
  Calls: `Document::style_name` · `Document.style_name` · `Document.styleName`
  Calls: `Document::style_ids` · `Document.style_ids` · `Document.styleIds`
  Result: — · — · — · —

#### V-DOCX-01.5 — `Document::blank`'s part set, and the margins it fixes

- **Risk** medium.
- **Shipped by** `MJXOFF-98`.
- **Artefact** `v-docx-01-authored.docx`
- **Object** the package: `word/document.xml` and both `docProps` parts, and **deliberately no** `styles.xml`, `numbering.xml`, `settings.xml`, `fontTable.xml` or `theme1.xml`.
- **Action** open it, read Layout → Margins, then Layout → Size.
- **Expect** it opens with **no repair prompt** despite carrying none of those five parts. Margins are Word's **"Normal" template default** regardless of page size — they are fixed, not derived — and a page those margins do not fit inside is refused by `DocxError::InvalidPageSize` rather than written for Word to repair.
  Calls: `Document::blank` · `Document.blank` · `Document.blank`
  Calls: `Document::validate` · `Document.validate` · `Document.validate`
  Result: — · — · — · —

#### V-DOCX-01.6 — a numbering definition resolves, and a displayed number is not computed

- **Risk** high — an interrogation of a **documented non-goal**, and the place a known defect lives.
- **Shipped by** `MJXOFF-104`.
- **Artefact** `tests/fixtures/numbering_definitions.docx`
- **Object** a paragraph carrying a `w:numPr`, and the `w:num` → `w:abstractNum` chain behind it.
- **Action** read the level, format, template and start value this library reports; then read the number Word actually draws.
- **Expect** the definition resolves. The **displayed** number is a documented non-goal — "3.2.1" depends on every preceding paragraph in reading order, the restart rules at each level and the `w:lvlOverride`s along the way, which is a walk of the whole document — so a difference between the two is **not** a finding. **Note for the reviewer:** `Document::attach_paragraph_to_list` writes a `w:numPr` and nothing in this workspace authors a `word/numbering.xml` for it to point at, so a document built from `Document::blank` with a list attached carries a dangling reference; `Document::effective_run_properties` raises *no `w:num` with numId 1* on it. That is why `V-DOCX-01`'s artefact writes no list.
  Calls: `Document::attach_paragraph_to_list` · `Document.attach_paragraph_to_list` · `Document.attachParagraphToList`
  Calls: `Document::effective_paragraph_properties` · `Document.effective_paragraph_properties` · `Document.effectiveParagraphProperties`
  Result: — · — · — · —

#### V-DOCX-01.7 — revision marks survive an edit

- **Risk** high — the second of Word's five risk areas.
- **Shipped by** `MJXOFF-126`.
- **Artefact** `v-docx-01-authored.docx` — **the reviewer makes the input**: no committed fixture carries a revision mark, and the facade cannot author one. In Word, turn **Track Changes on**, edit two of the four paragraphs, and save.
- **Object** the `w:ins` and `w:del` runs Word wrote, and the `w:rPrChange` on a reformatted run.
- **Action** read the saved file back through this library; then make an unrelated edit through this library and reopen in Word.
- **Expect** `revisions` reports every mark Word wrote, with its author and its kind. `text_with_revisions_accepted` and `_rejected` answer *what the text would be* — neither rewrites anything, because **accepting a revision is a document rewrite this library deliberately does not perform**. After the unrelated edit, Word still shows every mark, in the same place, attributed to the same author.
  Calls: `Document::revisions` · `Document.revisions` · `Document.revisions`
  Calls: `Document::text_with_revisions_accepted` · `Document.text_with_revisions_accepted` · `Document.textWithRevisionsAccepted`
  Calls: `Document::text_with_revisions_rejected` · `Document.text_with_revisions_rejected` · `Document.textWithRevisionsRejected`
  Result: — · — · — · —

#### V-DOCX-01.8 — the walkthrough document, in three languages

- **Risk** medium.
- **Shipped by** `MJXOFF-139`, projected by the two bindings.
- **Artefact** `crates/mjx-ooxml/examples/build_a_document.rs` — `cargo run -p mjx-ooxml --example build_a_document`, and the Python and Node copies beside it.
- **Object** the document each of the three writes.
- **Action** open one of the three in real Word and read it against what the walkthrough describes.
- **Expect** it renders as the walkthrough says. **Note:** the shipped example attaches a paragraph to a list whose `w:num` nothing defines — the shape `V-DOCX-01.6` describes — so if Word draws no number there, that is the known omission and not a new finding.
  Calls: `Document::save` · `Document.save` · `Document.save`
  Result: — · — · — · —

## `V-DOCX-02` · `sections-and-headers` — page size and margins, headers and footers

Risk **medium**. Shipped by `MJXOFF-109` (sections) and `MJXOFF-113` (headers, footers and the
legacy VML they carry).

#### V-DOCX-02.1 — a landscape US Letter section with Normal margins

- **Risk** low.
- **Shipped by** `MJXOFF-109`.
- **Artefact** `v-docx-02-authored.docx`
- **Object** the body section's `w:pgSz`, which is `w="15840" h="12240" orient="landscape"`.
- **Action** Layout → Size, Layout → Orientation, Layout → Margins.
- **Expect** **Letter**, **Landscape** — 11 × 8.5 in — with **Normal** margins. Note that the width and height are *swapped* by the landscape helper rather than only the `@orient` attribute being set; a page that reads 8.5 × 11 with `orient="landscape"` is what Word repairs.
  Calls: `Document::set_section_page_size` · `Document.set_section_page_size` · `Document.setSectionPageSize`
  Calls: `Document::set_section_page_margins` · `Document.set_section_page_margins` · `Document.setSectionPageMargins`
  Result: — · — · — · —

#### V-DOCX-02.2 — which header variant Word actually picks

- **Risk** high — the third of Word's five risk areas.
- **Shipped by** `MJXOFF-113`, from ECMA-376 Part 1 §17.10.1.
- **Artefact** `v-docx-02-authored.docx`, and `tests/fixtures/header_footer_variants.docx` for the three-variant case
- **Object** in the artefact: a **default** header reading *"Quarterly Review — Internal"*, a **first-page** header reading *"First page header"*, and a **default** footer reading *"Page footer, default type"*. In the fixture: all three of `header1`, `header2` and `header3` with their `w:titlePg` and `w:evenAndOddHeaders` flags.
- **Action** in the artefact, look at page 1's header and page 2's header. In the fixture, look at pages 1, 2 and 3.
- **Expect** page 1 shows *First page header* and page 2 shows *Quarterly Review — Internal*; the footer is the same on both. In the fixture, the variant Word draws on each page is the one `header_text` answers for that `HeaderFooterType`. **And the interesting half:** where no section back to the first states a reference, this library answers **`None`** rather than fabricating the blank header Word would create — so a page Word shows an empty header on must correspond to a `None`, not to an empty string.
  Calls: `Document::set_header_text` · `Document.set_header_text` · `Document.setHeaderText`
  Calls: `Document::header_text` · `Document.header_text` · `Document.headerText`
  Calls: `Document::even_and_odd_headers` · `Document.even_and_odd_headers` · `Document.evenAndOddHeaders`
  Result: — · — · — · —

#### V-DOCX-02.3 — a header carrying legacy VML

- **Risk** medium.
- **Shipped by** `MJXOFF-113`.
- **Artefact** `tests/fixtures/header_watermark.docx`
- **Object** the `w:pict` watermark in the header — VML, which is where VML still routinely appears in the wild.
- **Action** open the document and look at the watermark on every page.
- **Expect** it draws. VML geometry is **preserved, not evaluated** — the `path` command string and `v:formulas` come back verbatim — so the only claim is that Word can still draw it after a round trip.
  Calls: `Document::header_text` · `Document.header_text` · `Document.headerText`
  Result: — · — · — · —

#### V-DOCX-02.4 — three sections, each with its own page setup

- **Risk** medium.
- **Shipped by** `MJXOFF-109`.
- **Artefact** `tests/fixtures/three_section_document.docx`
- **Object** the three `w:sectPr`s and the breaks between them.
- **Action** open it and step through the sections in Layout → Breaks.
- **Expect** three sections, each with the page setup `sections` reports for it, and the section breaks where the markup puts them.
  Calls: `Document::sections` · `Document.sections` · `Document.sections`
  Calls: `Document::section_count` · `Document.section_count` · `Document.sectionCount`
  Result: — · — · — · —

## `V-DOCX-03` · `tables` — horizontal spans and vertical merges

Risk **high**. Shipped by `MJXOFF-116` (the grid) and `MJXOFF-119` (properties, styles and
conditional formatting).

#### V-DOCX-03.1 — a horizontal span and a vertical merge, spelled differently

- **Risk** high.
- **Shipped by** `MJXOFF-116`.
- **Artefact** `v-docx-03-authored.docx`
- **Object** the 3 × 3 table. The last row's second cell carries `w:gridSpan w:val="2"`; the first column carries `w:vMerge w:val="restart"` on row 1 and a bare `w:vMerge` on row 2.
- **Action** click each merged region and read Layout → the selection's extent.
- **Expect** the span covers the **last two columns of the last row**; the vertical merge covers **rows 1 and 2 of the first column**. WordprocessingML spells the two merge shapes differently and a reader that conflated them would report the same answer for both.
  Calls: `Document::set_cell_span` · `Document.set_cell_span` · `Document.setCellSpan`
  Calls: `Document::set_cell_vertical_merge` · `Document.set_cell_vertical_merge` · `Document.setCellVerticalMerge`
  Calls: `Document::cell_span` · `Document.cell_span` · `Document.cellSpan`
  Result: — · — · — · —

#### V-DOCX-03.2 — a ragged table, held rather than repaired

- **Risk** medium.
- **Shipped by** `MJXOFF-116`.
- **Artefact** `tests/fixtures/ragged_table.docx`
- **Object** a table whose rows do not all have the same number of grid columns.
- **Action** open it in Word and look at the table's shape.
- **Expect** it opens without a repair. `table_grid_discrepancies` reports the ragged rows and **nothing corrects them** — correcting a file to match what this library expects is how a fidelity library loses the argument it exists to win. What Word draws for a ragged row is the answer this check records.
  Calls: `Document::table_grid_discrepancies` · `Document.table_grid_discrepancies` · `Document.tableGridDiscrepancies`
  Calls: `Document::table_dimensions` · `Document.table_dimensions` · `Document.tableDimensions`
  Result: — · — · — · —

#### V-DOCX-03.3 — cell fills and borders, resolved through the table style

- **Risk** high.
- **Shipped by** `MJXOFF-119`.
- **Artefact** `v-docx-03-authored.docx`
- **Object** the header row's cells, and what `effective_cell_fill` and `effective_cell_border` report for each.
- **Action** compare the reported values against Table Design → Borders and Shading for the same cells.
- **Expect** they agree. Word's conditional table formatting (first row, banded rows, first column) is applied by the style, and a resolver that applied the wrong condition gives a plausible wrong answer rather than an obviously wrong one.
  Calls: `Document::effective_cell_fill` · `Document.effective_cell_fill` · `Document.effectiveCellFill`
  Calls: `Document::effective_cell_border` · `Document.effective_cell_border` · `Document.effectiveCellBorder`
  Calls: `Document::effective_cell_run_properties` · `Document.effective_cell_run_properties` · `Document.effectiveCellRunProperties`
  Result: — · — · — · —

#### V-DOCX-03.4 — a table inside a block-level content control

- **Risk** low — a **documented non-goal**, recorded here so it is not filed as a defect.
- **Shipped by** `MJXOFF-138`.
- **Artefact** `tests/fixtures/structured_content.docx`
- **Object** a table wrapped in a block-level `w:sdt`.
- **Action** open it in Word; the table is a table there.
- **Expect** Word shows it. `table_count` does **not**, because it addresses `w:body`'s own content — a single flat index whose numbering changed when somebody wrapped a table in Word would be a worse contract than a documented boundary. Row and cell addressing *does* see through row- and cell-level wrappers. **Do not file this.**
  Calls: `Document::table_count` · `Document.table_count` · `Document.tableCount`
  Result: — · — · — · —

## `V-DOCX-04` · `charts` — inline and floating charts, and their embedded workbooks

Risk **medium**. Shipped by `MJXOFF-103`, on the writer `MJXOFF-99` moved to `mjx-sml`, with the
anchoring from `MJXOFF-131`.

#### V-DOCX-04.1 — an inline chart and a floating one, wrapped square

- **Risk** high — the fifth of Word's five risk areas, *where LibreOffice diverges from Word most visibly*.
- **Shipped by** `MJXOFF-103` over `MJXOFF-131`.
- **Artefact** `v-docx-04-authored.docx`
- **Object** the two charts: a `wp:inline` one titled *Revenue by quarter*, and a `wp:anchor` one titled *The same numbers, wrapped square* with `wp:wrapSquare wrapText="bothSides"`, offset half an inch from its anchor paragraph.
- **Action** click each and read Picture Format → Wrap Text and Position.
- **Expect** the first is **In Line with Text**; the second is **Square**, wrapping on **both sides**, positioned 0.5" right and 0.5" down from the paragraph it is anchored to. This is where LibreOffice's agreement is worth least.
  Calls: `Document::add_chart` · `Document.add_chart` · `Document.addChart`
  Calls: `Document::add_floating_chart` · `Document.add_floating_chart` · `Document.addFloatingChart`
  Result: — · — · — · —

#### V-DOCX-04.2 — *Edit Data* on a chart in a document

- **Risk** high — **R4** in its second host.
- **Shipped by** `MJXOFF-103`, on `MJXOFF-99`'s writer.
- **Artefact** `v-docx-04-authored.docx`
- **Object** the embedded `.xlsx` behind each chart.
- **Action** right-click the inline chart → Edit Data.
- **Expect** the same numbers `V-PPTX-04.1` asks for — `12`, `15.5`, `14`, `19.25` and `10.5`, `13`, `13.75`, `16` over `Q1`…`Q4` — because both hosts draw the same `ChartData` through the same writer. Two hosts disagreeing here would mean the writer is not actually shared.
  Calls: `Document::chart_workbooks` · `Document.chart_workbooks` · `Document.chartWorkbooks`
  Calls: `Document::refresh_chart_workbook` · `Document.refresh_chart_workbook` · `Document.refreshChartWorkbook`
  Result: — · — · — · —

#### V-DOCX-04.3 — a chart in a document Word wrote

- **Risk** medium.
- **Shipped by** `MJXOFF-103`.
- **Artefact** `tests/fixtures/chart_in_word.docx`
- **Object** a chart addressed by its drawing's own `wp:docPr` id.
- **Action** open it, edit the chart's title through this library, and reopen.
- **Expect** the chart still renders and the title changed. The address is the `wp:docPr` id — the same address `MJXOFF-131` uses for a drawing — so a chart addressed by relationship order instead would break the moment a second drawing appeared.
  Calls: `Document::chart_drawing_ids` · `Document.chart_drawing_ids` · `Document.chartDrawingIds`
  Calls: `Document::set_chart_title` · `Document.set_chart_title` · `Document.setChartTitle`
  Result: — · — · — · —

## `V-DOCX-05` · `pictures` — inline pictures

Risk **low**. Shipped by `MJXOFF-131`.

#### V-DOCX-05.1 — two inline pictures, at two sizes

- **Risk** low.
- **Shipped by** `MJXOFF-131`.
- **Artefact** `v-docx-05-authored.docx`
- **Object** the two `w:drawing`s: one **2 × 2 in**, named *Placeholder*; one **1 × 1 in**, named *Placeholder, small*.
- **Action** click each and read Picture Format → Size, and Alt Text.
- **Expect** 2" and 1" square respectively, both **In Line with Text**, with those two names. Note how the image part is declared: `Document` writes an **`Override` on the part name** in `[Content_Types].xml`, where `Deck` writes a `Default Extension="png"`. Both are valid OPC, and this is the one place a reviewer sees the two writers make the same declaration two different ways.
  Calls: `Document::add_inline_picture` · `Document.add_inline_picture` · `Document.addInlinePicture`
  Calls: `Document::remove_drawing` · `Document.remove_drawing` · `Document.removeDrawing`
  Result: — · — · — · —

## `V-DOCX-06` · `notes-comments-links` — footnotes, endnotes, comments and hyperlinks

Risk **medium**. Shipped by `MJXOFF-124` (comments, footnotes, endnotes, bookmarks) and
`MJXOFF-121` (fields, hyperlinks and form fields).

#### V-DOCX-06.1 — a footnote and an endnote on the same paragraph

- **Risk** medium.
- **Shipped by** `MJXOFF-124`.
- **Artefact** `v-docx-06-authored.docx`
- **Object** `word/footnotes.xml` and `word/endnotes.xml`, and the two references on the heading paragraph.
- **Action** References → Show Notes, and look at the bottom of the page and the end of the document.
- **Expect** the footnote reads *"Figures are unaudited and subject to revision."* at the foot of the page; the endnote reads *"Prepared by the validation harness."* at the end. Both parts carry their separator and continuation-separator notes, without which Word repairs the file.
  Calls: `Document::add_footnote` · `Document.add_footnote` · `Document.addFootnote`
  Calls: `Document::add_endnote` · `Document.add_endnote` · `Document.addEndnote`
  Calls: `Document::footnotes` · `Document.footnotes` · `Document.footnotes`
  Result: — · — · — · —

#### V-DOCX-06.2 — a comment, with its author and initials

- **Risk** medium.
- **Shipped by** `MJXOFF-124`.
- **Artefact** `v-docx-06-authored.docx`
- **Object** the comment anchored to the heading paragraph.
- **Action** Review → Show Comments.
- **Expect** one comment by **Reviewer**, initials **R**, reading *"Confirm the North America figure before publishing."*, anchored to the heading. The comment range start and end must bracket the right text; a range whose ids do not match is a repair.
  Calls: `Document::add_comment` · `Document.add_comment` · `Document.addComment`
  Calls: `Document::comment_range_text` · `Document.comment_range_text` · `Document.commentRangeText`
  Calls: `Document::remove_comment` · `Document.remove_comment` · `Document.removeComment`
  Result: — · — · — · —

#### V-DOCX-06.3 — a hyperlink inserted mid-paragraph

- **Risk** low.
- **Shipped by** `MJXOFF-121`.
- **Artefact** `v-docx-06-authored.docx`
- **Object** the paragraph reading *"Full figures: "* with *investor relations page* inserted after it as a link.
- **Action** hover the link, then Ctrl-click it.
- **Expect** the target is `https://example.com/investors`, and only the inserted run is a link — the leading *"Full figures: "* is not.
  Calls: `Document::insert_hyperlink` · `Document.insert_hyperlink` · `Document.insertHyperlink`
  Calls: `Document::hyperlink_target` · `Document.hyperlink_target` · `Document.hyperlinkTarget`
  Result: — · — · — · —

#### V-DOCX-06.4 — the field instruction, and the cached result beside it

- **Risk** high — the fourth of Word's five risk areas.
- **Shipped by** `MJXOFF-121`.
- **Artefact** `tests/fixtures/fields_and_hyperlinks.docx`
- **Object** the `w:fldSimple` and `w:instrText` fields, including one whose instruction is spread across several runs.
- **Action** rewrite an instruction through this library, then open in Word and press Alt+F9 to see the codes, and F9 to update the field.
- **Expect** the instruction Word shows is the one that was written, **as a single run** — an instruction spread over several `w:instrText` runs is rewritten as one. The **cached result** is untouched until Word recomputes it: a field is **never evaluated or refreshed** here, because `TOC`, `PAGE`, `DATE` and `REF` are computed from a renderer and a clock this library does not have. A field whose zone holds a **nested** field is refused rather than flattened.
  Calls: `Document::fields` · `Document.fields` · `Document.fields`
  Calls: `Document::set_field_instruction` · `Document.set_field_instruction` · `Document.setFieldInstruction`
  Calls: `Document::set_field_cached_result_text` · `Document.set_field_cached_result_text` · `Document.setFieldCachedResultText`
  Result: — · — · — · —

#### V-DOCX-06.5 — the `w:altChunk` import, performed by Word

- **Risk** medium.
- **Shipped by** `MJXOFF-138`.
- **Artefact** none — **blocked**: `mjx_docx::Document::add_alt_chunk` is not projected onto the facade, so no artefact this pass generates can carry one, and no committed fixture has one either. Closing this needs either the facade method or an original from `MJXOFF-130`.
- **Object** a `w:altChunk` whose payload part holds an HTML or RTF document.
- **Action** open the document in Word.
- **Expect** Word performs the import and shows the payload as document content. The part, its content type and its relationship are written, and the payload is stored **byte-for-byte** — converting one format into another is not this library's job — so **whether Word performs the import as expected has never been watched happen**.
  Calls: `Document::open` · `Document.open` · `Document.open`
  Result: — · — · — · —

#### V-DOCX-06.6 — everything this crate authors, opened at once

- **Risk** medium.
- **Shipped by** `MJXOFF-124`, `MJXOFF-121`, `MJXOFF-116`, `MJXOFF-131` and `MJXOFF-98`.
- **Artefact** `v-docx-01-authored.docx`, `v-docx-02-authored.docx`, `v-docx-03-authored.docx`, `v-docx-04-authored.docx`, `v-docx-05-authored.docx`, `v-docx-06-authored.docx`
- **Object** all six at once.
- **Action** open each in Word, in order.
- **Expect** **not one repair prompt**. Every one of them is schema-valid against the ECMA-376 XSDs and in child order, and the gaps page says in as many words that *schema-valid is not the same claim as "Word opens it without a repair prompt"*. This entry is where the two claims are finally compared.
  Calls: `Document::validate` · `Document.validate` · `Document.validate`
  Result: — · — · — · —
