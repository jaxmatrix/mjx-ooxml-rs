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
