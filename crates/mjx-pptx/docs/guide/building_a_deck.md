# Building a deck

One continuous story: open a file, add slides, put things on them, save. Everything else in the guide
is a closer look at a step you meet here.

The runnable version of this page is `examples/build_a_deck.rs`:

```sh
cargo run -p mjx-pptx --example build_a_deck -- out.pptx
```

## Where the first deck comes from

**Today, from a file you supply.** [`Presentation::open`] is the only constructor — there is no
`Presentation::blank()` yet, so a deck cannot be conjured from nothing. In practice this is less
limiting than it sounds: the deck you open is a *template*, and a template is what you want anyway —
it carries the theme, the master and the layouts that give a deck its identity. A one-slide file
exported from PowerPoint is a fine starting point, and so is
[`tests/fixtures/layouts.pptx`](https://github.com/jaxmatrix/mjx-ooxml-rs/blob/main/tests/fixtures/layouts.pptx)
in this repository.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_pptx::Presentation;

let template = std::fs::read("template.pptx")?;
let mut deck = Presentation::open(&template)?;
# Ok(())
# }
```

## Look before you edit

A template you did not author is worth inspecting first. The three questions that matter are *what
layouts do I have*, *what does slide 0 contain*, and *which of those shapes are placeholders I am
expected to fill*.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
println!("{} slides, {} layouts", deck.slide_count(), deck.layout_count());

for index in 0..deck.layout_count() {
    let name = deck.layout_name(index)?.unwrap_or_default();
    println!("layout {index}: {name} ({:?})", deck.layout_kind(index)?);
}
# Ok(())
# }
```

[`layout_kind`](Presentation::layout_kind) answers with a `SlideLayoutKind` — `Title`, `TwoColumnText`,
`ObjectOnly` and so on — which is the layout's declared *intent*. [`layout_name`](Presentation::layout_name)
is what a human called it. Neither is guaranteed to be sensible in a file you did not write, so read
both.

## Add a slide, and fill it

[`add_slide_from_layout`](Presentation::add_slide_from_layout) is the method that does real work. It
copies the layout's placeholders onto a new slide — ready to fill, in the right positions, inheriting
the right formatting — and hands you the new slide's index.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
let slide = deck.add_slide_from_layout(1)?;

for shape in 0..deck.shape_count(slide)? {
    if let Some(placeholder) = deck.shape_placeholder(slide, shape)? {
        println!("shape {shape} is a {:?} placeholder", placeholder.kind);
    }
}
# Ok(())
# }
```

Deliberately, it does **not** copy the date, footer and slide-number placeholders: those render from
the layout already, and copying them onto every slide is how decks accumulate junk.

Now fill them. A placeholder is an ordinary shape on the slide's index space, so
[`set_shape_text_content`](Presentation::set_shape_text_content) is all it takes.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let slide = 0usize;
deck.set_shape_text_content(slide, 0, "Quarterly results")?;
deck.set_shape_text_content(slide, 1, "Revenue up 14% year on year")?;
# Ok(())
# }
```

Note what did **not** happen: nothing set a font, a size or a colour. The title renders at the
master's title size in the theme's major typeface because that is what the layout and master say, and
[the inheritance page](inheritance_and_masters) explains how to find out what that resolved to.

## Add your own shapes

Two constructors cover most needs. [`add_text_box`](Presentation::add_text_box) makes a plain text
box; [`add_shape`](Presentation::add_shape) makes one of the 117 named preset geometries. Both take a
[`ShapeBounds`] and return the new shape's index.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let slide = 0usize;
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_pptx::ShapeBounds;

let caption = deck.add_text_box(slide, "Source: internal", ShapeBounds::from_inches(0.5, 6.5, 4.0, 0.4))?;
let badge = deck.add_shape(slide, PresetShapeType::Ellipse, ShapeBounds::from_inches(8.0, 0.4, 1.2, 1.2))?;
# let _ = (caption, badge);
# Ok(())
# }
```

`from_inches(x, y, width, height)` is the ergonomic constructor; the underlying unit is EMU, 914 400
to the inch. Coordinates are absolute on the slide, origin top-left.

## Make it look like something

Fill, outline and effects are three independent surfaces on the same shape, and each takes an
interner-free *spec* you build up front.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let (slide, badge) = (0usize, 0usize);
use mjx_dml::{ColorSpec, FillSpec, LineSpec, LineWidth};

deck.set_shape_fill(slide, badge, &FillSpec::solid(ColorSpec::Srgb("1F3864".into())))?;
deck.set_shape_outline(
    slide,
    badge,
    &LineSpec::solid(LineWidth::from_points(1.5), ColorSpec::Srgb("FFFFFF".into())),
)?;
# Ok(())
# }
```

`ColorSpec::Srgb` takes six hex digits and no leading `#`. You can also name a theme colour with
`ColorSpec::Scheme(..)`, which is usually the better choice — it follows the theme when the theme
changes.

Setting three properties on one shape reads badly when spelled out three times. That is what
[`ShapeCursor`] is for; [the shapes page](shapes_and_text) covers it.

## Pictures

[`add_picture`](Presentation::add_picture) takes raw image bytes, works out the format by sniffing
magic bytes, adds the media part, wires the relationship, and places a picture shape — in one call.
The image is stored exactly as you handed it over; nothing is decoded or re-encoded.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let slide = 0usize;
use mjx_pptx::ShapeBounds;

let logo = std::fs::read("logo.png")?;
deck.add_picture(slide, &logo, ShapeBounds::from_inches(7.5, 0.3, 1.5, 1.5))?;
# Ok(())
# }
```

Adding the same bytes twice reuses one media part — deduplication is by content, so a logo on twenty
slides costs one copy.

## A table

[`add_table`](Presentation::add_table) creates the grid; cells are then addressed by `(row, column)`.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let slide = 0usize;
use mjx_dml::{ColorSpec, FillSpec};
use mjx_pptx::{CellFormat, Cells, ShapeBounds};

let table = deck.add_table(slide, 3, 2, ShapeBounds::from_inches(1.0, 2.0, 6.0, 2.0))?;

deck.set_cell_text(slide, table, 0, 0, 0, "Region")?;
deck.set_cell_text(slide, table, 0, 1, 0, "Revenue")?;

deck.format_cells(
    slide,
    table,
    Cells::row(0),
    &CellFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into()))),
)?;
# Ok(())
# }
```

The `0` before the text is the *run* index — a cell's text is paragraphs of runs, same as any other
text body. [`Cells`] describes a selection (one cell, a row, a column, a rectangle, or all of them)
and [`CellFormat`] describes what to do to it, so bulk formatting is one call rather than a loop.

## A chart

[`ChartData`] is a builder; [`add_chart`](Presentation::add_chart) turns it into a chart part, a
relationship and a graphic frame on the slide.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let slide = 0usize;
use mjx_pptx::{ChartData, ChartKind, ShapeBounds};

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3", "Q4"])
    .series("2026", [12.0, 15.5, 14.0, 19.25]);

deck.add_chart(slide, &chart, ShapeBounds::from_inches(1.0, 2.0, 8.0, 4.0))?;
# Ok(())
# }
```

One caveat worth knowing before you rely on it: the chart carries **cached** values, which is what
PowerPoint renders, but no embedded workbook — so "Edit Data" in PowerPoint has nothing to open. See
[the gaps page](fidelity_and_gaps).

## Speaker notes

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
# let slide = 0usize;
deck.set_notes_text(slide, "Lead with the revenue number, then the regional split.")?;
# Ok(())
# }
```

A notes slide is created on demand if the deck has none, along with the notes master it needs.

## Save

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("template.pptx")?)?;
std::fs::write("out.pptx", deck.save()?)?;
# Ok(())
# }
```

[`save`](Presentation::save) returns the container bytes. Every part you did not touch is re-emitted
exactly as it arrived — not re-serialised from a model, but the original bytes — which is why you can
open a deck full of features this library has never heard of, change one word, and hand it back
intact. [The fidelity page](fidelity_and_gaps) states the guarantee precisely.

## What to read next

- Addressing a shape inside a group, or editing part of a paragraph → [Shapes and text](shapes_and_text)
- More on tables, charts and images → [Tables, charts and pictures](tables_charts_pictures)
- "Why is my text 18pt when nothing says 18pt?" → [Inheritance, layouts and masters](inheritance_and_masters)
- Before production → [Fidelity and the known gaps](fidelity_and_gaps)
