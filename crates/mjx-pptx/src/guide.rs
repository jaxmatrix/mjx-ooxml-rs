// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item.
//
// Each module imports the crate's public vocabulary so the guides' intra-doc links resolve. The
// imports are otherwise unused, which is what the `allow` is for.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            AxisOrientation, CellFormat, Cells, ChartAxisData, ChartData, ChartDataError,
            ChartKind, ChartLabelScope, DataLabelSettings, DataLabelSpec, Geometry, Hyperlink,
            LayoutInfo, LegendPosition, PlaceholderInfo, PptxError, Presentation, ShapeBounds,
            ShapeCursor, ShapeInfo, ShapeKind, ShapePath, SlideSize, Surface, TableStyleDefinition,
            TableStyleFormat,
        };
    };
}

guide_vocabulary!();

/// The whole story once, end to end.
pub mod building_a_deck {
    #![doc = include_str!("../docs/guide/building_a_deck.md")]
    guide_vocabulary!();
}

/// Addressing a shape, and editing its text precisely.
pub mod shapes_and_text {
    #![doc = include_str!("../docs/guide/shapes_and_text.md")]
    guide_vocabulary!();
}

/// Placing structured content.
pub mod tables_charts_pictures {
    #![doc = include_str!("../docs/guide/tables_charts_pictures.md")]
    guide_vocabulary!();
}

/// Where a property comes from when the slide does not state it.
pub mod inheritance_and_masters {
    #![doc = include_str!("../docs/guide/inheritance_and_masters.md")]
    guide_vocabulary!();
}

/// What survives a round trip, and what this library does not model.
pub mod fidelity_and_gaps {
    #![doc = include_str!("../docs/guide/fidelity_and_gaps.md")]
    guide_vocabulary!();
}
