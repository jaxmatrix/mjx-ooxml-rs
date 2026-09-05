//! Shared plumbing for the examples: where the template comes from and where output goes.
//!
//! This is deliberately the *only* place the examples touch a filesystem beyond their own final
//! write. The library never does.
//!
//! The four helpers are `mjx-pptx`'s four, by name and by signature
//! (`crates/mjx-pptx/examples/support/mod.rs`) — a reader who has run one crate's examples already
//! knows this one's. The one difference is where [`fixture_dir`] gets its path: `mjx-pptx`'s
//! recomputes `../../tests/fixtures` from its own manifest directory, while this one asks
//! [`mjx_fixtures`], the crate that already owns the corpus for every byte-identity suite and schema
//! gate in the workspace. Two spellings of the same directory is exactly the drift `mjx-fixtures`
//! was created to end.

#![allow(dead_code)] // Each example uses a subset.
#![allow(unreachable_pub)] // `pub` is the only visibility that reaches an example's `main`.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// The repository's `tests/fixtures/` directory.
pub fn fixture_dir() -> PathBuf {
    mjx_fixtures::fixtures_dir()
}

/// A fixture's bytes.
pub fn fixture(name: &str) -> Result<Vec<u8>> {
    let path = fixture_dir().join(name);
    std::fs::read(&path).with_context(|| format!("reading fixture {}", path.display()))
}

/// The document the template-based examples start from — the corpus's plain Word document, with a
/// real `styles.xml`, `fontTable.xml`, `settings.xml` and theme.
///
/// `blank_document.rs` needs none of this: `Document::blank` builds a document from nothing. The
/// other examples begin from a file when they are about editing somebody *else's* markup.
/// Substitute any `.docx` of your own by setting `MJX_TEMPLATE`.
pub fn template() -> Result<Vec<u8>> {
    match std::env::var_os("MJX_TEMPLATE") {
        Some(path) => {
            std::fs::read(&path).with_context(|| format!("reading {}", path.to_string_lossy()))
        }
        None => fixture("sample.docx"),
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
