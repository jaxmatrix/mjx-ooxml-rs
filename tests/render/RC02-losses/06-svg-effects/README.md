# 06 Effects the SVG cannot express

**Input** (`input.pptx`): A blue rectangle with an inner shadow and an orange rectangle with a reflection.

**Expected output**: The SVG draws both rectangles, each covered by a labelled box reading "Effect not drawn". The software render draws both effects and no placeholder.

**Proved by** `rc02_06_an_effect_the_svg_cannot_express_is_counted_and_labelled`: SVG painter losses `[0, 0, 2, 0, 0]` with 2 loss placeholders; the SVG contains exactly 2 `data-mjx-loss="Effect not drawn"`; software losses `[0; 5]` and `layers == 2`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_06_an_effect_the_svg_cannot_express_is_counted_and_labelled -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: the SVG writes `data-mjx-effect-unsupported` on each group and counts nothing.
