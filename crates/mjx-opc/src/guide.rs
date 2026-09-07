// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s,
// `mjx_docx::guide`'s, `mjx_xlsx::guide`'s and `mjx_ooxml::guide`'s shape — see any of them for why
// each module imports the crate's public vocabulary: so that the guide's intra-doc links resolve.
//
// This set covers a *tier* rather than a crate, because the fidelity mechanism is spread over four
// of them and no one of them can be understood alone (MJXOFF-215). It is hosted here because
// `mjx-opc` is the only crate in the tier that can see two of the other three: `mjx-ooxml-core` and
// `mjx-xml` are below it, so their symbols resolve. `mjx-mce` is the same rank, so it cannot be
// named in an intra-doc link at all, and its page is hosted by that crate as `mjx_mce::guide`.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            doc_props, ImageFormat, OpcError, Package, PackageDefect, PartBody, PartName,
            PartProvenance, Relationship, TargetMode, ZipEntry,
        };
    };
}

guide_vocabulary!();

/// Parts, names, content types, relationships — and why saving checks the graph before it writes.
pub mod the_package {
    #![doc = include_str!("../docs/guide/the_package.md")]
    guide_vocabulary!();
}

/// What an open package costs, which of its two meanings of "lazy" is the true one, and when a
/// part's bytes are actually released.
pub mod laziness_and_copy_on_write {
    #![doc = include_str!("../docs/guide/laziness_and_copy_on_write.md")]
    guide_vocabulary!();
}

/// Four removals, four blast radii, and the one an edit is allowed to call.
pub mod removing_a_part {
    #![doc = include_str!("../docs/guide/removing_a_part.md")]
    guide_vocabulary!();
}

/// The lossless tree under every part, the byte ranges that make an untouched subtree free, and the
/// two readers only one of which preserves.
pub mod the_preservation_tree {
    #![doc = include_str!("../docs/guide/the_preservation_tree.md")]
    guide_vocabulary!();
}

/// What the round-trip contract promises, what enforces each clause of it, and what it leaves out.
pub mod the_round_trip_contract {
    #![doc = include_str!("../docs/guide/the_round_trip_contract.md")]
    guide_vocabulary!();
}
