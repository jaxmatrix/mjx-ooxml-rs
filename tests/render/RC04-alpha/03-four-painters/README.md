# 03 Four painters carry the same opacities

**Input** (`input.pptx`): The same five bars as case 01 — black at 0 %, 25 %, 50 %, 75 % and opaque,
over a white page.

**Expected output**: The same five greys from every painter, in the vocabulary each of them has for
opacity. The two rasterisers answer in pixels; the two exporters answer in the attribute their format
uses, because a screenshot of a PDF would be a test of whatever rasterised it rather than of the
export.

**Proved by** `rc04_03_four_painters_carry_the_same_opacities`:

- **tiny-skia** — the five bar centres are `255`, `191`, `127`, `64`, `0`.
- **SVG** — the document states `fill-opacity="0.251"`, `"0.502"` and `"0.749"` (the exporter writes
  three decimals with trailing zeros trimmed, so `0x40` is `0.251`).
- **PDF** — the content stream states `/ca 0.251`, `/ca 0.502` and `/ca 0.749`, one `ExtGState` per
  translucent fill.
- **wgpu** — the same five greys when an adapter exists; a named skip when there is none, and a
  failure instead under `MJX_REQUIRE_GPU=1`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc04_alpha rc04_03_four_painters_carry_the_same_opacities -- --ignored`

Our renders go to `output/` (git-ignored), including `export.svg` and `export.pdf`.
`reference/libreoffice.pdf` and `.png` come from `soffice --headless --convert-to pdf` and
`pdftoppm -png -r 96 -singlefile`; `reference/windows.png` is empty until the Windows sitting.

## Red before

```
assertion `left == right` failed: tiny-skia
  left: [0, 0, 0, 0, 0]
 right: [255, 191, 127, 64, 0]
```

The run stops at the first painter, so the SVG and PDF assertions have not been reached yet; both
exporters already write `fill-opacity` / `/ca` from `mjx_scene::Color`'s alpha, which is `0xFF` for
every colour today.
