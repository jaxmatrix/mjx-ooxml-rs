# 13 Icon-set icons

**Input** (`input.xlsx`): A1:A3 hold 10, 20 and 30 under a `3Arrows` icon set (percent thresholds 0 / 33 / 67).

**Expected output**: Each of A1, A2 and A3 carries a labelled box over the cell reading "Picture not rendered"; the numbers are still drawn.

**Proved by** `rc02_13_every_chosen_icon_is_a_labelled_placeholder`: layout losses `[0, 0, 0, 0, 3, 0, 0, 0]`; placeholders exactly A1 (0, 0, 61, 20), A2 (0, 20, 61, 40), A3 (0, 40, 61, 60), `Layout(FrameContentNotLaidOut(Picture))`, paths `[0, 0]`, `[1, 0]`, `[2, 0]`; software `loss_placeholders == 3`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_13_every_chosen_icon_is_a_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: `CellReport::icon` chooses indices 0, 1 and 2 of `ThreeArrows`; only the three numbers are drawn (3 draw calls).
