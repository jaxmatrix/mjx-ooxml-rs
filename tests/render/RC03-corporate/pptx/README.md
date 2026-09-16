# RC03 — the corporate deck (`tests/fixtures/corporate.pptx`)

One widescreen slide shaped like a page somebody would actually send: a master carrying a logo, an
accent band and a gradient background; a title in two colours with a soft line break **in the
layout's own title placeholder**, and a summary line in its body placeholder, neither stating a font
size so that the master's `p:titleStyle` and `p:bodyStyle` are what lay them out; slide-number and
date fields; a cropped picture and an ellipse-masked one; a custom-geometry icon; an arrow
connector; a bar chart on the theme's accents; a SmartArt frame whose cached drawing holds three
real boxes; a styled 3×3 table; a semi-transparent rounded overlay stating its own corner radius;
and a Wingdings bulleted list whose three bullets are underlined, struck through and highlighted.

**How it is authored.** `crates/mjx-reference-pack/tests/the_corporate_fixtures_are_authored.rs`
builds it with this workspace's own writers and nothing else — no file was copied from
`tests/office-authored/` or `tests/office-exports/`, which agents may not fill. Re-derive it with:

```sh
MJX_AUTHOR_FIXTURES=1 cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_authored
```

That suite also holds the committed bytes to those writers: it re-authors the deck on every run and
fails if the result differs, so the fixture and the code that claims to author it cannot drift
apart.

Six elements are **spliced as markup** rather than written by a typed writer, because this workspace
has a reader for each and no writer: the master's `p:bg`, the title's `a:br`, the two `a:fld`
fields, the picture's `a:srcRect`, the `p:cxnSp` connector and the SmartArt frame's `dsp:drawing`.
The splices refuse a missing anchor, so a writer that starts emitting one of them fails there rather
than producing a fixture quietly missing the element. The crop is anchored on **the cropped
picture's own relationship id**: it went before *the first `a:stretch` on the slide* until RC03's
audit, which would have moved it to the other picture the day a writer changed the order it emits
shapes in, with every gate still green.

**Proved by** `crates/mjx-reference-pack/tests/the_corporate_fixtures_are_corporate.rs` (the elements,
through the typed model where a reader exists) and
`crates/mjx-reference-pack/tests/an_acceptance_render_of_pptx.rs` (the whole pipeline to pixels, with
every loss named).

**Run**:

```sh
cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_corporate
cargo test -p mjx-reference-pack --test an_acceptance_render_of_pptx -- --ignored
```

The plate is written to `target/rc03-acceptance/pptx.png`, which is git-ignored.

## Decision D9 — one folder per test, a LibreOffice reference for this run

Each render test owns its folder, holding a synthetic input and the reference for **this** run, which
is LibreOffice's. LibreOffice is a change detector and **not** parity: whether these pixels look like
PowerPoint's is a question only a person sitting against real Microsoft Office on Windows answers,
and that sitting re-runs these same cases later (`docs/validation/07-the-reference-pack.md`).
`reference/windows.png` stays empty until it happens.

## What the render loses today

The acceptance journey pins the whole loss vector rather than describing it. Today the deck reaches
pixels with: the SmartArt frame unlaid (RC28), one text body measured rather than shaped, the chart
unresolved (RC06), 21 runs taking a default colour (RC16), **the overlay band's 35 % fill painted
opaque** (RC04), both pictures undecoded (RC11), the connector's arrowhead undrawn (RC14) and the
custom geometry's outline unresolved (RC24). Three outline handles go unanswered — the two inherited
placeholders and the `custGeom` icon — and exactly one of them is drawn as a stand-in.

**The overlay is the one worth reading twice.** `mjx-dml` bakes a colour to a six-digit hex triplet
and drops the alpha, so a band the document states at 35 % paints as a solid slab over the table's
third row — the row is in the display list and not in the picture. Until RC03's audit **nothing
counted that**: no loss kind named a discarded opacity and `color_of` answered `Ok` for a valid
triplet, so the render called itself lossless while losing a row. Carrying the channel through is
RC04 (MJXOFF-243); the count is what stops the loss being silent in the meantime.

The journey also pins a **structural floor** the ink-coverage bound cannot give it: 46 boxes, 17
lines, 21 glyph runs, 2 images, 18 shapes and 1 table fragment, and 8 `FillPath`, 1 `StrokePath`, 21
`DrawGlyphs` and 2 `DrawImage` commands — plus a named assertion that each of the **third row's**
three cells has a glyph run starting inside it, which is the row the overlay covers.

## Checklist coverage

| features.json row | fixture element |
| --- | --- |
| pptx-slide-reaches-pixels | the whole slide, carried to a display list and rasterised by the acceptance journey |
| pptx-line-breaks | the title's soft line break (`a:br`) between its two runs |
| pptx-fields | the footer's `slidenum` and `datetimeFigureOut` fields (`a:fld`) |
| pptx-run-colour | the title's first run, coloured `accent1` while the second inherits |
| pptx-slide-background | the master's `p:bg`, a linear `a:gradFill` from `bg1` to a lightened `accent1` |
| pptx-master-and-layout-shapes | the master's logo picture and its accent band, inherited by the slide |
| pptx-placeholder-geometry | the two layout placeholders the slide inherits, which state no geometry of their own |
| pptx-custom-geometry | the chevron icon's `a:custGeom` path |
| pptx-shape-adjustments | the preset shapes' `a:avLst`, read back through `shape_adjustments` when the outline is registered |
| pptx-picture-crop-and-adjustments | the cropped picture's `a:srcRect` |
| pptx-picture-shape-properties | the ellipse-masked picture, whose `prstGeom` is its mask |
| pptx-picture-pixels | both pictures' PNG bytes, read out of the package by the journey |
| pptx-colour-opacity | the overlay rectangle's fill, an `a:alpha` transform on a solid colour |
| pptx-gradient-kinds | the background's linear gradient, with its `a:lin` angle and two stops |
| pptx-charts | the bar chart's `c:chartSpace` part and its embedded workbook |
| pptx-smartart | the SmartArt frame's four `dgm:` parts |
| pptx-table-rendering | the 3×3 table's rows, columns and cell text |
| pptx-table-style-bands | the table's style id and its `firstRow` / `bandRow` flags |
| pptx-theme-schemes | the theme's six accents, which the band, the icon and the chart's series all take |
| pptx-arrowheads | the connector's `tailEnd`, written through the typed outline writer |
| pptx-render-losses | the journey's pinned loss vector, layout then scene then painter |
| shared-corporate-fixtures | this fixture, one of the three RC03 commits |
| shared-render-loss-vocabulary | the same pinned vector, read through `page_losses` |
| shared-custom-geometry | the chevron icon, resolved through `mjx-geometry`'s real provider |
| shared-smartart-cached-drawing | the diagram's `dsp:drawing`, carried through the round trip untouched |
| shared-colour-alpha | the overlay's `a:alpha`, the one colour transform on the slide |
| shared-line-ends-and-dashes | the connector's arrow tail |
| shared-headless-painters | the render, taken through the pure-Rust software painter with no graphics stack |
| shared-box-model-issued-contract | the catalogue's geometry and image handles, which the journey resolves |
| shared-image-decoding | the two pictures, which reach the painter as bytes it cannot yet decode |
| pptx-underline-strike-highlight | the three bullets, one underlined, one struck through and one highlighted |
| shared-text-decorations | those three bullets, the document's decorated runs and the workbook's struck total — one decorated run per fixture |
