# Guide

Five pages, in reading order. Each is written to be read start to finish once, then returned to as a
reference.

| Page | Read it when |
|---|---|
| [Building a deck](building_a_deck) | You want the whole story once: start blank or from a file, add slides, fill them, save |
| [Shapes and text](shapes_and_text) | You need to address a particular shape, or edit text precisely |
| [Tables, charts and pictures](tables_charts_pictures) | You are placing structured content |
| [Inheritance, layouts and masters](inheritance_and_masters) | A property is not where you expected it |
| [Fidelity and the known gaps](fidelity_and_gaps) | Before you rely on something in production |

The [effective-properties guide](crate::effective_properties) is the deep reference behind page 4 —
what a file *states* versus what a renderer *shows*.

## The shape of the API, in one page

Three facts explain most of it.

**Bytes in, bytes out.** [`Presentation::open`] takes `&[u8]` and [`save`](Presentation::save)
returns `Vec<u8>`; [`Presentation::blank`] takes no bytes at all and builds a deck from nothing. The
library never touches a filesystem, a network, or a clock. Whoever calls it owns the file handle —
which is also why the same code compiles to WebAssembly and runs in a browser unchanged.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("in.pptx")?;
let mut deck = mjx_pptx::Presentation::open(&bytes)?;
// … edits …
std::fs::write("out.pptx", deck.save()?)?;
# Ok(())
# }
```

**Everything is addressed, nothing is handed out.** There is no `Slide` object and no `Shape` object
to hold. You name what you want on each call — a surface and a shape index — and the deck answers.
That is what keeps a `.pptx`'s copy-on-write fidelity intact: the library knows exactly which part
you touched, so every part you did not touch is re-emitted byte-for-byte.

**Reads take `&mut self`.** Not because reading changes the document — it does not — but because a
part is raw bytes until something needs it parsed. The first read of a slide materialises its tree;
the deck you save is identical to the deck you opened.

## Units

Offsets and sizes are **EMU** (English Metric Units): 914 400 to the inch, 12 700 to the point.
[`ShapeBounds::from_inches`] exists so you rarely have to care, and the constants
[`EMU_PER_INCH`](ShapeBounds::EMU_PER_INCH) / [`EMU_PER_POINT`](ShapeBounds::EMU_PER_POINT) are
public for when you do. Font sizes are in **points** on every `*Spec` builder.
