# 07 Line ends that are not drawn

**Input** (`input.pptx`): One horizontal 6 in connector at y = 2 in, 3 pt black, with `a:headEnd type="triangle"` and `a:tailEnd type="arrow"`.

**Expected output**: The line is drawn without arrowheads. The connector is zero pixels tall, so there is no room for a placeholder; the two missing ends are counted.

**Proved by** `rc02_07_a_line_end_that_is_not_drawn_is_counted`: software painter losses `[0, 0, 0, 2, 0]` and `loss_placeholders == 0`; `LineEndNotDrawn.label() == "Arrowhead not drawn"`; scene losses `[0; 4]`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_07_a_line_end_that_is_not_drawn_is_counted -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: the `StrokePath` carries `head: Triangle` and `tail: Arrow`, the tessellator draws neither, nothing is counted.
