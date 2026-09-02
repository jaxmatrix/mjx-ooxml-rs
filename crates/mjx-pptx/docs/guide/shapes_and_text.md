# Shapes and text

Everything here is about saying precisely *which* thing you mean — which shape, which run, which
characters — and then changing it.

## One index space

A slide's shapes live in **one index space covering every kind**: autoshapes (`p:sp`), pictures
(`p:pic`), groups (`p:grpSp`), graphic frames (`p:graphicFrame`, which is what tables and charts are),
connectors (`p:cxnSp`). They are numbered in document order, which is also back-to-front paint order.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
for shape in deck.shapes(0)? {
    println!("{}: {:?}", shape.index, shape.kind);
}
# Ok(())
# }
```

There is no separate list of pictures. A picture is shape 3 if it is the fourth thing on the slide.
[`shapes`](Presentation::shapes) hands you the whole inventory in one read — index, kind, and the
placeholder slot each fills — and [`shape_kind`](Presentation::shape_kind) answers for one address you
already have. The `p:spPr` surface — fill, outline, effects, geometry, transform — applies to shapes,
pictures and connectors alike. Text APIs return [`PptxError::ShapeHasNoTextBody`] for a kind that has
none.

## Descending into groups

A group counts as **one** shape on the top-level space. To reach inside it, pass an array instead of a
number: `[2, 1]` is member 1 of the group at index 2, nesting as deep as the groups do.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
let members = deck.shape_member_count(0, 2)?;
let text = deck.shape_text(0, [2, 1])?;
# let _ = (members, text);
# Ok(())
# }
```

That is [`ShapePath`], and every shape API takes `impl Into<ShapePath>` — so a bare `usize` and an
array are both accepted, and you never construct one explicitly unless you are storing it.

## Surfaces: slides are not the only thing with shapes

A layout has shapes. So does a master, a notes slide, and the notes master. [`Surface`] names which,
and every shape API takes `impl Into<Surface>` — a bare `usize` means `Surface::Slide(n)`.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# use mjx_dml::{ColorSpec, FillSpec};
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::Surface;

// Recolour the title placeholder on layout 1 — every slide built on it follows.
deck.set_shape_fill(Surface::Layout(1), 0, &FillSpec::solid(ColorSpec::Srgb("C00000".into())))?;
# Ok(())
# }
```

This is the highest-leverage idea in the library. Editing a layout reaches every slide that inherits
from it, without touching a single slide part — so the slides stay byte-identical.

## Text has four scopes

A text body is paragraphs; a paragraph is runs; a run is a span of characters sharing one set of
character properties. You can address any level.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
let all = deck.shape_text(0, 1)?;                     // every paragraph, newline-joined
let para = deck.paragraph_text(0, 1, 0)?;             // one paragraph
let run = deck.run_text(0, 1, 0, 0)?;                 // one run
# let _ = (all, para, run);
# Ok(())
# }
```

Writing has the matching pair: [`set_shape_text`](Presentation::set_shape_text) replaces one run's
text and leaves its formatting alone;
[`set_shape_text_content`](Presentation::set_shape_text_content) replaces the whole body with a single
run, which is what you want when you are filling a placeholder and do not care what was there.

## Formatting a range of characters

The four setters mirror the four scopes — run, paragraph, whole shape, and an arbitrary character
range. The range setter **splits runs** so the range gets its own.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_dml::CharacterPropertiesSpec;

let bold = CharacterPropertiesSpec::new().with_bold(true);
deck.set_text_range_properties(0, 1, 0, 6..11, &bold)?;
# Ok(())
# }
```

Two things to know. First, **an unset property means "leave alone", never "clear"** — a spec setting
only `bold` will not wipe the colour. Second, the plain range setter counts `char`s; if you are
working with user-supplied text containing emoji or combining marks, use
[`set_text_range_properties_by_grapheme`](Presentation::set_text_range_properties_by_grapheme), which
counts what a reader would call a character.

Splitting runs repeatedly leaves a paragraph fragmented.
[`coalesce_paragraph_runs`](Presentation::coalesce_paragraph_runs) merges adjacent runs back together
when their *effective* formatting matches, and returns how many it removed.

## List formatting for the whole shape

The four setters above each name a place in the text. A fifth scope sits underneath all of them: the
shape's own list style (`a:lstStyle`), which says *every paragraph at this indent level, in this
shape*. State it once and every paragraph at that level picks it up — including paragraphs added
later, which is what makes it different from looping the paragraph setter.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_dml::{CharacterPropertiesSpec, IndentLevel, ParagraphPropertiesSpec};

deck.set_shape_list_style_level(
    0,
    1,
    IndentLevel::TOP,
    &ParagraphPropertiesSpec::new()
        .with_bullet_character("•")
        .with_left_margin_points(18.0)
        .with_default_run_properties(CharacterPropertiesSpec::new().with_size_points(20.0)),
)?;
let stated = deck.shape_list_style_level(0, 1, IndentLevel::TOP)?;
# let _ = stated;
# Ok(())
# }
```

The level's `with_default_run_properties` is how its *character* formatting — size, weight, colour —
is stated; there is one spec for both halves because `a:lvlNpPr` carries both.

