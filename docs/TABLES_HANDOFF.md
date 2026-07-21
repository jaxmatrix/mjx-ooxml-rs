# Handoff — tables (`a:tbl`) — IN PROGRESS

The grid a `p:graphicFrame` frames. Read after `docs/PHASE2_HANDOFF.md` (§3 guardrails);
`docs/TRANSFORM_HANDOFF.md` is the immediately preceding workstream and left the graphic frame
positionable, which is what made this one possible.

**Status: B1–B4a shipped, `0.0.17`, 705 tests green.** Four of six tiers are in.

**➡ NEXT: B4b — inserting and removing rows and columns.** See [Roadmap](#roadmap) for what that
has to get right, and why it is the awkward one.

## Why this workstream exists

Tables and speaker notes are the two features between the project and **`v0.1` — PowerPoint
complete**. After the transform workstream a `p:graphicFrame` could be positioned, but everything
inside `a:graphic > a:graphicData > a:tbl` was opaque: a caller could not create a table, read a
cell, write one, or format one.

## What shipped

```rust
// Make one. It is a shape on the ordinary index space, so `set_shape_bounds` moves it and
// `remove_shape` drops it.
let table = deck.add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0))?;
deck.table_dimensions(0, table)?;                       // (rows, columns)
deck.set_column_width(0, table, 1, Emu::from_points(120.0))?;
deck.set_row_height(0, table, 0, Emu::from_points(30.0))?;

// Text in a cell — the whole text surface, addressed at a cell.
deck.set_cell_text(0, table, 0, 0, 0, "Region")?;
deck.cell_paragraph_count(0, table, 0, 0)?;
deck.set_cell_text_range_properties(0, table, 0, 0, 0, 0..6, &bold)?;

// Formatting, per property or per region.
deck.set_cell_border(0, table, 0, 0, CellBorder::Bottom, &rule)?;
deck.format_cells(0, table, Cells::row(0), &CellFormat::new()
    .with_fill(navy)
    .with_border(CellBorder::Bottom, rule)
    .with_anchor(TextAnchoring::Center))?;
deck.format_cell_text(0, table, Cells::row(0), &bold)?;

// Merging.
deck.merge_cells(0, table, Cells::rectangle(0..1, 0..3))?;
deck.unmerge_cells(0, table, 0, 2)?;                    // any cell of the region
```

Per PR:

- **B1 (#59, `0.0.13`)** — the model: `Table`, `TableProperties`, `TableGrid`, `TableColumn`,
  `TableRow`, `TableCell`, `TableCellProperties`, plus `TablePart` and `CellBorder`, in
  `crates/mjx-dml/src/table/`.
- **B2 (#60, `0.0.14`)** — `add_table`, dimensions, widths and heights, `cell_span`,
  `merged_cell_anchor`, and thirteen `cell_*` text methods over one generalized text-body locator.
- **B3 (#61, `0.0.15`)** — cell fill, the six borders, margins, anchoring, text direction; the
  writer half of `TableCellProperties`; the generated `TextAnchoring` / `TextDirection` /
  `TextHorizontalOverflow`.
- **#62 → #63 (`0.0.16`)** — the ergonomics pass: `Cells`, `CellFormat`, `format_cells` /
  `format_cell_text` / `format_cell_paragraphs`. (#62 merged into a retired base and had to be
  recovered by #63 — see [Guardrails](#guardrails).)
- **B4a (#64, `0.0.17`)** — `merge_cells` / `unmerge_cells`, and `TableCell::set_spans` /
  `set_merged` / `clear_merge`.

Tests: `crates/mjx-dml/tests/table_model.rs` (24), and in `crates/mjx-pptx/tests/`
`tables.rs` (18), `table_formatting.rs` (16), `table_selection.rs` (16), `table_merging.rs` (17),
plus an `office_open.rs` canary that builds a styled, merged 3×3 and renders it through LibreOffice.

## Decisions settled — do not re-litigate

1. **Merging never removes a cell.** A merged region is anchored at its top-left cell, which states
   `gridSpan` / `rowSpan`; every covered cell **stays in the table** stating `hMerge` / `vMerge`. So
   a row always holds as many `a:tc` as the grid has `a:gridCol`, `(row, column)` addressing has no
   holes, and `Table::merge_anchor` can answer *which cell renders here* by walking left then up —
   because a covered cell says "something to my left owns me", never which cell that is.
2. **A covered cell keeps its own text.** Merging hides it; unmerging gives it back. The user chose
   non-destructive over matching PowerPoint's discard-on-merge behaviour.
3. **A default is removed, not written.** `gridSpan="1"`, `hMerge="0"` and the like are what the
   schema already assumes. This is what makes *merge then unmerge* **byte-identical** to no change.
4. **The grid is the authority on column count.** `a:tblGrid` is where a table declares its width —
   never count a row's cells. A table with no grid reports `0` rather than inferring one.
5. **Merge, not rebuild**, for `a:tcPr` and `a:tbl` alike. Both carry content this tier does not
   model (`cell3D`, `headers`, `extLst`, a style reference), so a child is replaced in place or
   inserted at its rank in the schema sequence — `tcpr_child_rank` in
   `crates/mjx-dml/src/table/cell.rs` is that sequence.
6. **Removing a property is not writing an explicit "none".** `clear_cell_fill` lets the table style
   decide again; `FillSpec::None` states the cell is deliberately unfilled and blocks the style. The
   two must stay distinguishable, and `CellFormat`'s `without_*` methods carry the distinction.
7. **Unstated is not zero.** A cell's margins default to `0.1"` horizontally and `0.05"` vertically,
   so `CellMargins` keeps every field `Option` in both directions; a `None` on write leaves that
   inset exactly as it was.
8. **Explicit `cell_*` methods, over one implementation.** The user chose spelled-out methods rather
   than a widened shape parameter. The mitigation is binding: every text operation is a **named
   function** in `presentation.rs` (`paragraph_text_of`, `set_run_properties_in`, …) that both the
   `shape_*` and `cell_*` methods call. **Add a text feature as one operation plus two one-line
   delegators — never a second implementation.**
9. **Reaching a cell walks the raw tree.** Only the addressed `a:txBody` or `a:tcPr` is parsed and
   rebuilt, so editing one cell costs what editing a shape costs. Do not "simplify" this into
   parsing the whole `Table` for a text edit.
10. **A partial merge overlap is refused.** A selection containing a merged region that reaches
    *outside* it gets `TableMergeCrossesSelection`; one wholly inside is absorbed. Truncating would
    leave a span that no longer fits; expanding would merge cells the caller never named.

## Verified schema — read from the XSDs, not assumed

```
CT_Table          tblPr?, tblGrid, tr*
CT_TableGrid      gridCol*                       CT_TableCol   @w (required)
CT_TableRow       tc*, extLst?                   @h (required)
CT_TableCell      txBody?, tcPr?, extLst?        @rowSpan @gridSpan @hMerge @vMerge @id
CT_TableCellProperties
                  lnL? lnR? lnT? lnB? lnTlToBr? lnBlToTr? cell3D? <fill> headers? extLst?
                  @marL=91440 @marR=91440 @marT=45720 @marB=45720
                  @vert=horz @anchor=t @anchorCtr=false @horzOverflow=clip
CT_TableProperties
                  <fill> <effects> (tableStyle | tableStyleId)? extLst?
                  @rtl @firstRow @firstCol @lastRow @lastCol @bandRow @bandCol  (all default false)
```

The `a:graphicData@uri` a table frame carries is
`http://schemas.openxmlformats.org/drawingml/2006/table` — but `slide::shape_table` looks for the
`a:tbl` **element** rather than trusting that string, since it is the payload that decides whether a
frame holds a table, a chart or a diagram.

**How little of this is new:** a cell's content is a `CT_TextBody` — the *same* type a shape's
`p:txBody` is — so the whole text tree applies inside a cell unchanged. Cell borders are six
`CT_LineProperties`, so one `LineSpec` describes all of them (a border is an `a:ln` under another
name). Cell and table fills are the fill model. Widths, heights and margins are `Emu`.

## Fixtures, as this workstream found and left them

- **`tests/fixtures/layouts.pptx` slide 2** holds a `p:graphicFrame` with a **real one-cell table**
  (`a:tblPr firstRow="1" bandRow="1"`, one `a:gridCol w="3048000"`, one row `h="370840"`, cell text
  `Cell`, an empty `a:tcPr`, and **no** `tableStyleId`). It was left there deliberately by the
  transform workstream. It is the *unstyled, someone-else's-file* case.
- Everything else is built at runtime through `add_table`, which is cheaper than authoring fixtures
  and tests the creation path at the same time.
- **B5 will need a new fixture** — a deck carrying a `tableStyles.xml` part and a table naming a
  `tableStyleId`. Author it as `tests/fixtures/tables.pptx` rather than growing `layouts.pptx`,
  which already carries the layout/master, text-inheritance and transform workstreams.

## Roadmap

### B4b — inserting and removing rows and columns (next)

```rust
deck.insert_row(0, table, 2)?;      deck.remove_row(0, table, 2)?;
deck.insert_column(0, table, 1)?;   deck.remove_column(0, table, 1)?;
```

**The invariant to test hardest:** a column edit must keep `a:tblGrid` and **every** `a:tr` in step —
one `a:gridCol` and one `a:tc` per row — or the rows disagree with the table's declared width.

Span adjustment is why this is its own tier (the user chose *adjust*, not *refuse*):

| case | behaviour |
| --- | --- |
| a merge **crosses** the inserted/removed line | the anchor's `gridSpan` / `rowSpan` is incremented or decremented; a cell inserted inside a merged region is born `hMerge` / `vMerge` |
| the removed row/column holds the **anchor** of a multi-cell merge | promote the next cell of the region: it loses `hMerge` / `vMerge`, gains the reduced span, and takes the old anchor's `a:txBody` and `a:tcPr`, so the table looks unchanged |
| a merge lies entirely outside | untouched |

Removing the **last** row or column is refused with the existing `PptxError::InvalidTableSize`, which
`add_table` already enforces at creation. A new row copies the height of the row beside it (a new
column its width), so the table grows predictably rather than gaining a zero-sized band.

### B5 — the `tableStyles.xml` part

`TableStyleList` (`a:tblStyleLst@def`), `TableStyle` (14 part slots: `wholeTbl`, `band1H`/`band2H`,
`band1V`/`band2V`, `firstRow`/`lastRow`, `firstCol`/`lastCol`, the four corner cells, `tblBg`),
`TablePartStyle`, `TableStyleTextStyle`, `TableStyleCellStyle`, `TableCellBorderStyle`, and
`ThemeableLineStyle` (`ln | lnRef`). New simple type: `ST_OnOffStyleType` — a **tri-state**
(`on`/`off`/`def`), the first here that is *not* the `bool` the `ST_OnOff` family is.

Part wiring mirrors images: a `REL_TABLE_STYLES` + content-type constant in `constants.rs`, reached
by `follow_rel` from the presentation part, created on demand.

**The seven `a:tblPr` flags belong here, not in B3.** They emphasize nothing on their own — they tell
a table style which parts to treat specially — so they land beside something that responds to them.
`TablePart` (B1) already names them; only the surface is missing.

### B6 — effective cell formatting

What a cell *renders* as: its own `a:tcPr`, then the table style's applicable part styles selected by
position and the `a:tblPr` flags (row 0 of a table with `firstRow="1"` takes `firstRow`; banding
alternates `band1H`/`band2H`), then the theme for `lnRef` and themeable fills. Colours bake to
concrete `RRGGBB`, as `effective_shape_fill` and `effective_run_properties` do.

Borrow the *shape* of `Presentation::placeholder_candidates` — a candidate list walked in priority
order — but the candidates are style parts, not ancestor shapes.

## Known follow-ups (not blockers)

- **A table's cells cannot be addressed across a merge boundary in bulk.** `Cells::rectangle` selects
  positions; it does not know that a region is one cell. Formatting a selection that partly covers a
  merge writes to covered cells too, which is harmless (they render nothing) but is worth revisiting
  when B6 can say what actually renders.
- **`a:tableStyle` inline** (as opposed to `tableStyleId`) is preserved and not reported at all.
- **`cell3D`, `headers`, `extLst`** round-trip opaquely and have no typed surface.
- **Text in a covered cell is reachable.** `cell_text` on a merged-away cell returns text nothing
  renders. That is deliberate (it is how unmerge restores it) but a caller wanting *what is visible*
  should consult `merged_cell_anchor` first.
- **`graphicFrame` holding a chart or diagram** reads as `ShapeIsNotATable`. Charts are Phase 6.

## Guardrails

Standard project rules (`CLAUDE.md`, `PHASE2_HANDOFF.md` §3). The ones this workstream keeps meeting:

- **Fidelity first.** Every write test pairs its structural assertion with "every other part is
  byte-identical"; every read test asserts reading dirtied nothing. Creating a table adds **no parts
  and no relationships**, so its save test asserts every part *except* the slide is untouched.
- **Names come from the ECMA-376 Part 1 prose, never guessed.** `pdftotext` on the References PDF is
  how B3's three enums were named — each token's title is given in its enumeration table. That is how
  `wordArtVertRtl` turned out to be *"Vertical WordArt Right to Left"*, a different word order from
  its siblings, which was kept.
- **Check the schema before assuming a type.** A graphic frame's transform is `p:xfrm` —
  PresentationML's namespace, a direct child, and *required* — which would have been wrong on all
  three counts if guessed from the other shape kinds.
- **A loop in a feature's own example code is a design defect.** B3's canary needed four; that became
  `Cells` + `CellFormat`. The public API is not stable until `v0.1`, so fix ergonomics now.
- **Branch every PR from `main`.** #62 was stacked on #61's branch, #61 merged and was retired, and
  #62 then merged into the dead branch — stranding 1078 lines while `main` looked healthy. After any
  merge, verify the content reached `main` rather than trusting the PR's "merged" badge.
- Every PR bumps the patch version and adds a `CHANGELOG.md` entry as its last commit; commits split
  by concern (xtask / generated / dml / pptx / tests / fixture / docs).

## Where to look

`crates/mjx-dml/src/table/` — `table.rs` (`Table`, `merge_anchor`), `cell.rs` (`TableCell`,
`TableCellProperties`, `CellBorder`, `tcpr_child_rank`), `grid.rs`, `row.rs`, `properties.rs`.

`crates/mjx-pptx/src/table.rs` — `Cells` (and its `bounds` / `resolve`) and `CellFormat`.

`crates/mjx-pptx/src/slide.rs` — `shape_table` / `shape_table_mut`, `nth_row_mut`, `nth_cell_mut`.

`crates/mjx-pptx/src/presentation.rs` — the whole surface, plus the private spine: `TextSite` and
`with_text_body_at` / `edit_text_body_at`, `with_table` / `edit_table_child`,
`with_cell_properties` / `edit_cell_properties`, `edit_selected_cells` (which passes each cell's
position), and `check_merges_fit`.
