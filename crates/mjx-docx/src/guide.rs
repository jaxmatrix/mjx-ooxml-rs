// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors
// `mjx_pptx::guide`'s own shape — see that module's doc comment for why: no page declares an item,
// and each module imports the crate's public vocabulary so the guide's intra-doc links resolve.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            AbstractNumbering, BlockPath, BookmarkResolution, Document, DocxError, FieldForm,
            GridDiscrepancy, HeaderFooterType, HyperlinkTarget, MergedCellType, NumberingInstance,
            NumberingLevel, NumberingLookup, PageMargins, PageOrientation, PageSize, Paragraph,
            Run, RunInnerContent, RunPath, SectionLocation, SectionSpan, StyleDefinition,
            StyleIndex, StyleSheet,
        };
    };
}

guide_vocabulary!();

/// The whole story once, end to end.
pub mod building_a_document {
    #![doc = include_str!("../docs/guide/building_a_document.md")]
    guide_vocabulary!();
}

/// Addressing a run, editing it precisely, and the annotations that hang off it.
pub mod text_and_formatting {
    #![doc = include_str!("../docs/guide/text_and_formatting.md")]
    guide_vocabulary!();
}

/// Placing structured content, and the sections and headers it sits inside.
pub mod tables_sections_and_headers {
    #![doc = include_str!("../docs/guide/tables_sections_and_headers.md")]
    guide_vocabulary!();
}

/// Where a property comes from when the run or the paragraph does not state it.
pub mod styles_and_inheritance {
    #![doc = include_str!("../docs/guide/styles_and_inheritance.md")]
    guide_vocabulary!();
}

/// What survives a round trip, and what this library does not model.
pub mod fidelity_and_gaps {
    #![doc = include_str!("../docs/guide/fidelity_and_gaps.md")]
    guide_vocabulary!();
}
