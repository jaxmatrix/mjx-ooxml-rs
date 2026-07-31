# Inheritance, layouts and masters

The page to read when a property is not where you expected it — when a title renders at 44pt and
nothing in the slide says 44.

## The three tiers

A slide inherits from a **layout**, which inherits from a **master**, which points at a **theme**.
Almost nothing a deck shows is stated on the slide itself; the slide states the exceptions.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
let layout = deck.slide_layout(0)?;                       // which layout slide 0 uses
let master = layout.and_then(|l| deck.layout_master(l));  // and which master that layout uses
# let _ = master;
# Ok(())
# }
```

A notes slide has its own shorter chain: it goes straight to the notes master.

## Editing a layout reaches every slide

Because inheritance is resolved at render time, changing a layout changes every slide built on it —
and does so **without touching a single slide part**, so all of them stay byte-identical.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# use mjx_dml::{ColorSpec, FillSpec};
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::Surface;

deck.set_shape_fill(Surface::Layout(1), 0, &FillSpec::solid(ColorSpec::Srgb("C00000".into())))?;
# Ok(())
# }
```

Every shape API accepts a [`Surface`], so a layout and a master are edited exactly as a slide is.

## Declared versus effective

This is the distinction the whole library turns on.

- **Declared** readers — `shape_fill`, `shape_bounds`, `run_properties` — answer *what this part
  states*. A property the part does not state comes back `None`. Use them when editing: they show what
  an edit would overwrite.
- **Effective** readers — the ten `effective_*` methods — answer *what a renderer shows*. They walk
  the layout, master, theme and `presentation.xml`, resolve the colours, and hand back a
  self-contained answer. Use them when measuring, laying out or rendering.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
assert!(deck.shape_fill(0, 0)?.is_none());          // the slide says nothing
let shown = deck.effective_shape_fill(0, 0)?;       // …but something is painted
# let _ = shown;
# Ok(())
# }
```

The same pair exists for outline, effects, transform, bounds, run properties, paragraph properties and
the three table-cell readers. Reading never dirties a part, so a deck you fully measure and save is
identical to the deck you opened.

**[The effective-properties guide](crate::effective_properties) is the full reference** — the candidate
walk, the seven text tiers, why colours bake to concrete `RRGGBB`, and where each reader stops. Two
points are worth repeating here because they surprise people:

- **A shape that is not a placeholder inherits no fill, outline, effect or position.** It has no slot
  to be matched on. (Text is the exception — it still takes a master text style.)
- **A transform is inherited whole.** The first tier that places a shape wins entirely; a shape cannot
  take its position from the layout and its size from the master.

## Placeholders and slots

Inheritance matches on a *slot*: a placeholder's type plus its index. A title on a slide inherits from
the title on its layout, which inherits from the title on the master.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
if let Some(info) = deck.shape_placeholder(0, 0)? {
    println!("{:?} slot {}, named {:?}", info.kind, info.index, info.name);
}
# Ok(())
# }
```

`None` means the shape is not a placeholder — and therefore that it inherits nothing structural.

## The theme

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
if let Some(theme) = deck.theme(0)? {
    println!("{theme:?}");
}
let map = deck.color_map(0)?;
# let _ = map;
# Ok(())
# }
```

A scheme colour is rarely a colour. `<a:schemeClr val="tx2"/>` names a slot, the master's `p:clrMap`
maps it to a different slot, the theme's colour scheme resolves that to a value, and then `lumMod`,
`shade` and `tint` transform it. Three parts, three interners. The `effective_*` readers do all of it
and return concrete `RRGGBB`; [`theme`](Presentation::theme) and
[`color_map`](Presentation::color_map) are there for when you want the raw inputs instead.

## Building on a layout

[`add_slide_from_layout`](Presentation::add_slide_from_layout) is inheritance used deliberately: the
new slide carries the layout's placeholders, positioned and formatted by inheritance rather than by
copied values, so a later change to the layout still reaches it.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
let slide = deck.add_slide_from_layout(1)?;
deck.set_shape_text_content(slide, 0, "Second quarter")?;
# Ok(())
# }
```

Compare [`add_slide`](Presentation::add_slide), which gives you an empty slide on the first layout —
useful when you are placing everything yourself.
