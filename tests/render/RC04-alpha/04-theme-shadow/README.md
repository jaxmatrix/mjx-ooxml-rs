# 04 A 63 % shadow is not a solid one

**Input** (`input.pptx`): A white widescreen page carrying two white 3 × 2 in rectangles, each with an
`a:outerShdw` offset half an inch down and to the right and **no blur** — the left one's shadow is
black at 63 %, the right one's is black with no `a:alpha` at all.

The 63 % is not arbitrary: the standard Office theme's third effect style is
`<a:outerShdw …><a:schemeClr val="phClr"><a:alpha val="63000"/></a:schemeClr></a:outerShdw>`, so
every shape a person styles from PowerPoint's gallery has exactly this shadow. Rendering it without
the channel is a solid slab under every styled shape on every deck anybody sends. The blur is set to
zero so the shadow's interior is a flat area a single pixel can be sampled from.

**Expected output**: Two rectangles, the left one with a soft mid-grey shadow and the right one with
a solid black one — visibly different.

**Proved by** `rc04_04_a_sixty_three_per_cent_shadow_is_not_a_solid_one`: the pixel just past the
left shape's bottom-right corner is `5E 5E 5E FF` — `round(255 × (1 − 161/255))`, black at 63 % over
white — the same pixel on the right shape is `00 00 00 FF`, and the two differ by at least 80 levels.

**Run**: `cargo test -p mjx-reference-pack --test render_rc04_alpha rc04_04_a_sixty_three_per_cent_shadow_is_not_a_solid_one -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from
`soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png`
is empty until the Windows sitting.

## Red before

```
assertion `left == right` failed: a 63 % black shadow over white is `round(255 * (1 - 161/255))` = 0x5E. Solid black here is the defect RC04 exists to fix.
  left: [0, 0, 0, 255]
 right: [94, 94, 94, 255]
```

The 100 % shadow already renders correctly, which is what makes the comparison a comparison: the two
shadows are the same black at two opacities, and today they are the same pixel.
