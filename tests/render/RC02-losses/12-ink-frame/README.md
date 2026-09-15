# 12 An ink content part

**Input** (`input.pptx`): One `p:contentPart` inside `mc:AlternateContent` (`Requires="p14"`, empty fallback) with a `p14:xfrm` of 3 x 2 in at (1 in, 1 in), pointing at a small InkML part.

**Expected output**: One labelled box at (96, 96)-(384, 288) reading "Ink not rendered".

**Proved by** `rc02_12_an_ink_part_is_one_labelled_placeholder`: layout losses `[0, 0, 0, 1, 0, 0, 0, 0]`; one placeholder at (96, 96, 384, 288), `Layout(FrameContentNotLaidOut(Ink))`, path `[0, 0]`; software `loss_placeholders == 1`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_12_an_ink_part_is_one_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: the reader reports 0 shapes on the slide and the display list is empty.
