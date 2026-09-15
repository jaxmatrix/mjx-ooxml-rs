# 03 An unresolved colour

**Input** (`input.pptx`): Two 3 x 2 in rectangles: one filled with a bare `a:schemeClr val="phClr"`, one with `phClr` plus `a:lumMod`. Outside a style there is no placeholder colour, so `mjx-dml` leaves them as `Scheme` and `Transformed { Scheme }`.

**Expected output**: Two labelled boxes, at (96, 96)-(384, 288) and (480, 96)-(768, 288), each reading "Colour not resolved".

**Proved by** `rc02_03_an_unresolved_colour_is_a_labelled_placeholder`: scene losses `[0, 2, 0, 0]`; placeholders exactly those two rectangles, label `Colour not resolved`, category `Scene(ColourNotResolved)`, paths `[0, 0]` and `[0, 1]`; software `loss_placeholders == 2`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_03_an_unresolved_colour_is_a_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: neither shape emits a `FillPath` (`color_of` answers `None`), and nothing is counted.