Three rules. The setter **merges**, as every other setter does, so naming an indent does not drop the
bullet a previous call set. A paragraph that states the property itself still wins — this tier sits
*beneath* the paragraph, not above it. And what you remove falls through rather than becoming a
default: [`clear_shape_list_style_level`](Presentation::clear_shape_list_style_level) hands the level
back to the layout, the master and `presentation.xml`, while
[`clear_shape_list_style`](Presentation::clear_shape_list_style) drops the element entirely.
[`set_shape_list_style_default`](Presentation::set_shape_list_style_default) states the `a:defPPr` that
answers where the style names no level at all.

## Editing one shape several ways

Stating the address once per edit reads badly. [`Presentation::shape`] opens a [`ShapeCursor`]: the
address once, the edits after it, applied together in one pass over the part.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# use mjx_dml::{ColorSpec, FillSpec};
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
# let (navy, gold) = (FillSpec::solid(ColorSpec::Srgb("1F3864".into())), FillSpec::solid(ColorSpec::Srgb("FFC000".into())));
deck.shape(0, 2)?                    // the group at top-level index 2
    .member(0)?.fill(navy)
    .sibling(1)?.fill(gold).text("Q3")
    .apply()?;
# Ok(())
# }
```

**Nothing is written until `.apply()`.** The cursor records edits and navigates — `.member(i)`,
`.sibling(i)`, `.parent()` — so a whole group can be restyled in one traversal. It is marked
`#[must_use]` for exactly that reason: a cursor you drop is a set of edits you threw away.

## Position and size

[`shape_bounds`](Presentation::shape_bounds) and
[`set_shape_bounds`](Presentation::set_shape_bounds) work in absolute EMU on the slide.
[`shape_transform`](Presentation::shape_transform) is the fuller picture — offset, extent, rotation,
and the two mirror flags.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::ShapeBounds;

if let Some(bounds) = deck.shape_bounds(0, 1)? {
    println!("{} EMU wide", bounds.width_emu);
}
deck.set_shape_bounds(0, 1, ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0))?;
# Ok(())
# }
```

A placeholder often declares no bounds at all — it takes them from the layout. `shape_bounds` answers
`None` in that case, honestly reporting that the slide says nothing; `effective_shape_bounds` is the
one that walks the layout and master. See [the inheritance page](inheritance_and_masters).

## Geometry, and the guides it is drawn from

[`shape_geometry`](Presentation::shape_geometry) answers with one of three things: a **preset** shape
(a `roundRect`, an `arc`), a **custom** path list a freeform shape was drawn as, or *inherited* —
the shape states no geometry and takes one from its layout.

A custom geometry does not have to place its points at numbers. DrawingML lets a coordinate name a
*guide* instead, and the guide is a formula over the shape's own width and height:
`<a:gd name="x1" fmla="*/ w adj1 100000"/>` puts a point a quarter of the way across a shape whose
`adj1` is 25000. Tell the library how big the shape is and it evaluates the whole guide list —
all seventeen operators, the built-in variables (`w`, `hc`, `ss`, `3cd4`, …) — and hands back the same
geometry in plain EMU and angles.

The same evaluator answers the other geometry question a preset shape raises: *how far can I drag
this adjustment?* A preset's domain is frequently a guide rather than a number — a `chevron`'s point
may not exceed `maxAdj`, which is `*/ 100000 w ss`, so it depends on the shape's proportions.
[`shape_adjustments`](Presentation::shape_adjustments) resolves both the value and the domain.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_dml::{GuideContext, Size};
use mjx_pptx::Geometry;

let size = GuideContext::from_size(Size::from_emu(1_828_800, 914_400));

if let Geometry::Custom(spec) = deck.shape_geometry(0, 1)? {
    let resolved = spec.resolve(size)?;
    println!("{} paths, every point a number", resolved.paths.len());
}

for adjustment in deck.shape_adjustments(0, 1, size)? {
    println!(
        "{}: {} in {}..={}",
        adjustment.spec.wire_name, adjustment.value, adjustment.minimum, adjustment.maximum,
    );
}
# Ok(())
# }
```

The size is a parameter, not the shape's own extents, for the same reason `shape_bounds` can answer
`None`: a placeholder inherits its size, and the library will not guess which one you mean.

Resolving is a **read**. The formula text stays exactly as the file wrote it, so asking where a point
is never changes a byte of what is written back. A malformed or self-referential guide list is a typed
error ([`PptxError::GuideFormula`]), never a panic and never a loop: guides are evaluated once, in the
order the file declares them, which is the order [the spec itself
requires](https://ecma-international.org/publications-and-standards/standards/ecma-376/) and which
leaves a cycle nowhere to form.

## Grouping

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
let group = deck.group_shapes(0, &[1usize.into(), 2usize.into(), 3usize.into()])?;
let freed = deck.ungroup(0, group)?;
# let _ = freed;
# Ok(())
# }
```

Grouping computes the group's bounds from its members and sets up the child coordinate space so
nothing moves. Resizing a group afterwards rescales its members — that is PowerPoint's behaviour, and
it is reproduced rather than special-cased.

## Hyperlinks

A link is either an external URL or a jump to another slide; the packaging difference is handled for
you.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::Hyperlink;

deck.set_run_hyperlink(0, 1, 0, 0, &Hyperlink::Url("https://example.com".into()))?;
deck.set_shape_hyperlink(0, 2, &Hyperlink::Slide(3))?;
# Ok(())
# }
```

Clearing a link removes the relationship it created, so the package does not accumulate dead
external references.
