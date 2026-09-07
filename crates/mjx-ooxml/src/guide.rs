// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s,
// `mjx_docx::guide`'s and `mjx_xlsx::guide`'s shape — see any of them for why each module imports
// the crate's public vocabulary: so that the guide's intra-doc links resolve.
//
// What this set does *not* do is repeat the other three. Each of those describes one format; this
// one describes the surface all three are reached through, and every page is about a question that
// only has an answer here (MJXOFF-214).
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::document::{BlockPath, RunPath};
        #[allow(unused_imports)]
        use crate::{
            detect_format, CellBlock, CellData, CellInput, CellWrite, CharacterPropertiesSpec,
            ChartData, ChartKind, ColorSpec, Deck, Document, EffectListSpec, Error, ErrorCode,
            ErrorDetail, FillSpec, Format, FormatFamily, GridAnomalyInfo, LineSpec, PageSize,
            ParagraphPropertiesSpec, PptxError, PresetShapeType, ResizingBehavior, ShapeBounds,
            ShapePath, SlideSize, Surface, Workbook,
        };
    };
}

guide_vocabulary!();

/// Bytes in, bytes out: what a package is, which surface opens it, and what saving refuses.
pub mod opening_and_saving {
    #![doc = include_str!("../docs/guide/opening_and_saving.md")]
    guide_vocabulary!();
}

/// Naming a thing on each of the three surfaces — and the four calls that take the column first.
pub mod addressing {
    #![doc = include_str!("../docs/guide/addressing.md")]
    guide_vocabulary!();
}

/// What a caller who learned one format already knows about the other two.
pub mod one_vocabulary_three_surfaces {
    #![doc = include_str!("../docs/guide/one_vocabulary_three_surfaces.md")]
    guide_vocabulary!();
}

/// One error type, eleven codes, and the typed cause still underneath.
pub mod errors {
    #![doc = include_str!("../docs/guide/errors.md")]
    guide_vocabulary!();
}

/// What this crate selects from the three below it, what it leaves behind, and the one gap.
pub mod the_curated_surface {
    #![doc = include_str!("../docs/guide/the_curated_surface.md")]
    guide_vocabulary!();
}

/// The round-trip contract, what is preserved rather than modelled, and what is not verified yet.
pub mod fidelity_and_gaps {
    #![doc = include_str!("../docs/guide/fidelity_and_gaps.md")]
    guide_vocabulary!();
}
