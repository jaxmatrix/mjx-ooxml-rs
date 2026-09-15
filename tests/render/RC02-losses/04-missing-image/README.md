# 04 A picture with no pixels

**Input** (`input.pptx`): One 3 x 2 in picture at (1 in, 1 in) whose relationship points at `/ppt/media/image1.png`, a part that was deleted from the package. The image source is built from the package, so it holds nothing for the handle.

**Expected output**: A labelled box at (96, 96)-(384, 288) reading "Picture not available".

**Proved by** `rc02_04_a_picture_with_no_pixels_is_a_labelled_placeholder`: painter losses `[image pixels, glyph run, effect, line end, outline] == [1, 0, 0, 0, 0]`; `drawn.images == 0` and `loss_placeholders == 1`; `ImageWithNoPixels.label() == "Picture not available"`; ink inside the box > 0, outside == 0.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_04_a_picture_with_no_pixels_is_a_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: `DrawImage` is emitted, the software painter skips it silently, and `DrawReport::images` still reports 1 with nothing inked.
