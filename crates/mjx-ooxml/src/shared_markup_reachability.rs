// A documentation-only module: the page lives in `docs/shared_markup_reachability.md` so that it
// reads as prose on a source host and renders as a chapter in this crate's rustdoc — the same
// arrangement `mjx_pptx::effective_properties` uses. There is no code here, and nothing imports it.
//
// `crates/mjx-ooxml/tests/shared_markup_reachability.rs` re-derives both of the page's tables from
// the workspace itself and fails when they disagree with it.
#![doc = include_str!("../docs/shared_markup_reachability.md")]
