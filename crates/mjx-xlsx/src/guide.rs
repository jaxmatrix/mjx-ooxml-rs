// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s
// and `mjx_docx::guide`'s shape — see either module's own doc comment for why each module imports
// the crate's public vocabulary: so that the guide's intra-doc links resolve.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            ChartSeriesFreshness, CommentBox, HyperlinkKind, HyperlinkTarget, PartClassification,
            PartInventoryEntry, PartKind, RangeCellValue, RangeProblem, ResolvedArea,
            ResolvedRange, ResolvedRangeCell, Sheet, SheetChartSeries, SheetChartSource,
            SheetChartWorkbook, SheetComment, SheetHyperlink, SheetKind, SheetMarkup, SheetTable,
            SheetTableColumn, SpreadsheetDefect, Workbook, WorkbookParts, Worksheet,
            WorksheetParts, XlsxError,
        };
    };
}

guide_vocabulary!();

/// The whole of the current surface, once, in the order you meet it.
pub mod opening_and_saving {
    #![doc = include_str!("../docs/guide/opening_and_saving.md")]
    guide_vocabulary!();
}

/// Getting a value out of a sheet, and one into it.
pub mod reading_and_editing_cells {
    #![doc = include_str!("../docs/guide/reading_and_editing_cells.md")]
    guide_vocabulary!();
}

/// A workbook written from code, and the surface for filling it in.
pub mod authoring_a_workbook {
    #![doc = include_str!("../docs/guide/authoring_a_workbook.md")]
    guide_vocabulary!();
}

/// Formulas as text, the cached values beside them, and why neither is ever recalculated.
pub mod formulas_and_cached_values {
    #![doc = include_str!("../docs/guide/formulas_and_cached_values.md")]
    guide_vocabulary!();
}

/// Merged ranges, row and column geometry, page breaks, sheet protection — and why none of it is
/// ever repaired on read.
pub mod the_sheet_grid {
    #![doc = include_str!("../docs/guide/the_sheet_grid.md")]
    guide_vocabulary!();
}

/// Which conditional-formatting rules apply to a cell, in which order — and why none of them is
/// ever evaluated.
pub mod conditional_formatting {
    #![doc = include_str!("../docs/guide/conditional_formatting.md")]
    guide_vocabulary!();
}

/// Autofilters, sort state and data validation — and why none of the three is ever applied.
pub mod filters_and_data_validation {
    #![doc = include_str!("../docs/guide/filters_and_data_validation.md")]
    guide_vocabulary!();
}

/// Everything on a sheet that is not a cell — the three anchors, what each promises when the grid
/// moves, and which half of an anchor's resolved rectangle is a measurement.
pub mod worksheet_drawings {
    #![doc = include_str!("../docs/guide/worksheet_drawings.md")]
    guide_vocabulary!();
}

/// A chart on a sheet — the one chart in this library whose data is a live range, and what happens
/// when its cache and its cells disagree.
pub mod charts {
    #![doc = include_str!("../docs/guide/charts.md")]
    guide_vocabulary!();
}

/// A table is a part of its own — the four things that have to agree, and why a preset style name
/// is not a missing style.
pub mod worksheet_tables {
    #![doc = include_str!("../docs/guide/worksheet_tables.md")]
    guide_vocabulary!();
}

/// Print setup, opaque header/footer strings, custom views, and the four sheet kinds — one of which
/// has no cells at all.
pub mod print_setup_and_sheet_kinds {
    #![doc = include_str!("../docs/guide/print_setup_and_sheet_kinds.md")]
    guide_vocabulary!();
}

/// A cell comment is two parts in two vocabularies — and half of one is a file Excel repairs.
pub mod cell_comments_and_legacy_content {
    #![doc = include_str!("../docs/guide/cell_comments_and_legacy_content.md")]
    guide_vocabulary!();
}

/// A hyperlink and its relationship are one thing — and an external target is never followed.
pub mod hyperlinks {
    #![doc = include_str!("../docs/guide/hyperlinks.md")]
    guide_vocabulary!();
}

/// What survives a round trip, what this crate does not model, and what a save refuses.
pub mod fidelity_and_the_part_graph {
    #![doc = include_str!("../docs/guide/fidelity_and_the_part_graph.md")]
    guide_vocabulary!();
}

/// What a sheet costs to hold, what it costs to open, and why the second one is paid on every call.
pub mod large_workbooks {
    #![doc = include_str!("../docs/guide/large_workbooks.md")]
    guide_vocabulary!();
}

/// The page to read before filing a bug: every deliberate refusal, its reason, and its workaround.
pub mod deliberate_limitations {
    #![doc = include_str!("../docs/guide/deliberate_limitations.md")]
    guide_vocabulary!();
}

/// The same workbook through `mjx-ooxml`, Python and TypeScript — and the one place the four
/// surfaces deliberately do not agree.
pub mod through_the_facade {
    #![doc = include_str!("../docs/guide/through_the_facade.md")]
    guide_vocabulary!();
}
