//! Shared plumbing for the examples: where the template comes from and where output goes.
//!
//! This is deliberately the *only* place the examples touch a filesystem beyond their own final
//! write. The library never does.

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

/// The deck the examples start from — a small multi-layout template.
///
/// There is no `Presentation::blank()` yet, so every example begins from a file. Substitute any
/// `.pptx` of your own by setting `MJX_TEMPLATE`.
pub fn template() -> Result<Vec<u8>> {
    match std::env::var_os("MJX_TEMPLATE") {
        Some(path) => {
            std::fs::read(&path).with_context(|| format!("reading {}", path.to_string_lossy()))
        }
        None => fixture("layouts.pptx"),
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
