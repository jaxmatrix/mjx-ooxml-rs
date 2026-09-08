// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_ooxml::guide`'s
// shape — see it for why the guides are wired this way at all.
//
// This is **one guide set over two crates**: `mjx-python` and `mjx-wasm` are siblings, neither may
// see the other, and the projection is one story told twice. The four pages that are about the
// projection live here; the one that is about the npm package alone lives in `mjx_wasm::guide`,
// exactly as the packaging tier's MCE page lives in `mjx-mce` rather than in `mjx-opc`.
//
// No `guide_vocabulary!` macro and no intra-doc item links: the snippets on these pages are Python
// and JavaScript, and the names they discuss are the *projected* names rather than Rust items.
// Every claim that a gate can check is written as a repository path or a crate-qualified symbol in
// a code span, which is what `xtask/tests/doc_gate.rs` reads.
#![doc = include_str!("../docs/guide/README.md")]

/// The wheel, the npm package, and what building either from source needs.
pub mod installing {
    #![doc = include_str!("../docs/guide/installing.md")]
}

/// Identity in Python, camelCase in TypeScript, and the five differences the target languages force.
pub mod the_mapping_rules {
    #![doc = include_str!("../docs/guide/the_mapping_rules.md")]
}

/// The nine methods that stay in Rust, the one real gap, and two classes nothing can produce.
pub mod what_is_not_projected {
    #![doc = include_str!("../docs/guide/what_is_not_projected.md")]
}

/// What the two suites actually call, measured, and the gate that keeps the figure from expiring.
pub mod how_much_is_exercised {
    #![doc = include_str!("../docs/guide/how_much_is_exercised.md")]
}
