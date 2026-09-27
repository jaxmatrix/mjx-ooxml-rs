# 19 An unresolved worksheet outline

**Input** (`input.xlsx`): The worksheet chart from test 09 (same generator). Its first axis is a `ShapeFragment` whose outline handle `SheetGeometry` does not know.

**Expected output**: The sheet renders; the chart is one "Chart not rendered" box, and the worksheet geometry answers any unknown outline with a stand-in labelled "Shape outline not resolved" instead of failing.

**Proved by** `rc02_19_an_unresolved_worksheet_outline_is_a_placeholder_not_a_failure`: `SheetGeometry::outline(first axis handle, its rect)` is `Ok` with provenance `Placeholder` and label `Shape outline not resolved`; software painter losses `[0; 5]`, `loss_placeholders == 1`; `placeholder_at(200, 120)` is labelled `Chart not rendered`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_19_an_unresolved_worksheet_outline_is_a_placeholder_not_a_failure -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: `SheetGeometry::outline` answers `Err(SceneError::UnresolvedOutline { outline })`, which fails any frame that asks.
