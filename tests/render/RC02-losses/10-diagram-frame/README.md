# 10 A SmartArt frame

**Input** (`input.pptx`): One `p:graphicFrame` at (1 in, 1 in), 6 x 4 in, whose `a:graphicData` is a diagram (`dgm:relIds`) with an empty data model.

**Expected output**: One labelled box over the frame, (96, 96)-(672, 480), reading "Diagram not rendered".

**Proved by** `rc02_10_a_diagram_frame_is_one_labelled_placeholder`: layout losses `[chart, diagram, object, ink, picture, unshaped, not read, approximated] == [0, 1, 0, 0, 0, 0, 0, 0]`; one placeholder at (96, 96, 672, 480), `Layout(FrameContentNotLaidOut(Diagram))`, path `[0, 0]`; `placeholder_at(384, 288)` answers that category; software `loss_placeholders == 1`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_10_a_diagram_frame_is_one_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: the frame lays out as an undecorated box and the display list is empty (0 commands).
