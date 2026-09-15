# 02 A run whose paint is defaulted

**Input** (`input.pptx`): One unfilled text box with the run "Red text" whose `a:rPr` states a red solid fill.

**Expected output**: The text is still drawn, in the default colour (black), and no placeholder is drawn. The approximation is counted once against the run.

**Proved by** `rc02_02_a_run_whose_paint_is_defaulted_is_counted`: scene losses `[0, 0, 0, 1]`; the one loss's source equals the glyph-run fragment's `SourceRef` and its label is `Text colour approximated`; no placeholders; software draws 7 glyphs and 0 loss placeholders.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_02_a_run_whose_paint_is_defaulted_is_counted -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0599]: no method named `losses` found for struct `DisplayList`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: one `DrawGlyphs` in opaque black, nothing counted (`SlideResources::text_decoration` answers `None`).
