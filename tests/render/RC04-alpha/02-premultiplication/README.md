# 02 Straight in the display list, premultiplied in the readback

**Input** (`input.pptx`): A widescreen slide with **no background**, carrying one 4 × 3 in rectangle
at (1 in, 1 in) filled `1F3864` at 50 %.

**Expected output**: A half-transparent navy rectangle on an empty page — and, more to the point,
two different numbers for the same colour at two different stages, each in the convention that stage
uses.

**Proved by** `rc04_02_the_display_list_is_straight_and_the_readback_is_premultiplied`: the display
list's one solid paint is `Color { red: 0x1F, green: 0x38, blue: 0x64, alpha: 0x80 }` — the channels
**straight**, as the document states them — while the software painter's readback of the same pixel
is `0F 1C 32 80`, each channel scaled by the alpha, because `mjx_paint::Pixels::rgba` is
premultiplied and says so in its own documentation. The page outside the rectangle stays
`00 00 00 00`.

This is the case that catches an inconsistency instead of letting it darken the page quietly: if the
display list ever carried premultiplied channels, every translucent thing would be multiplied by its
alpha twice and nothing else in the suite would notice.

**Run**: `cargo test -p mjx-reference-pack --test render_rc04_alpha rc04_02_the_display_list_is_straight_and_the_readback_is_premultiplied -- --ignored`

Our renders go to `output/` (git-ignored). `reference/libreoffice.pdf` and `.png` come from
`soffice --headless --convert-to pdf` and `pdftoppm -png -r 96 -singlefile`; `reference/windows.png`
is empty until the Windows sitting.

## Red before

```
assertion `left == right` failed: the display list must carry the colour **straight** — the channels as the document states them, with the opacity beside them.
  left: [Color { red: 31, green: 56, blue: 100, alpha: 255 }]
 right: [Color { red: 31, green: 56, blue: 100, alpha: 128 }]
```

The colour survives; the opacity does not, so the list carries `0xFF` where the document says 50 %.
The pixel assertion is not reached until that is fixed.
