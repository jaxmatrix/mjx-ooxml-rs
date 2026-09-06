//! Shared helpers for this crate's integration suites.
//!
//! Each integration test is its own crate, so a helper here is `pub(crate)` *within that test's*
//! crate; a plain `pub` would trip the workspace's `unreachable_pub` lint. `dead_code` is allowed
//! for the same reason: a suite that uses one helper and not the other is not carrying dead code, it
//! is one of several crates sharing a file.
#![allow(dead_code)]

pub(crate) mod plain_text;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mjx_text::FontFace;

/// The bundled tier-2 faces `mjx-text` commits under `assets/fonts/`.
///
/// Reached by a path relative to *this* crate's manifest rather than through an API, because there
/// is no API to reach them through: `mjx-text` ships the files (its `include` list names
/// `assets/fonts/*.ttf`) but exposes no accessor, which is MJXOFF-155 §9 item 16 and is not this
/// child's to fix. A workspace-relative path is honest about that and breaks loudly if the layout
/// moves, rather than skipping — a test that silently skips is the vacuous gate this programme keeps
/// finding.
pub(crate) fn bundled_font_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts")
}

/// Read and parse one bundled face by file name.
pub(crate) fn bundled_face(file_name: &str) -> Arc<FontFace> {
    let path = bundled_font_directory().join(file_name);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("reading the committed face `{}`: {error}", path.display()));
    Arc::new(
        FontFace::parse(Arc::from(bytes.as_slice()), 0).unwrap_or_else(|error| {
            panic!("parsing the committed face `{}`: {error}", path.display())
        }),
    )
}

/// Liberation Sans — metrically compatible with Arial, and the face whose `AV` pair kerns.
pub(crate) fn liberation_sans() -> Arc<FontFace> {
    bundled_face("LiberationSans-Regular.ttf")
}

/// Carlito — metrically compatible with Calibri, and the one bundled face that forms `fi` and `ffi`
/// ligatures.
pub(crate) fn carlito() -> Arc<FontFace> {
    bundled_face("Carlito-Regular.ttf")
}

/// A deterministic 64-bit generator, for the randomised fragment sets the spatial index is checked
/// against.
///
/// `xorshift64*`: eight lines, no dependency, and — the property that matters for a gate — the same
/// sequence on every machine and every run, so a failure is reproducible from the seed printed
/// beside it. A `rand` dependency for a test would put a crate in the graph to produce numbers whose
/// only requirement is that they be varied and repeatable.
#[derive(Clone, Debug)]
pub(crate) struct DeterministicNumbers {
    state: u64,
}

impl DeterministicNumbers {
    pub(crate) fn from_seed(seed: u64) -> Self {
        Self {
            // A zero state is xorshift's one fixed point, so it is moved off it.
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        let mut state = self.state;
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        self.state = state;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..bound`, or zero for an empty bound.
    pub(crate) fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }

    /// A number in `low..=high`.
    pub(crate) fn between(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        // The span fits an `i128` for any pair of `i64`s, so this cannot wrap.
        let span = (i128::from(high) - i128::from(low) + 1) as u64;
        low.saturating_add(self.below(span) as i64)
    }
}
