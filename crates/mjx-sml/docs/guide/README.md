# Guide

**SpreadsheetML is not Excel's.** A `.xlsx` package is Excel's, but the markup inside it is shared:
a chart authored on a PowerPoint slide embeds a whole workbook at `/ppt/embeddings/*.xlsx` — that
package is what **Edit Data** opens — and a chart in a Word document does the same. So `mjx-chart`
needed a SpreadsheetML writer before Excel existed here, and this crate is where the markup lives so
that all three formats reach one copy of it.

`mjx-sml` is **rank 2.1**: it may reach [`mjx-dml`](https://docs.rs/mjx-dml) (2.0), `mjx-ooxml-types`,
`mjx-opc`, `mjx-mce`, `mjx-xml` and `mjx-ooxml-core`, and nothing above. `mjx-chart`, `mjx-omml` and
`mjx-vml` (2.2) reach *down* into it, and so do all three format crates — which is exactly what makes
`mjx-chart → mjx-sml → mjx-dml` a chain of legal downward edges and what let `mjx-chart`'s duplicate
workbook writer be deleted. `xtask/tests/layering.rs` reads the real graph out of `cargo metadata`
and fails on any edge that does not point strictly down.

Six pages, in reading order.

| Page | Read it when |
|---|---|
| [Reaching SpreadsheetML](reaching_spreadsheetml) | You are wiring a crate onto this one, or wondering whether an answer is here or in `mjx-xlsx` |
| [The slot frame](the_slot_frame) | You are holding a part and want to know what *held* means, and why an untouched child comes back byte for byte |
| [The cell store](the_cell_store) | A sheet is large, or you want to know what a cell costs |
| [Shared strings](shared_strings) | A cell says `t="s"` and you want the text, or a string has to be written |
| [The stylesheet](the_stylesheet) | A colour, a font, a border or an `xf` — reading one, or authoring one |
| [Fidelity and the known gaps](fidelity_and_gaps) | Before you rely on something in production |

**If you have a `.xlsx` and want to open it, you are in the wrong crate.**
[`mjx-xlsx`](https://docs.rs/mjx-xlsx)'s guide is written for that caller: `Workbook::open`, cells,
formats, comments, charts and saving. Come here when a report does not carry what you need, or when
you are building a package rather than opening one.

## The shape of the API, in one page

There are around sixteen hundred public declarations here — this is the largest crate in the
workspace — and you do not need to meet them. Five facts explain the lot.

### 1 · A *part* owns its document; everything below it is a view over one element

Three things in this crate are whole parts, and each is a **frame**: a root element, its attributes,
and a vector of slots in document order.
[`mjx_sml::WorksheetPart`](crate::WorksheetPart) (`xl/worksheets/sheetN.xml`),
[`mjx_sml::WorkbookPart`](crate::WorkbookPart) (`xl/workbook.xml`) and
[`mjx_sml::StylesheetPart`](crate::StylesheetPart) (`xl/styles.xml`), joined by the three sheet kinds
that are not worksheets — [`mjx_sml::ChartSheetPart`](crate::ChartSheetPart),
[`mjx_sml::DialogSheetPart`](crate::DialogSheetPart) and
[`mjx_sml::MacroSheetPart`](crate::MacroSheetPart) — and by
[`mjx_sml::Comments`](crate::Comments).

Everything else — [`mjx_sml::Font`](crate::Font), [`mjx_sml::Fill`](crate::Fill),
[`mjx_sml::CellFormat`](crate::CellFormat), [`mjx_sml::AutoFilter`](crate::AutoFilter),
[`mjx_sml::SheetViews`](crate::SheetViews) and their several hundred siblings — wraps **one element**
of a part someone else owns, keeps the element's qualified name with its source prefix, all its
attributes verbatim, its self-closing flag and every child it has no accessor for, and is rebuilt
from those. [The slot frame](the_slot_frame) is the mechanism;
[Fidelity and the known gaps](fidelity_and_gaps) is what backs it.

**A worksheet is the exception, and the reason is a measurement.** It *consumes* the document it was
parsed from rather than borrowing one, because a 300,000-cell sheet held as a preservation tree costs
913 bytes of peak resident set per cell and [the packed store](the_cell_store) holds the same sheet in
36.8. Consuming the document is what makes the writer a byte writer, which is in turn what lets
`sheetData` be a packed store rather than a subtree.

### 2 · Interner-bound value, interner-free spec

A string in a preservation tree is a symbol in that part's [`Interner`](mjx_ooxml_core::Interner), so
a type holding one is only meaningful beside that interner — and a caller of
`mjx_xlsx::Workbook::append_pattern_fill` does not hold it. Every type a caller might reasonably want
to *describe* therefore has a plain-Rust twin whose name ends in `Spec`: public fields, no interner,
no lifetime, `Default` throughout, and one `build` method that turns the description into markup
inside the part that will hold it.

| The markup | The description |
|---|---|
| [`mjx_sml::Fill`](crate::Fill) | [`mjx_sml::PatternFillSpec`](crate::PatternFillSpec) |
| [`mjx_sml::Border`](crate::Border) | [`mjx_sml::BorderSpec`](crate::BorderSpec) / [`mjx_sml::BorderEdgeSpec`](crate::BorderEdgeSpec) |
| [`mjx_sml::CellFormat`](crate::CellFormat) | [`mjx_sml::CellFormatSpec`](crate::CellFormatSpec) |
| [`mjx_sml::DifferentialFormat`](crate::DifferentialFormat) | [`mjx_sml::DifferentialFormatSpec`](crate::DifferentialFormatSpec) |
| [`mjx_sml::ConditionalFormattingRule`](crate::ConditionalFormattingRule) | [`mjx_sml::ConditionalRuleSpec`](crate::ConditionalRuleSpec) |
| [`mjx_sml::WorksheetTable`](crate::WorksheetTable) | [`mjx_sml::WorksheetTableSpec`](crate::WorksheetTableSpec) |
| [`mjx_sml::RichTextRun`](crate::RichTextRun) | [`mjx_sml::RichTextRunSpec`](crate::RichTextRunSpec) |

Fonts are the deliberate exception: [`mjx_sml::FontProperties`](crate::FontProperties) is *already*
that description — a plain struct of `Option` fields — and
[`mjx_sml::Font::from_properties`](crate::Font::from_properties) is its build step. A `FontSpec`
would be a second copy of the same list.

### 3 · Absent means absent

Every spec field is an `Option`, and `None` writes **no attribute and no element**. That is not
politeness. `<xf/>` is a meaningful record naming font 0, fill 0, border 0 and `General` by omission;
`<patternFill/>` states a fill whose pattern is inherited; a border edge with no `@style` is `none`;
and a colour with no `@theme` is not a colour with `theme="0"`, which is the *first* theme colour. A
builder that filled in defaults would author markup the caller did not ask for, on a path whose whole
point is that this project can explain every byte it emits.

The same rule runs the other way on read. A `@count` is a **hint**: it is updated when the collection
is edited *and the file declared one*, and never added to an element that wrote none.

### 4 · Nothing is evaluated, and nothing is repaired

There is no calculation engine and there will not be one, so a formula is *text*
([`mjx_sml::CellFormula`](crate::CellFormula)), a cached `<v>` is what a producer last computed and is
never invalidated, an autofilter is a **record** of a filter rather than a filter, a sort state is a
record of a sort, and a conditional-formatting rule is described rather than resolved.

And a file that disagrees with itself is **reported**, never corrected: a `dimension` that does not
match the cells, a duplicate `row@r`, a merge that overlaps another and a table whose totals row does
not fit come back as a [`mjx_sml::GridAnomaly`](crate::GridAnomaly), a
[`mjx_sml::SheetDataAnomaly`](crate::SheetDataAnomaly) or as `None` from a method that cannot answer.
Correcting a file to match what this library expects is how a fidelity library loses the argument it
exists to win.

### 5 · Indices are positions, and a position is a promise

`styles.xml` is addressed by index the whole way down: a cell's `@s` is a position in `cellXfs`, an
`xf`'s `@fontId` a position in `fonts`, `@fillId` in `fills`, `@borderId` in `borders`, `@xfId` in
`cellStyleXfs`. So **appending never moves anything**, deduplication is never done on the quiet, and
two byte-identical `<font>` entries stay two entries.
`crates/mjx-sml/tests/style_resources.rs`'s
`font_indices_are_positions_and_appending_never_moves_them` is what holds that.

[The stylesheet](the_stylesheet) is the whole of that indirection, and
`crates/mjx-xlsx/docs/effective_properties.md` walks it end to end for a caller who wants to know
what a cell actually looks like.

## What this crate does not do

It does not open a package: `mjx-opc` is a lower rank and models the container, not the markup, and
resolving an `r:id` to a part is `mjx-xlsx`'s. It does not render. It does not calculate. And it
types no payload belonging to a crate above it — a drawing part's SpreadsheetDrawingML is
`mjx_dml::spreadsheet_drawing`'s, a chart inside it is `mjx-chart`'s, and the VML behind a comment box
is `mjx-vml`'s. This crate holds the `r:id` as the string the file wrote and says which prefix that
file bound the relationship namespace to.
