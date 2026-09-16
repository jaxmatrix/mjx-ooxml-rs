# RC03 — the corporate document (`tests/fixtures/corporate.docx`)

A first page shaped like one somebody would actually send: styles carrying `w:themeColor` and
`w:themeShade`; a header with a three-cell table and a logo; a table with a style id and a banded
header row; an inline picture and a floating one; a text box; a footnote; tracked changes; Symbol
bullets; an equation; and a page border.

**How it is authored.** `crates/mjx-reference-pack/tests/the_corporate_fixtures_are_authored.rs`
builds it with this workspace's own writers and nothing else — no file was copied from
`tests/office-authored/` or `tests/office-exports/`, which agents may not fill. Re-derive it with:

```sh
MJX_AUTHOR_FIXTURES=1 cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_authored
```

That suite re-authors the document on every run and fails if the committed bytes differ.

Five elements are **spliced as markup** rather than written by a typed writer, because `mjx-docx`
keeps the mutators they would need crate-private: the tracked changes (`w:ins` / `w:del`), the
floating picture (`wp:anchor`), the text box (`w:txbxContent`), the header's own content, and the
theme part `Document::blank` does not write. The styles, numbering, table, inline picture, footnote,
equation and page border are all written through the typed API.

**Proved by** `crates/mjx-reference-pack/tests/the_corporate_fixtures_are_corporate.rs` and
`crates/mjx-reference-pack/tests/an_acceptance_render_of_docx.rs`.

**Run**:

```sh
cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_corporate
cargo test -p mjx-reference-pack --test an_acceptance_render_of_docx -- --ignored
```

## ⚠ Word cannot reach pixels

PowerPoint's scene companion is `mjx-scene-pptx` and Excel's is `mjx-scene-xlsx`. **`mjx-scene-docx`
does not exist**, so a Word `FragmentTree` has no route into a display list and none into a painter.
The journey asserts that state concretely — it fails the day RC09 (MJXOFF-255) lands the crate — and
its plate is therefore the **fragment tree**, written to `target/rc03-acceptance/docx.txt` as text.
That is what a reviewer reads to see where Word put things, and what the pixel plate will be diffed
against once the companion exists.

## Decision D9 — one folder per test, a LibreOffice reference for this run

Each render test owns its folder, holding a synthetic input and the reference for **this** run, which
is LibreOffice's. LibreOffice is a change detector and **not** parity: whether the output looks like
Word's is a question only a person sitting against real Microsoft Office on Windows answers, and that
sitting re-runs these same cases later (`docs/validation/07-the-reference-pack.md`).
`reference/windows.png` stays empty until it happens.

## What the layout loses today

The journey pins the whole loss vector of the last stage Word reaches. Today page 0 lays out with
three pictures framed but not laid out — the inline logo, the floating one and the header's — and
nothing else lost. The page holds 41 boxes, 25 lines and 23 glyph runs, and no image, shape or table
fragment: the table reaches the page as boxes and lines rather than as a `Fragment::Table`.

## Checklist coverage

| features.json row | fixture element |
| --- | --- |
| docx-page-catalogue | page 0's fragments, produced by `DocumentBoxModel` against the section's geometry |
| docx-first-page-reaches-pixels | the same page, which is the input RC09's scene companion will carry to a painter |
| docx-run-paint | the body's runs, whose colour resolves from the styles above them |
| docx-theme-tint-shade | the two styles carrying `w:themeColor="accent1"` and `w:themeShade` |
| docx-table-visual-properties | the table's `w:tblStyle` and its `w:tblLook` banding flags |
| docx-table-painting | the same table, which reaches page 0 as boxes and lines |
| docx-header-footer-blocks | the default header's three-cell table and its logo |
| docx-header-floats | the header's inline drawing, anchored inside a header cell |
| docx-furniture-paragraph-decoration | the header's own paragraphs, which the furniture layer draws |
| docx-pictures | the inline picture and the floating one, sharing one image part |
| docx-float-z-order | the floating picture's `wp:anchor`, with its `relativeHeight` and `behindDoc` |
| docx-text-boxes-and-groups | the VML text box and its `w:txbxContent` |
| docx-vml-and-watermarks | the same text box, which is legacy VML (`v:shapetype` / `v:shape` / `v:textbox`) |
| docx-review-markup | the tracked insertion and deletion, read back through `Document::revisions` |
| docx-note-marks-and-separators | the footnote reference in the body and the notes part behind it |
| docx-lost-run-content | the same footnote reference, which is run content a reader must not drop |
| docx-numbering-formats | the numbering definition's level 0, a `bullet` format |
| docx-symbol-and-picture-bullets | that level's `w:rFonts w:ascii="Symbol"` bullet font |
| docx-page-borders | the section's `w:pgBorders` |
| docx-equations | the paragraph's `m:oMath` |
| docx-equation-model | the same equation, read back through the read-once residency |
| shared-box-model-issued-contract | page 0's fragment tree, the contract Word answers in |
