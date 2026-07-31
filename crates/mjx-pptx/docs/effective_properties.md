# Effective properties — what a file states versus what a renderer shows

A `.pptx` states remarkably little about how it looks. A title placeholder whose slide declares no
size, no font, no colour and no position still renders at 44pt, centred, in the theme's major
typeface, in a rectangle two thirds of the way across the slide. Every one of those values lives in
another part.

Two families of readers answer the two different questions this raises:

- The **declared** readers — [`shape_fill`], [`shape_bounds`], [`run_properties`], and their kin —
  answer *what this part says*. A property the part does not state comes back `None`. They are the
  right readers for editing: they show what an edit would overwrite.
- The **effective** readers — every `effective_*` method — answer *what a renderer shows*. They walk
  the layout, the master, the theme and `presentation.xml`, resolve the colours, and hand back a
  self-contained answer. They are the right readers for measuring, laying out, or rendering.

No `effective_*` read dirties a part. Resolution parses parts the package had not needed yet — which
is why these methods take `&mut self` — but it never marks one modified, so a deck opened, fully
measured and saved is byte-identical to the deck that went in.

## The APIs

| Reader | Answers |
|---|---|
| [`Presentation::effective_shape_fill`] | the fill the shape paints with |
| [`Presentation::effective_shape_outline`] | the line it is stroked with |
| [`Presentation::effective_shape_effects`] | its shadow / glow / reflection / soft edge |
| [`Presentation::effective_shape_transform`] | the offset, extent, rotation and flips it renders under |
| [`Presentation::effective_shape_bounds`] | the same, as a plain rectangle |
| [`Presentation::effective_run_properties`] | the character formatting one run renders with |
| [`Presentation::effective_paragraph_properties`] | the bullet, indent and alignment one paragraph renders with |
| [`Presentation::effective_cell_fill`] | a table cell's fill |
| [`Presentation::effective_cell_border`] | one edge of a table cell |
| [`Presentation::effective_cell_run_properties`] | the character formatting of a run inside a cell |

## The candidate walk

Every shape-level resolver walks the same short list of candidate shapes, in inheritance order:

1. the addressed shape itself;
2. then — **only if it carries a `p:ph`** — the same-slot placeholder on each part the surface
   inherits from.

The inheritance chain is fixed by the surface: a slide resolves through its layout and then that
layout's master; a layout through its master; a master stands alone; a notes slide goes straight to
the notes master. A tier that does not define the slot simply says nothing and the walk continues.

The consequence is worth stating plainly, because it surprises people: **a shape that is not a
placeholder inherits nothing.** A plain text box takes no fill, no outline, no effect and no position
from the layout it sits on, however much the layout declares. It has no slot to be matched on.

Text is the one exception, and only at the master: a non-placeholder shape still takes a master
*text* style, because that tier is chosen by the shape's kind rather than by a slot. See the seven
tiers below.

There is exactly one implementation of this walk (`placeholder_candidates`), and every resolver
below is built on it. A second copy would be a bug, not a convenience.

## Fill, outline and effects

These three share one ladder. Three sources are tried in order, per candidate:

1. an explicit `p:spPr` fill / `a:ln` / `a:effectLst`;
2. a `p:style > a:fillRef` / `a:lnRef` / `a:effectRef` — the theme's style at that index, with
   `phClr` substituted by the colour the reference carries;
3. failing both, the next candidate in the walk.

The first source that yields anything wins outright. A style reference pointing at an index the
theme does not define yields nothing, so the walk steps past it to the next candidate rather than
returning an empty answer.

## Transform and bounds

`effective_shape_transform` walks the same candidates but combines them differently, and the
difference matters.

**A transform is inherited whole.** The first tier that states anything wins entirely — a shape
cannot take its position from the layout and its size from the master, because PowerPoint offers no
such thing. A present-but-empty `<a:xfrm/>` states nothing, so the walk steps past it exactly as it
steps past a tier with no transform at all.

Once a tier has placed the shape, the enclosing groups are composed on top: a shape addressed as
`[2, 1]` is placed in its group's child space (`a:chOff` / `a:chExt`), and composing that group's own
transform is what turns the result into a rectangle on the slide. For a top-level shape the
composition is the identity.

`effective_shape_bounds` is that answer reduced to a plain rectangle, absolute within
[`Presentation::slide_size`].

## Text — the seven tiers

Unlike a transform, character and paragraph formatting **merges**: each tier contributes only what
the tiers above it left unset. Seven tiers, highest priority first:

1. the run's own `a:rPr`;
2. the paragraph's `a:pPr > a:defRPr`;
3. the shape's own `a:lstStyle`;
4. the same-slot placeholder's `a:lstStyle` on the layout, then the master;
5. the master's `p:txStyles` — `p:titleStyle` for a title placeholder, `p:otherStyle` for the
   date / footer / slide-number slots, `p:bodyStyle` for the rest;
6. `p:defaultTextStyle` in `presentation.xml`;
7. the theme's font scheme, for a typeface still naming `+mj-lt` / `+mn-lt`.

Tier 5 reaches a shape that is **not** a placeholder too, even though tier 4 cannot. ECMA-376
§19.3.1.35 splits them by kind rather than by slot: a text box (`p:cNvSpPr@txBox`) takes
`p:bodyStyle`, and any other non-placeholder shape takes `p:otherStyle`. So a free-standing text box
still renders at the master's body size, while the layout's placeholder styling — which needs a slot
to match on — never reaches it.

