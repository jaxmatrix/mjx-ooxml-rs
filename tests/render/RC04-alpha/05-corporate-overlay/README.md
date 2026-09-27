# 05 The corporate overlay lets the row beneath it through

**Input** (`input.pptx`): `tests/fixtures/corporate.pptx`, RC03's corporate deck, unchanged. Its
overlay band is a rounded rectangle from 0.6 in to 12.7 in across and 3.25 in to 3.85 in down, filled
`1F3864` at 35 %; the 3 × 3 table stands at 9.4 in to 12.8 in, and its **third row** is the row the
band crosses.

**Expected output**: The band is a translucent navy stripe. The table's third row reads through it —
the cells and their text are visible, tinted, rather than covered.

**Proved by** `rc04_05_the_corporate_overlay_lets_the_row_beneath_it_through`: a pixel inside a
third-row cell and under the band is **not** `1F3864`, and it equals the 35 % blend of `1F3864` over
the colour the same cell paints just below the band (sampled at the same x, 3.87 in down) to within
one rounding step per channel. The two samples must also differ, so a band that drew nothing cannot
pass by comparing a colour with itself.

RC03 left two facts here for RC04 to change, and this is the second of them; the first is the deck's
pinned loss vector in `crates/mjx-reference-pack/tests/an_acceptance_render_of_pptx.rs`, which no
longer carries `(Scene(PaintApproximated), 1)`.

**Run**: `cargo test -p mjx-reference-pack --test render_rc04_alpha rc04_05_the_corporate_overlay_lets_the_row_beneath_it_through -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from
`soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png`
is empty until the Windows sitting.

## Red before

```
assertion `left != right` failed: the pixel inside the third row is the band's own colour, so the band painted opaque over the row — which is exactly the loss RC03 pinned and RC04 removes
  left: [31, 56, 100]
 right: [31, 56, 100]
```

`31 56 100` is `1F3864` exactly: the band is painted at 100 % and the row underneath it is in the
display list and not in the picture.
