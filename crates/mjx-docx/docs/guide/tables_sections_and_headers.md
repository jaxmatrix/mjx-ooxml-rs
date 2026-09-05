# Tables, sections, headers and structured content

The four structures a paragraph can sit inside. Each has a shape that surprises somebody the first
time, and each of those is called out below rather than left to be discovered.

The runnable versions are `examples/build_table.rs`, `examples/sections_and_headers.rs`,
`examples/fields_and_hyperlinks.rs` and `examples/structured_content.rs`.

## Tables

[`append_table`](Document::append_table) builds a table with every cell already holding one empty
paragraph, so [`set_cell_text`](Document::set_cell_text) has somewhere to write.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let table = document.append_table(2, 2)?;
document.set_cell_text(table, 0, 0, "Region")?;
document.set_cell_text(table, 0, 1, "Growth")?;

assert_eq!(document.table_count()?, 1);
assert_eq!(document.table_dimensions(table)?, (2, 2));
assert_eq!(document.cell_text(table, 0, 1)?, "Growth");
# Ok(())
# }
```

### `(row, column)` is a grid position, not a cell

A merge means several grid positions are drawn by one cell.
[`cell_span`](Document::cell_span) answers how far a cell reaches, and
[`merged_cell_anchor`](Document::merged_cell_anchor) answers the other direction: given a position,
which cell actually holds it. Both are `(row, column)`-ordered, like every other cell method here.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, MergedCellType, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let table = document.append_table(3, 3)?;

// Horizontal: `w:gridSpan` on one cell.
document.set_cell_span(table, 0, 0, Some(2))?;
assert_eq!(document.cell_span(table, 0, 0)?, (1, 2));
assert_eq!(document.merged_cell_anchor(table, 0, 1)?, (0, 0));

// Vertical: `w:vMerge`, an anchor that restarts and continuations that continue.
document.set_cell_vertical_merge(table, 1, 0, Some(MergedCellType::Restart))?;
document.set_cell_vertical_merge(table, 2, 0, Some(MergedCellType::Continue))?;
assert_eq!(document.merged_cell_anchor(table, 2, 0)?, (1, 0));
# Ok(())
# }
```

Widening a cell does **not** delete the cell it now covers: this library never removes content a
caller did not ask it to remove. The row therefore spans more grid columns than `w:tblGrid` declares,
and [`table_grid_discrepancies`](Document::table_grid_discrepancies) says so by name rather than
silently normalising the table — which matters, because Word writes tables that disagree with their
own grids and a reader that "fixes" them loses the file's own intent.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, GridDiscrepancy, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let table = document.append_table(1, 2)?;
assert!(document.table_grid_discrepancies(table)?.is_empty());

document.set_cell_span(table, 0, 0, Some(2))?;
assert_eq!(
    document.table_grid_discrepancies(table)?,
    [GridDiscrepancy::RowWidthMismatch {
        row: 0,
        declared_columns: 2,
        spanned_columns: 3,
    }]
);
# Ok(())
# }
```

### Rows, columns, and the escape hatch

[`insert_row`](Document::insert_row) / [`remove_row`](Document::remove_row) /
[`insert_column`](Document::insert_column) / [`remove_column`](Document::remove_column) do the
structural edits; a row inserted inside a vertical merge grows the merge rather than splitting it.
[`edit_table`](Document::edit_table) and [`edit_cell`](Document::edit_cell) reach the `w:tblPr` and
`w:tcPr` themselves — the style reference, `w:tblLook`, band sizes, borders, shading — because there
are dozens of those and a named method for each would be a worse API than one that hands you the
element.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let table = document.append_table(1, 1)?;
document.edit_table(table, |table, interner| {
    if let Some(properties) = table.properties_mut() {
        properties.set_style_id(interner, Some("TableGrid"));
    }
})?;

let style = document.edit_table(table, |table, interner| {
    table
        .properties_mut()
        .and_then(|properties| properties.style_id(interner).ok().flatten())
})?;
assert_eq!(style.as_deref(), Some("TableGrid"));
# Ok(())
# }
```

## Sections

A section's properties live at the **end** of the range it governs, not at the start. `w:sectPr`
inside a paragraph's own `w:pPr` *ends* a section at that paragraph; the body-level `w:sectPr` is the
last section's. That is why [`SectionLocation`] has exactly two shapes —
`Paragraph(path)` and `Body` — and why [`SectionSpan`] reports a first and last paragraph rather
than a start marker.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageMargins, PageSize, SectionLocation};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Portrait")?;
document.append_paragraph()?;
document.append_run(1, "Landscape")?;
assert_eq!(document.sections(|spans, _| spans.len())?, 1);

// Ending a section at paragraph 0 makes two of them.
document.edit_section_properties(SectionLocation::Paragraph(0.into()), |properties, interner| {
    properties.set_page_size(interner, Some(PageSize::a4()));
    properties.set_page_margins(interner, Some(PageMargins::NORMAL));
})?;
document.edit_section_properties(SectionLocation::Body, |properties, interner| {
    properties.set_page_size(interner, Some(PageSize::a4().landscape()));
})?;

