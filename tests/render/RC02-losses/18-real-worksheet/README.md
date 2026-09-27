# 18 The real worksheet journey

**Input** (`input.xlsx`): `style_resources.xlsx` from `tests/fixtures/` (an input `a_real_worksheet_reaches_pixels.rs` walks): A1 holds 42 in a custom format, a path gradient fill and border 1.

**Expected output**: A1 draws as today (fill, number, four edges, double bottom) plus one labelled box over A1, (0, 0)-(61, 20), reading "Content not read".

**Proved by** `rc02_18_the_real_worksheet_journey_has_an_exact_loss_vector`: layout losses `[0, 0, 0, 0, 0, 0, 1, 1]`; one placeholder at (0, 0, 61, 20), `Layout(DroppedByReader)`, path `[0, 0]`; scene losses `[0; 4]`; painter losses `[0; 5]`; `loss_placeholders == 1`.

## How the numbers were counted

- `DroppedByReader` 1: border 1 has `diagonalUp="true"` with a `slantDashDot` diagonal, and no band draws a diagonal.
- `ValueApproximated` 1: the right edge is `mediumDashDotDot`, drawn as a solid band (`the_dash_is_lost_at_the_band.rs`). Left `medium`, top `hair` and bottom `double` are solid styles, so they are not approximations.
- Placeholders 1: the dropped diagonal, in A1's own box. Approximations draw none.
- Out of RC02's twelve sites and not counted: the font's strike, double-accounting underline, shadow and superscript. A later ticket that counts them changes this vector.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_18_the_real_worksheet_journey_has_an_exact_loss_vector -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0609]: no field `loss_placeholders` on type `DrawReport`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: 5 bands and 7 draw calls; the diagonal is absent and the dash-dot-dot edge is solid, with nothing counted.
