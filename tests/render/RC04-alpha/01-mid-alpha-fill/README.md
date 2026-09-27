# 01 Five opacities paint five different greys

**Input** (`input.pptx`): A widescreen slide filled white, with five black bars 1.5 × 2 in on it at
1 in from the top, starting at 0.5 in and every 2 in after that. The bars state `a:alpha` of 0 %,
25 %, 50 %, 75 % and nothing at all (opaque).

**Expected output**: Five bars fading from invisible to solid, left to right — the leftmost one is
not there at all, the rightmost is solid black, and the three between are three distinct greys.

**Proved by** `rc04_01_five_opacities_paint_five_different_greys`: the centre pixel of each bar is
`255`, `191`, `127`, `64`, `0` (the grey `round(255 × (1 − alpha/255))` gives, exactly, because the
painter composites `dst = src + dst × (1 − a)` over an opaque page), the three mid values are
strictly ordered, and the page behind the bars is opaque white.

**Run**: `cargo test -p mjx-reference-pack --test render_rc04_alpha rc04_01_five_opacities_paint_five_different_greys -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from
`soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png`
is empty until the Windows sitting.

## Red before

```
assertion `left == right` failed: black over white at 0 %, 25 %, 50 %, 75 % and 100 % must paint [255, 191, 127, 64, 0]; a build that carried one opacity everywhere paints one value five times
  left: [0, 0, 0, 0, 0]
 right: [255, 191, 127, 64, 0]
```

Today `mjx-dml` discards the resolved alpha, so **all five** bars paint as solid black — including
the one the document says is invisible. The white background is drawn correctly (the pixel at
(19, 19) is `255 255 255 255`), so the sample points are over the bars and the five zeros are the
defect rather than a mis-aimed probe.
