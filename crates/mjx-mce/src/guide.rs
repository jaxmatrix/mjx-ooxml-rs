// The sixth page of the packaging tier's guide, whose other five live in `mjx-opc` and whose index
// is `crates/mjx-opc/docs/guide/README.md`. It is hosted here rather than there because `mjx-opc`
// and `mjx-mce` are the same layering rank: neither may depend on the other, so a page in that
// directory could not resolve a single intra-doc link into this crate (MJXOFF-215).
//
// One page, so this module *is* the page rather than an index over submodules — the shape
// `mjx_pptx::effective_properties` uses for the same reason.
#![doc = include_str!("../docs/markup_compatibility.md")]

// Imported so the page's intra-doc links resolve, exactly as the other four guide sets do.
#[allow(unused_imports)]
use crate::{
    resolve, NamespaceScope, ResolveError, ResolvedElement, ResolvedNode, UnderstoodNamespaces,
};
