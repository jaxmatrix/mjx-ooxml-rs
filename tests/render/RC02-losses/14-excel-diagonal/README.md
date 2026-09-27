# 14 A diagonal border

**Input** (`input.xlsx`): An empty B2 whose only border is a thin black top-left to bottom-right diagonal (`diagonalDown="1"`).

**Expected output**: A labelled box over B2, (69, 20)-(138, 40), reading "Content not read". Nothing else on the page.

**Proved by** `rc02_14_a_dropped_diagonal_border_is_a_labelled_placeholder`: layout losses `[0, 0, 0, 0, 0, 0, 1, 0]`; one placeholder at (69, 20, 138, 40), `Layout(DroppedByReader)`, path `[1, 1]`; software `loss_placeholders == 1`; ink inside B2 > 0 and outside == 0.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_14_a_dropped_diagonal_border_is_a_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: 36 commands, all clips; 0 draw calls. The diagonal never reaches a band.