Cutting across all of them is the paragraph's **level** (`a:pPr@lvl`, top level when unstated). It is
read once and selects which `a:lvlNpPr` every tier from 3 down contributes. That is why demoting a
line changes its size, indent and bullet without a single character being written to the run.

Each of tiers 3 to 6 is a list style, and each contributes **twice**: what it says at the paragraph's
level, and beneath that its own `a:defPPr` — "the paragraph properties that are to be applied when no
other paragraph properties have been specified" (§21.1.2.2.2). A level a style does not define falls
to that default; there is no fallback to `a:lvl1pPr`, because §21.1.2.4.13 keys the nine level
elements strictly to `a:pPr@lvl`. A deck whose master styles four levels and states a `defPPr` will
therefore answer at level 8 — with the default, not with level 0's bullet.

[`Presentation::effective_paragraph_properties`] answers the same ladder minus the two run tiers; the
`a:defRPr` it carries is the merged character default of every tier that contributed.

Tier 7 is deliberately conservative: a font slot the theme does not define keeps its `+mj-lt`
reference rather than being replaced by a guess. The file points somewhere the theme does not go,
and that is the honest answer.

## Table cells

A cell resolves against its **table style**, not against a placeholder chain, so it gets its own
short ladder:

1. the cell's own `a:tcPr`;
2. the table style's parts, selected by the cell's position and the `a:tblPr` flags, most specific
   first: corner cells, then first/last column, then first/last row, then row bands, then column
   bands, then `wholeTbl`;
3. the theme, for a part that names an `a:lnRef` or `a:fillRef`.

[`Presentation::effective_cell_border`] adds one wrinkle: a cell on the table's rim takes the style's
outer edge (`top`, `left`, …) while a cell inside it takes the interior edge (`insideH`, `insideV`).

[`Presentation::effective_cell_run_properties`] is shorter still — the run's `a:rPr`, the paragraph's
`a:defRPr`, each applicable part's `a:tcTxStyle`, then `p:defaultTextStyle`, then the theme fonts.
**No placeholder chain and no master `p:txStyles`:** a table's text does not inherit from the slide's
body style.

## Why colours come back as `RRGGBB`

A DrawingML colour is rarely a colour. `<a:schemeClr val="tx2"/>` names a slot, which the master's
`p:clrMap` maps to a different slot, which the theme's colour scheme finally resolves to a value —
and then `lumMod`, `shade`, `tint` and friends transform it. Those three inputs live in three
different parts, each with its own string interner.

An effective answer would be useless if it handed back that puzzle. So every colour is resolved
against the surface's theme and colour map and every transform applied *before* the value is
returned, and the result is interner-free: a concrete `RRGGBB` a caller can hand to a renderer
without holding the package open.

The raw inputs are still reachable when they are what you actually want:
[`Presentation::theme`] and [`Presentation::color_map`].

A colour that genuinely cannot be resolved — a scheme slot the theme does not define, a `phClr` with
no reference colour to substitute — keeps its unresolved form rather than being invented.

## Where resolution stops

| Reader | Stops with |
|---|---|
| fill, outline, effects | `None` — no candidate yielded one |
| transform | `None` — no tier placed the shape |
| bounds | `None` — as transform, and also when the placing tier names a rotation or flip without naming both an `a:off` and an `a:ext` |
| run / paragraph properties | never `None` — an **empty spec** |
| cell fill, cell border | `None` — the table resolves to no style at all, or no applicable part yielded one |
| cell run properties | never `None` — an **empty spec** |

The asymmetry is deliberate. For a fill, "nothing anywhere states one" and "there is no fill" are the
same statement. For text, they are not: an empty spec means every tier was consulted and none had an
opinion, which is a real answer about a real file, not a failure to find one.

## Cost

One effective read walks a bounded set of parts — the surface, its layout, its master, the theme,
`presentation.xml`, and for cells `tableStyles.xml`. Each is parsed on first touch and the parsed
tree is kept, so the first read of a deck is the expensive one.

Nothing about the *resolution* is cached, though: asking for the same shape's fill twice walks the
ladder twice. Reading a property for every run of a long text body is fine; doing it inside a loop
that also re-resolves the paragraph is worth hoisting.

## Examples

Declared versus effective, on a title that states nothing:

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("deck.pptx")?;
let mut deck = mjx_pptx::Presentation::open(&bytes)?;

// What the slide says: nothing.
assert!(deck.shape_fill(0, 0)?.is_none());

// What the renderer shows: whatever the layout, master and theme add up to.
if let Some(fill) = deck.effective_shape_fill(0, 0)? {
    println!("{fill:?}");
}
# Ok(())
# }
```

The level axis — a bullet the slide never states:

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("deck.pptx")?;
let mut deck = mjx_pptx::Presentation::open(&bytes)?;

// Paragraph 1 of the body placeholder declares only its text. Its bullet, indent and
// size come from the `a:lvlNpPr` its level selects, on whichever tier defines one.
let paragraph = deck.effective_paragraph_properties(0, 1, 1)?;
println!("{:?}", paragraph.bullet());
# Ok(())
# }
```

[`shape_fill`]: Presentation::shape_fill
[`shape_bounds`]: Presentation::shape_bounds
[`run_properties`]: Presentation::run_properties
