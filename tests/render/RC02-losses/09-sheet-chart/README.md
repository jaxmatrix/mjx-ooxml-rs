# 09 A chart on a worksheet

**Input** (`input.xlsx`): A blank workbook with the same bar chart anchored from B2 to G13 (`Workbook::add_chart`).

**Expected output**: One labelled box over the drawing frame, (61, 20)-(366, 240), reading "Chart not rendered", over the empty grid.

**Proved by** `rc02_09_a_sheet_chart_is_one_labelled_placeholder`: layout losses `[0; 8]`; scene losses `[1, 0, 0, 0]`; one placeholder at (61, 20, 366, 240), `Scene(ChartNotResolved)`, path `[u32::MAX, 0]` (the drawing address); software `loss_placeholders == 1`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_09_a_sheet_chart_is_one_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: 22 commands, all clips; 0 draw calls. The bars carry chart handles `SheetResources` answers `None` for.
