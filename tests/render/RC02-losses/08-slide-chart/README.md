# 08 A chart on a slide

**Input** (`input.pptx`): A clustered bar chart (three categories, one series) authored with `Presentation::add_chart` at (1 in, 1 in), 6 x 4 in.

**Expected output**: One labelled box over the whole chart frame, (96, 96)-(672, 480), reading "Chart not rendered". Clicking inside it hits the chart and its loss category.

**Proved by** `rc02_08_a_slide_chart_is_one_labelled_placeholder`: layout losses `[0; 8]`; scene losses `[1, 0, 0, 0]` (one per chart, however many handles the chart engine issued); one placeholder at (96, 96, 672, 480), `Scene(ChartNotResolved)`, path `[0, 0]`; `placeholder_at(384, 288)` answers that category and path; `placeholder_at(10, 10)` answers nothing; software `loss_placeholders == 1`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc02_losses rc02_08_a_slide_chart_is_one_labelled_placeholder -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

- Compile: `error[E0599]: no method named `placeholder_at` found for struct `DisplayList`` (the suite does not compile until the RC02 API exists; the first error is `error[E0432]: unresolved imports `mjx_layout::FrameContent`, `mjx_layout::LayoutLossKind`, `mjx_layout::LayoutLosses` (and `mjx_scene::build_page`, `mjx_paint::PainterLossKind`, …)`).
- Today's behaviour, measured with the current pipeline: 0 commands: every chart handle resolves to `None` in `SlideResources` and is skipped.
