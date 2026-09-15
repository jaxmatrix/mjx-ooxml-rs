# 16 The four painters agree

**Input** (`input.pptx`): The arrow connector from test 07 at y = 2 in, and below it a 3 x 2 in picture at (1 in, 3 in) whose media part is absent.

**Expected output**: Every painter draws the line without ends and one "Picture not available" box at (96, 288)-(384, 480).

**Proved by** `rc02_16_the_four_painters_agree_on_the_loss_counts`: tiny-skia, PDF and SVG each report painter losses `[1, 0, 0, 2, 0]` and `loss_placeholders == 1`; wgpu reports the same when an adapter exists and is skipped otherwise.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_16_the_four_painters_agree_on_the_loss_counts -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: no painter counts anything; the software report says `images: 1`.
