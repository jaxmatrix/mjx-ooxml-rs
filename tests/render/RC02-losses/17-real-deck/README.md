# 17 The real deck journey

**Input** (`input.pptx`): `text_levels.pptx` from `tests/fixtures/` (the input `a_real_deck_reaches_pixels.rs` walks): a title, a five-level body, a footer, a text box and a rectangle with text.

**Expected output**: The same page as today. No placeholder: nothing on it is dropped.

**Proved by** `rc02_17_the_real_deck_journey_has_an_exact_loss_vector`: layout losses `[0; 8]`; 14 `DrawGlyphs`; scene losses `[0, 0, 0, 14]`; no placeholders; painter losses `[0; 5]`, `loss_placeholders == 0`.

## How the numbers were counted

- Glyph runs: the title (1), the body's five paragraphs (5 text runs) plus four bullet markers on levels zero to three (4), the footer (1), the text box's text and its marker (2), the rectangle (1): 14.
- Every one of them is a `TextPaintDefaulted` loss, because the slide companion carries no run paint (MJXOFF-311 owns wiring it).
- No chart, diagram, object, ink, picture, unresolved colour or picture fill is on the slide, so every other entry is 0.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_17_the_real_deck_journey_has_an_exact_loss_vector -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: 14 `DrawGlyphs` in default black and no loss vector.
