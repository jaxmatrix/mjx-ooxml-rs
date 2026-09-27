# 15 Flattened rich text

**Input** (`input.xlsx`): A1 holds an inline string of two runs: bold red "Bold" and plain " plain".

**Expected output**: "Bold plain" is drawn once in the cell's own format, and no placeholder is drawn. The approximation is counted once against A1.

**Proved by** `rc02_15_flattened_rich_text_is_counted_as_approximated`: layout losses `[0, 0, 0, 0, 0, 0, 0, 1]`; the one loss has path `[0, 0]` and label `Approximated`; no placeholders; software draws 9 glyphs and 0 loss placeholders.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_15_flattened_rich_text_is_counted_as_approximated -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: one glyph run of "Bold plain" (9 glyphs) in the cell format, nothing counted.
