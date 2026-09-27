# 01 A shape whose fill cannot be answered

**Input** (`input.pptx`): One 3 x 2 in rectangle at (1 in, 1 in) filled with a picture fill (`a:blipFill` on the `p:sp`) and no outline. The picture part exists; the slide companion has no image table entry for a shape fill.

**Expected output**: A grey labelled box fills the rectangle at pixels (96, 96)-(384, 288), reading "Fill picture not available". Nothing is drawn outside it.

**Proved by** `rc02_01_a_shape_whose_fill_cannot_be_answered_is_a_labelled_placeholder`: scene losses `[chart, colour, fill picture, text paint] == [0, 0, 1, 0]`; exactly one placeholder at (96, 96, 384, 288) labelled `Fill picture not available`, category `Scene(FillImageNotSupplied)`, source path `[0, 0]`; software `loss_placeholders == 1`; ink inside the box > 0 and outside == 0.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_01_a_shape_whose_fill_cannot_be_answered_is_a_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: the display list is empty (0 commands): the decoration is invisible and `scene.rs` skips the shape.
