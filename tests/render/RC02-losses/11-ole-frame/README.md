# 11 An embedded OLE object

**Input** (`input.pptx`): One OLE `p:graphicFrame` at (1 in, 1 in), 4 x 3 in, with `p:oleObj` and no fallback picture; the embedding part is a few bytes.

**Expected output**: One labelled box over the frame, (96, 96)-(480, 384), reading "Embedded object not rendered".

**Proved by** `rc02_11_an_embedded_object_frame_is_one_labelled_placeholder`: layout losses `[0, 0, 1, 0, 0, 0, 0, 0]`; one placeholder at (96, 96, 480, 384), `Layout(FrameContentNotLaidOut(EmbeddedObject))`, path `[0, 0]`; software `loss_placeholders == 1`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_11_an_embedded_object_frame_is_one_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: an undecorated box and 0 commands.
