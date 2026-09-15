# 05 A glyph run the PDF cannot embed

**Input** (`input.pptx`): One text box with the black run "Hello". The PDF exporter is handed a font source with no faces, which is the seam under test.

**Expected output**: In the PDF, a labelled box where the run would be, reading "Text not embedded". The software render still draws "Hello".

**Proved by** `rc02_05_a_glyph_run_the_pdf_cannot_embed_is_a_labelled_placeholder`: PDF painter losses `[0, 1, 0, 0, 0]` with 1 loss placeholder; `pdftotext` of our PDF reads exactly `Text not embedded`; software painter losses `[0; 5]` with 0 placeholders; scene losses `[0, 0, 0, 1]`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_05_a_glyph_run_the_pdf_cannot_embed_is_a_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: `export/pdf.rs` returns early for a face the source lacks: no text operators, no count, and `pdftotext` reads nothing.
