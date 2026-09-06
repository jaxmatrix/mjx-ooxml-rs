//! Shared helpers for this crate's integration suites.
//!
//! Each integration test is its own crate, so a helper here is `pub(crate)` *within that test's*
//! crate; a plain `pub` would trip the workspace's `unreachable_pub` lint. `dead_code` is allowed
//! for the same reason: a suite that uses one helper and not the other is not carrying dead code,
//! it is one of several crates sharing a file.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mjx_text::FontFace;

/// The bundled tier-2 faces committed under `assets/fonts/`, with their licences and a `README.md`
/// recording each file's provenance and SHA-256.
pub(crate) fn bundled_font_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts")
}

/// Read and parse one bundled face by file name.
pub(crate) fn bundled_face(file_name: &str) -> FontFace {
    let path = bundled_font_directory().join(file_name);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("reading the committed face `{}`: {error}", path.display()));
    FontFace::parse(Arc::from(bytes.as_slice()), 0)
        .unwrap_or_else(|error| panic!("parsing the committed face `{}`: {error}", path.display()))
}
