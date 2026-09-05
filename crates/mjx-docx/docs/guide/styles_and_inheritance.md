# Styles, numbering and inheritance

A `.docx` states remarkably little about how a run looks. This page is about the other places the
answer lives, and how to author them. The deep reference behind it is the
[effective-properties guide](crate::effective_properties) — the exact ECMA-376 clauses, the
toggle-property rule, and where this reader stops. Read this page first; read that one when an answer
surprises you.

The runnable version is `examples/styles_and_numbering.rs`:

```sh
cargo run -p mjx-docx --example styles_and_numbering -- out.docx
```

## The ladder

Lowest priority to highest, per ECMA-376 Part 1 §17.7.2:

```text
docDefaults  →  table style  →  numbering level  →  paragraph-style chain  →  character style  →  direct
(lowest)                                                                                        (highest)
```

The rung most people place wrongly is the numbering level: it sits **below** the paragraph-style
chain, not above it. A document whose numbering level and paragraph style disagree about the same
property is the only place that distinction is visible, and it is the case this crate's own
`tests/effective.rs` fixtures are built around.

Two families of readers answer two different questions. The **declared** readers
([`StyleSheet`], [`ParagraphProperties`](crate::ParagraphProperties),
[`RunProperties`](crate::RunProperties)) answer *what this part says* — the right readers for
editing, because they show what an edit would overwrite. The **effective** readers
([`effective_run_properties`](Document::effective_run_properties),
[`effective_paragraph_properties`](Document::effective_paragraph_properties)) answer *what Word
renders*.

Neither dirties a part. The effective readers take `&mut self` because resolving may have to parse
parts the package had not needed yet.

## A blank document has no styles at all

`Document::blank` relates to no `word/styles.xml`, and this library does not fabricate one. Every
rung of the ladder is therefore silent, and an effective read comes back with every field `None` —
which is a real answer ("nothing anywhere had an opinion"), not a failure to find one.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Says nothing about itself")?;

assert!(document.style_sheet(|_, _| ())?.is_none());
let effective = document.effective_run_properties(0, 0)?;
assert_eq!(effective.font_size, None);
assert_eq!(effective.bold, None);
# Ok(())
# }
```

## Document defaults, and a style that nothing references

[`edit_style_sheet`](Document::edit_style_sheet) creates `word/styles.xml`, registers its content
type and relates it from the main document part, all on the first call. One primitive covers every
authoring shape — adding a style, changing one, starting a style sheet from nothing — because a style
has dozens of properties and a `Document`-level method per property would be a worse API than one
that hands you the element.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize, StyleDefinition};
use mjx_ooxml_types::wordprocessingml::{HalfPointMeasure, StyleType};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Says nothing about itself")?;

// The bottom rung: ten point, for every run that states no size.
document.edit_style_sheet(|sheet, interner| {
    sheet
        .document_defaults_or_insert(interner)
        .run_properties_default_or_insert(interner)
        .run_properties_or_insert(interner)
        .set_font_size(interner, Some(HalfPointMeasure::from_wire("20")));
})?;
assert_eq!(
    document.effective_run_properties(0, 0)?.font_size,
    Some(HalfPointMeasure::from_wire("20"))
);

// A style that exists but that nothing references changes nothing.
document.edit_style_sheet(|sheet, interner| {
    let mut style = StyleDefinition::new(interner, StyleType::Paragraph, "Heading1");
    style.set_name(interner, Some("heading 1"));
    style
        .run_properties_or_insert(interner)
        .set_font_size(interner, Some(HalfPointMeasure::from_wire("28")));
    sheet.add_style(style);
})?;
assert_eq!(
    document.effective_run_properties(0, 0)?.font_size,
    Some(HalfPointMeasure::from_wire("20"))
);
# Ok(())
# }
```

## The `w:basedOn` chain

A style inherits from the style its `w:basedOn` names, up to a root. Within one chain the rule is
plain fallback: the leaf's own value wins, and an unstated property falls through to the nearest
ancestor that states it. [`StyleIndex`] builds the lookup, and
[`based_on_chain`](StyleIndex::based_on_chain) returns the chain leaf-first.

A chain that does not terminate is a **typed error**, never a truncated `Ok`: a resolver handed a
silently shortened chain would produce a plausible-looking wrong answer with nothing red anywhere.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, DocxError, PageSize, StyleDefinition, StyleIndex};
use mjx_ooxml_types::wordprocessingml::StyleType;

let mut document = Document::blank(PageSize::a4())?;
document.edit_style_sheet(|sheet, interner| {
    // A two-deep chain …
    sheet.add_style(StyleDefinition::new(interner, StyleType::Paragraph, "Base"));
    let mut leaf = StyleDefinition::new(interner, StyleType::Paragraph, "Leaf");
    leaf.set_based_on(interner, Some("Base"));
    sheet.add_style(leaf);
    // … and a style that names itself.
    let mut cycle = StyleDefinition::new(interner, StyleType::Paragraph, "Cycle");
    cycle.set_based_on(interner, Some("Cycle"));
    sheet.add_style(cycle);
})?;

