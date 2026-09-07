# Text bodies

DrawingML text is four elements deep and the whole model follows from that:

```xml
<a:txBody>            <!-- TextBody      — bodyPr, lstStyle, then paragraphs -->
  <a:p>               <!-- Paragraph     — pPr, then runs / breaks / fields  -->
    <a:r>             <!-- TextRun       — rPr, then the text                -->
      <a:t>hello</a:t><!-- Text          — the decoded string               -->
```

A shape's words are a text body. So are a table cell's, a chart title's and a SmartArt node's — one
model, reached from everywhere, and `p:txBody` on a slide is the *same* `CT_TextBody` as `a:txBody`
in a table cell, wrapped in a different namespace.

## What is typed, and what is not

**Four types are typed: [`mjx_dml::TextBody`](crate::TextBody),
[`mjx_dml::Paragraph`](crate::Paragraph), [`mjx_dml::TextRun`](crate::TextRun) and
[`mjx_dml::Text`](crate::Text)** — plus, later,
[`mjx_dml::TextLineBreak`](crate::TextLineBreak) (`a:br`) and
[`mjx_dml::TextField`](crate::TextField) (`a:fld`) as paragraph content, and
[`mjx_dml::TextListStyle`](crate::TextListStyle) (`a:lstStyle`) on the body.

Everything else — `a:bodyPr`, `a:endParaRPr`, an unknown child, a comment, whitespace — is kept as an
opaque node. Not in a side bucket: in the **same ordered vector** as the typed children, as a `Raw`
variant of [`mjx_dml::TextBodyContent`](crate::TextBodyContent),
[`mjx_dml::ParagraphContent`](crate::ParagraphContent) or
[`mjx_dml::RunContent`](crate::RunContent). Order is fidelity here — a text body's `a:bodyPr` and
`a:lstStyle` come *before* its first `a:p`, and a side bucket would lose the interleave.

That is why the accessors are filters rather than fields:

```text
body.paragraphs()      → the Paragraph variants of body.content()
paragraph.runs()       → the TextRun variants, skipping breaks and fields
paragraph.text()       → every run's text concatenated
body.content()         → the whole ordered vector, typed and raw alike
```

## Formatting a run without destroying it

A run's `a:rPr` is [`mjx_dml::CharacterProperties`](crate::CharacterProperties) — a fidelity wrapper
with typed accessors over roughly thirty attributes and six children (fill, outline, effects,
highlight, underline fill and line, and the four font slots of
[`mjx_dml::FontSlot`](crate::FontSlot)).

There are two ways to write to it and they are not interchangeable:

```rust,ignore
// merge: writes only what the spec names, leaves lang / dirty / hyperlinks / unknowns alone
properties.apply(&CharacterPropertiesSpec::new().with_bold(true), interner);

// build fresh: everything the old element carried is gone. The local name is the caller's,
// because an `rPr` is `a:rPr` in one host and `a:defRPr` or `a:endParaRPr` in another.
let properties = CharacterPropertiesSpec::new()
    .with_bold(true)
    .to_properties(interner, "rPr");
```

[`mjx_dml::CharacterProperties::apply`](crate::CharacterProperties::apply) is what makes bolding a run
PowerPoint wrote a non-destructive operation. **An unset field means "don't touch", not "remove"** —
so `apply` can never clear a property, and clearing one is what the fresh build is for.
[`mjx_dml::Paragraph::set_properties`](crate::Paragraph::set_properties) and
[`set_end_properties`](crate::Paragraph::set_end_properties) are the same contract one level up, for
`a:pPr` and `a:endParaRPr`.

[`mjx_dml::CharacterPropertiesSpec`](crate::CharacterPropertiesSpec) is a builder —
[`with_bold`](crate::CharacterPropertiesSpec::with_bold),
[`with_size_points`](crate::CharacterPropertiesSpec::with_size_points),
[`with_color`](crate::CharacterPropertiesSpec::with_color),
[`with_fill`](crate::CharacterPropertiesSpec::with_fill),
[`with_outline`](crate::CharacterPropertiesSpec::with_outline) and a dozen more — and takes points
where the wire takes hundredths of a point, because every UI that shows a font size shows points.

Two accessors exist for callers that need to know whether an element holds anything the model does
*not* understand, which is what a coalescing pass has to ask before merging two runs:
[`has_only_modeled_state`](crate::CharacterProperties::has_only_modeled_state) and
[`unmodeled_state_eq`](crate::CharacterProperties::unmodeled_state_eq).
[`mjx_dml::Paragraph::split_run_at`](crate::Paragraph::split_run_at) and
[`coalesce_adjacent_runs`](crate::Paragraph::coalesce_adjacent_runs) are the operations that need
them.

## The one place the derive loses something, stated plainly

[`mjx_dml::Text`](crate::Text) (`a:t`) is a `#[xml(text)]` leaf. The derive's text arm reads only text and CDATA nodes
and writes a single minimally-escaped text node, so **an entity spelling, a character reference, a
CDATA section or an interleaved comment inside `a:t` does not survive a rebuild** — `&#38;` comes back
as `&amp;`.

This is a **write-path** property, not a reader one. `mjx-xml`'s fidelity reader never decodes text at
all, so an untouched part carrying `&#38;` round-trips byte for byte with no model involved
(`crates/mjx-xml/tests/subtree_cow.rs`). Only a part something has edited through the typed model
re-flows its text nodes. Five `mjx-sml` types decline the derive for exactly this reason and hand-write
their pair; `mjx_dml::Text` uses the derive and accepts the loss, because DrawingML text is what an
editing caller edits and the entity spelling of a character it just replaced is not information.

## Bullets and list styles

A paragraph's bullet is not one element but a family: [`mjx_dml::Bullet`](crate::Bullet) covers the
character, autonumber and picture forms, with
[`mjx_dml::BulletColor`](crate::BulletColor), [`BulletSize`](crate::BulletSize) and
[`BulletTypeface`](crate::BulletTypeface) each carrying their own three-way "follow the text /
explicit / none" choice, because that is what the schema says rather than a simplification made here.
[`mjx_dml::TextListStyle`](crate::TextListStyle) (`a:lstStyle`) is the nine-level default cascade a
body applies to paragraphs that state nothing themselves; resolving a paragraph against it is the
host's job, and `mjx-pptx`'s effective-properties walk is where that lives.
