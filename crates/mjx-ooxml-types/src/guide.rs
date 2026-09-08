// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s,
// `mjx_docx::guide`'s, `mjx_xlsx::guide`'s, `mjx_ooxml::guide`'s, `mjx_opc::guide`'s,
// `mjx_dml::guide`'s, `mjx_sml::guide`'s and `mjx_chart::guide`'s shape — see any of them for why
// each module imports the crate's public vocabulary: so that the guide's intra-doc links resolve.
//
// This set is unlike the other eight in one way, and the difference is the reason MJXOFF-224 wrote
// it: almost nothing here was written by a person. 84,107 of the crate's 85,296 lines are emitted by
// `xtask/src/codegen/`, and until MJXOFF-224 nothing re-derived them. `what_to_distrust` is the page
// that says so, and it is the one to read first.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        // Types rather than modules: a `use crate::child_order;` here would collide with this
        // set's own page module of that name, and an intra-doc link to a module resolves from its
        // full path anyway.
        #[allow(unused_imports)]
        use crate::child_order::{
            audit_tree, ChildOrder, ChildSlot, ContentModel, OutOfOrderChild, TreeAudit,
            TypeReference,
        };
        #[allow(unused_imports)]
        use crate::drawingml::{
            adjustable_shapes, adjustment_bound_guides_of, adjustments_of, AdjustmentAxis,
            AdjustmentBound, AdjustmentSpec, PresetGuide, PresetShapeType,
        };
        #[allow(unused_imports)]
        use crate::namespaces::SchemaNamespace;
        #[allow(unused_imports)]
        use crate::{
            on_off, true_false, true_false_blank, HexColorRgb, OnOff, TrueFalse, TrueFalseBlank,
            UnknownWireValue,
        };
    };
}

guide_vocabulary!();

/// The thirteen committed artefacts, the nine simple-type modules, the shape of an emitted item, and
/// the preset-shape tables that come from no schema.
pub mod what_is_generated {
    #![doc = include_str!("../docs/guide/what_is_generated.md")]
    guide_vocabulary!();
}

/// The 59,512 lines that say where a child element belongs, what rank means, and the three rules
/// that keep placement from ever reordering a document.
pub mod child_order {
    #![doc = include_str!("../docs/guide/child_order.md")]
    guide_vocabulary!();
}

/// How a cryptic `ST_*` token becomes a self-explanatory Rust name, which part of that is mechanical
/// and which is curated by hand, and what checks each half.
pub mod the_naming_convention {
    #![doc = include_str!("../docs/guide/the_naming_convention.md")]
    guide_vocabulary!();
}

/// What `cargo run -p xtask -- codegen` needs, why the output is committed rather than built, what
/// that costs, and how a generator change lands.
pub mod regenerating {
    #![doc = include_str!("../docs/guide/regenerating.md")]
    guide_vocabulary!();
}

/// The honest page: which gate catches what, which one skips silently, and the four things nothing
/// here checks at all.
pub mod what_to_distrust {
    #![doc = include_str!("../docs/guide/what_to_distrust.md")]
    guide_vocabulary!();
}
