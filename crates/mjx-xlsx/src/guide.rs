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
            PartClassification, PartInventoryEntry, PartKind, Sheet, SheetKind, SpreadsheetDefect,
            Workbook, WorkbookParts, Worksheet, WorksheetParts, XlsxError,
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

/// What survives a round trip, what this crate does not model, and what a save refuses.
pub mod fidelity_and_the_part_graph {
    #![doc = include_str!("../docs/guide/fidelity_and_the_part_graph.md")]
    guide_vocabulary!();
}