let orientations = document.sections(|spans, interner| {
    spans
        .iter()
        .map(|span| {
            span.properties
                .as_ref()
                .and_then(|properties| properties.page_size(interner).ok().flatten())
                .map(|size| size.orientation)
        })
        .collect::<Vec<_>>()
})?;
assert_eq!(orientations.len(), 2);
assert_eq!(orientations[0], Some(mjx_docx::PageOrientation::Portrait));
assert_eq!(orientations[1], Some(mjx_docx::PageOrientation::Landscape));
# Ok(())
# }
```

[`PageMargins::NORMAL`] is Word's own "Normal" template: 1 inch on every side, ½ inch header and
footer. `header` and `footer` are measured **from the page edge**, not from the body text margin —
a common misreading, restated here because getting it backwards silently produces a plausible file.

## Headers and footers

[`create_header`](Document::create_header) writes the part, registers its content type, relates it
from the main document part *and* wires `w:headerReference` into that section's own `w:sectPr` — all
four, because writing three of them is a file Word repairs. The new part holds one empty paragraph;
[`edit_header_footer`](Document::edit_header_footer) fills it in.

The reader that matters is [`resolve_header`](Document::resolve_header): a section that names no
header of its own **inherits** the previous section's (ECMA-376 Part 1 §17.10.1), and this answers
with the part that actually applies, not with the reference the section happens to carry.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, HeaderFooterType, PageMargins, PageSize, Paragraph, Run, SectionLocation};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "First section")?;
document.append_paragraph()?;
document.append_run(1, "Second section")?;
document.edit_section_properties(SectionLocation::Paragraph(0.into()), |properties, interner| {
    properties.set_page_margins(interner, Some(PageMargins::NORMAL));
})?;

let part = document.create_header(
    SectionLocation::Paragraph(0.into()),
    HeaderFooterType::Default,
)?;
document.edit_header_footer(&part, |content, interner| {
    if let Some(paragraph) = content.paragraph_mut(0) {
        paragraph.append_run(Run::with_text(interner, "Internal"));
    }
})?;

// Section 1 states no header of its own, and resolves to section 0's.
assert_eq!(
    document.resolve_header(1, HeaderFooterType::Default)?,
    Some(part.clone())
);
let text = document.header_footer(&part, |content, _| {
    content.paragraphs().map(Paragraph::text).collect::<String>()
})?;
assert_eq!(text, "Internal");

// Remove the only header, and there is nothing left to inherit.
document.remove_header(SectionLocation::Paragraph(0.into()), HeaderFooterType::Default)?;
assert!(document.resolve_header(1, HeaderFooterType::Default)?.is_none());
# Ok(())
# }
```

[`even_and_odd_headers`](Document::even_and_odd_headers) reads `w:settings/w:evenAndOddHeaders`,
which — together with a section's `w:titlePg` — decides which of
[`HeaderFooterType`]'s three variants a given page actually uses.

## Fields

WordprocessingML writes a field two ways: `w:fldSimple`, self-contained, and the
`begin`/`separate`/`end` form spread across sibling runs. [`fields`](Document::fields) is one read
model over both, and [`FieldForm`] says which one produced it.

**Nesting is paired with a stack, never counted.** A `TOC` field's cached result can hold `PAGEREF`
fields of its own, each with its own `begin`/`end`. A reader that counts markers reports three
top-level fields where there is one, and mis-scopes every instruction between them.

[`set_field_instruction`](Document::set_field_instruction) and
[`set_field_cached_result_text`](Document::set_field_cached_result_text) are deliberately separate:
a field's *code* and its last *rendered value* are different things, and this library never evaluates
one into the other. Refreshing a field is Word's job.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
// A blank document has no fields at all — the reader says so rather than guessing.
assert!(document.fields(0)?.is_empty());
# Ok(())
# }
```

## Structured content, and where addressing stops

A content control (`w:sdt`) or a custom-XML wrapper (`w:customXml`) can appear anywhere a paragraph,
a run, a table row or a table cell can. Row and cell addressing sees through the row- and cell-level
ones: `(row, column)` reaches the same cell whether or not a wrapper stands between the table and it.

The boundary is at the block level. A table wrapped in a **block-level** content control is not one
of the body's own top-level tables, so [`table_count`](Document::table_count) does not count it.
Reaching it is a walk down the wrapper's own `content()` — `examples/structured_content.rs` does that
walk, and this page names the limit rather than leaving it to be found.

[`resolve_data_binding`](Document::resolve_data_binding) resolves a control's
`w:dataBinding` — a store item id plus an XPath — across the Custom XML Data Storage parts
[`custom_xml_parts`](Document::custom_xml_parts) lists. The value it finds is what Word would push
into the control, which is **not** necessarily the text the control currently displays.

[`add_alt_chunk`](Document::add_alt_chunk) imports a whole other document — HTML, RTF, or a nested
`.docx` — as a part the body references. The payload is stored exactly as handed over and never
inspected or converted; Word does the import when the file is opened.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{constants::CONTENT_TYPE_ALT_CHUNK_HTML, Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let html = b"<html><body><p>Imported.</p></body></html>".to_vec();
let id = document.add_alt_chunk(CONTENT_TYPE_ALT_CHUNK_HTML, html.clone())?;

let (payload, content_type) = document.alt_chunk_payload(&id)?;
assert_eq!(payload, html.as_slice());
assert_eq!(content_type, "text/html");
assert_eq!(document.alt_chunk_parts()?.len(), 1);
# Ok(())
# }
```

## What to read next

[Styles and inheritance](styles_and_inheritance), for where a table's or a paragraph's formatting
comes from when it states none of its own.
