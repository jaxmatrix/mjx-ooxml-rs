//! Shared plumbing for the examples: where a template comes from and where output goes.
//!
//! This is deliberately the *only* place the examples touch a filesystem beyond their own final
//! write. The library never does — [`Workbook::open`](mjx_xlsx::Workbook::open) takes `&[u8]` and
//! [`save`](mjx_xlsx::Workbook::save) returns `Vec<u8>`.
//!
//! The same module exists under `crates/mjx-pptx/examples/` and `crates/mjx-docx/examples/`, with
//! the same three functions. It is copied rather than shared because an example's `mod support;`
//! can only reach a file beside it, and a fourth crate whose only purpose is three filesystem
//! helpers for `examples/` would be a workspace member nothing ships.

#![allow(dead_code)] // Each example uses a subset.
#![allow(unreachable_pub)] // `pub` is the only visibility that reaches an example's `main`.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// The repository's `tests/fixtures/` directory.
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

/// A fixture's bytes.
pub fn fixture(name: &str) -> Result<Vec<u8>> {
    let path = fixture_dir().join(name);
    std::fs::read(&path).with_context(|| format!("reading fixture {}", path.display()))
}

/// The workbook the template-based examples start from.
///
/// `build_a_workbook.rs`, `style_a_range.rs`, `table_and_autofilter.rs` and `large_sparse_sheet.rs`
/// need none of this: `Workbook::blank` builds a workbook from nothing. The others begin from a file
/// because they are about editing or reading one. Substitute any `.xlsx` of your own by setting
/// `MJX_TEMPLATE`.
pub fn template() -> Result<Vec<u8>> {
    match std::env::var_os("MJX_TEMPLATE") {
        Some(path) => {
            std::fs::read(&path).with_context(|| format!("reading {}", path.to_string_lossy()))
        }
        None => fixture("sample.xlsx"),
    }
}

/// Where an example writes: its first argument, or `default` under the target directory.
pub fn output_path(default: &str) -> PathBuf {
    match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            let _ = std::fs::create_dir_all(&dir);
            dir.join(default)
        }
    }
}
