//! Where an Office-authored original for an area lives, and what happens when there is none.
//!
//! **The slot, not its ingestion.** MJXOFF-130 (F3) owns the corpus itself — how a file gets here,
//! what is checked on the way in, and the `mc:Ignorable` / `CT_Extension` trap its first ingestion
//! will hit. This module owns one question only: *does this area have an original to edit, and if
//! not, what exactly did we look for?*
//!
//! # Why the corpus is not under `tests/fixtures/`
//!
//! `mjx-fixtures` derives every byte-identity corpus and the schema gate's corpus from a `read_dir`
//! of `tests/fixtures/`, and [`mjx_fixtures::assert_every_fixture_has_a_known_kind`] fails on any
//! entry it cannot classify — a subdirectory included. Putting Office-authored originals there would
//! either break that sweep or force them into corpora they do not belong to: they are inputs to a
//! human pass, not committed fixtures this library round-trips. They live beside it instead.
//!
//! # Skipping is never silent
//!
//! An absent original makes the edit variant skip **by name**, printing the area and the path it
//! looked for. `MJX_REQUIRE_OFFICE_CORPUS=1` turns any such skip into a failure, which is the same
//! arrangement `MJX_REQUIRE_SOFFICE=1` makes for the `office_open` canary and `MJX_REQUIRE_SCHEMA=1`
//! for the schema gate: the expectation lives in the code, and CI decides whether absence is
//! tolerable.
//!
//! [`mjx_fixtures::assert_every_fixture_has_a_known_kind`]: https://docs.rs/mjx-fixtures

use std::path::PathBuf;

use anyhow::{Context, Result};

use super::Area;

/// Where Office-authored originals live, relative to the workspace root.
pub const CORPUS_DIRECTORY: &str = "tests/office-authored";

/// The absolute path of [`CORPUS_DIRECTORY`].
#[must_use]
pub fn corpus_directory() -> PathBuf {
    super::workspace_root().join(CORPUS_DIRECTORY)
}

/// Where this area's Office-authored original would be, whether or not it exists.
///
/// One file per area, named by the area's own id, so a reviewer dropping a file in knows exactly
/// which entry it answers: `tests/office-authored/v-pptx-03.pptx`.
#[must_use]
pub fn original_path(area: &Area) -> PathBuf {
    corpus_directory().join(format!(
        "{}.{}",
        area.id.to_ascii_lowercase(),
        area.format.extension()
    ))
}

/// The bytes of this area's Office-authored original, or `None` when the slot is empty.
///
/// # Errors
/// If the file exists but cannot be read — an unreadable original is a problem to report, not an
/// absence to skip over.
pub fn original_for(area: &Area) -> Result<Option<Vec<u8>>> {
    let path = original_path(area);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(Some(bytes))
}