let (chain, cycle) = document
    .style_sheet(|sheet, interner| {
        let index = StyleIndex::build(sheet, interner)?;
        Ok::<_, DocxError>((
            index.based_on_chain("Leaf", interner)?.len(),
            index.based_on_chain("Cycle", interner).is_err(),
        ))
    })?
    .transpose()?
    .unwrap_or((0, false));

assert_eq!(chain, 2, "the leaf and its base");
assert!(cycle, "a cycle is reported, never walked forever");
# Ok(())
# }
```

## Toggle properties combine by XOR

Twelve run-level Boolean properties — `w:b`, `w:bCs`, `w:caps`, `w:emboss`, `w:i`, `w:iCs`,
`w:imprint`, `w:outline`, `w:shadow`, `w:smallCaps`, `w:strike`, `w:vanish` — do **not** combine by
override. §17.7.3: a direct value wins outright, a `true` at `docDefaults` wins outright, and
otherwise the tiers combine by Boolean XOR.

The consequence is worth stating plainly, because it looks like a bug the first time: a run whose
paragraph style says bold *and* whose character style also says bold renders **not bold**. Every
other `CT_OnOff`-shaped member, despite the identical wire shape, combines by plain override. The
[effective-properties guide](crate::effective_properties) has the full list and the fixture that
proves it.

## Numbering

Two elements, two identifiers. `w:abstractNum` is the definition — formats, templates, start values.
`w:num` is the instance a paragraph names, and it points at the definition.
[`attach_paragraph_to_list`](Document::attach_paragraph_to_list) writes the paragraph's own
`w:numPr`; [`edit_numbering`](Document::edit_numbering) creates `word/numbering.xml` on first use,
exactly as `edit_style_sheet` does for styles.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{
    AbstractNumbering, Document, LevelNumberFormat, LevelTextTemplate, NumberingInstance,
    NumberingLevel, NumberingLookup, PageSize, RunProperties,
};
use mjx_ooxml_types::wordprocessingml::{HalfPointMeasure, NumberFormat};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Not a list item")?;
document.append_paragraph()?;
document.append_run(1, "A list item")?;

document.edit_numbering(|numbering, interner| {
    let mut level = NumberingLevel::new(interner, 0);
    level.set_start(interner, Some(1));
    level.set_format(Some(LevelNumberFormat::new(interner, NumberFormat::Decimal)));
    level.set_text_template(Some(LevelTextTemplate::new(interner, "%1.")));
    // A level carries run properties of its own — one rung of the ladder.
    let mut runs = RunProperties::new(interner);
    runs.set_font_size(interner, Some(HalfPointMeasure::from_wire("24")));
    level.set_run_properties(Some(runs));

    let mut definition = AbstractNumbering::new(interner, 1);
    definition.push_level(level);
    numbering.push_abstract_numbering(definition);
    numbering.push_instance(NumberingInstance::new(interner, 1, 1));
})?;
document.attach_paragraph_to_list(1, 1, 0)?;

// The numbering rung answers for the attached paragraph, and only for it.
assert_eq!(
    document.effective_run_properties(1, 0)?.font_size,
    Some(HalfPointMeasure::from_wire("24"))
);
assert_eq!(document.effective_run_properties(0, 0)?.font_size, None);

// `resolve_numbering` walks instance → definition → level, following `w:numStyleLink` indirection.
let start = document.resolve_numbering(1, 0, |lookup, _| match lookup {
    NumberingLookup::Resolved(resolution) => resolution.effective_start(),
    NumberingLookup::None => None,
})?;
assert_eq!(start, Some(1));
# Ok(())
# }
```

Computing the *displayed* number a list item renders — "3.2.1" — is out of scope. That needs the
whole document's numbering state walked in reading order, restart rules included, which is a
rendering feature; see [fidelity and the known gaps](fidelity_and_gaps).

## Table cells have one extra rung

A run inside a table cell is affected by the table style's **conditional formatting**: the style can
say one thing for the first row, another for banded columns, another for the corner cells. Three
readers answer with that rung included —
[`effective_cell_fill`](Document::effective_cell_fill),
[`effective_cell_border`](Document::effective_cell_border) and
[`effective_cell_run_properties`](Document::effective_cell_run_properties).

With no `w:tblStyle` at all, every one of them degrades to "the cell's own direct formatting, nothing
more" — the same "no style, no opinion" behaviour every other tier has.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let table = document.append_table(1, 1)?;
document.set_cell_text(table, 0, 0, "Hello, cell.")?;

assert_eq!(document.effective_cell_fill(table, 0, 0)?, None);
assert_eq!(
    document.effective_cell_run_properties(table, 0, 0, 0, 0)?.bold,
    None
);
# Ok(())
# }
```

## What to read next

[Fidelity and the known gaps](fidelity_and_gaps), before relying on any of this in production.
