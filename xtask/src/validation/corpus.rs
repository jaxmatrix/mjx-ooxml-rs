//! Where the Office-authored corpus is, what it holds, and what happens when it holds nothing.
//!
//! Two questions, and they are separate on purpose. *Does this area have an original to edit, and if
//! not, what exactly did we look for?* — [`original_for`], which the artefact generators ask.
//! *What is in the directory at all?* — [`corpus_files`], which `xtask/tests/office_corpus.rs` and
//! the ingest report ask. What is checked on the way in, and the `mc:Ignorable` / `CT_Extension`
//! seam the first ingestion meets, are [`super::ingest`]'s.
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

use anyhow::{bail, Context, Result};

use super::Area;

/// Where Office-authored originals live, relative to the workspace root.
pub const CORPUS_DIRECTORY: &str = "tests/office-authored";

/// The absolute path of [`CORPUS_DIRECTORY`].
///
/// Canonicalized when it can be, because this path is *printed* — the ingest report tells a person
/// where to put a file, and `…/xtask/../tests/office-authored/v-xlsx-02.xlsx` is a path somebody has
/// to read twice. It falls back to the uncanonicalized form rather than failing: an absent directory
/// is something [`corpus_files`] reports as an empty corpus, not something a path accessor raises.
#[must_use]
pub fn corpus_directory() -> PathBuf {
    let path = super::workspace_root().join(CORPUS_DIRECTORY);
    path.canonicalize().unwrap_or(path)
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

/// One file the corpus directory holds.
#[derive(Debug)]
pub struct CorpusFile {
    /// Its absolute path.
    pub path: PathBuf,
    /// Its file name, which is also how it binds to an area.
    pub name: String,
    /// The format its extension names.
    pub format: super::ArtefactFormat,
    /// The area it answers, when its name is an area id.
    pub area: Option<&'static Area>,
}

/// Every Office-authored file in the corpus, sorted by name.
///
/// **The corpus is the directory**, exactly as `mjx-fixtures` makes every byte-identity corpus a
/// `read_dir` rather than a `const` list: a file dropped in joins the suite by being *there*, and a
/// suite that iterates a hand-maintained list is a suite a new file silently sits outside of.
///
/// `README.md` is the only name allowed to be there without being a corpus file. Anything else the
/// walk cannot classify is an **error naming it**, not a skip — a `.xlsb`, a `.zip` or a stray
/// `.DS_Store` would otherwise be a file nobody ever checked, which is the shape of silence this
/// whole phase is written against.
///
/// # Errors
/// If the directory cannot be read, or it holds an entry the walk cannot classify.
pub fn corpus_files() -> Result<Vec<CorpusFile>> {
    let directory = corpus_directory();
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in
        std::fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
    {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "README.md" {
            continue;
        }
        if !path.is_file() {
            bail!(
                "{} is not a file. The corpus is one flat directory of Office-authored packages; a \
                 subdirectory has no meaning here and would sit outside every check",
                path.display()
            );
        }
        let format = super::ingest::format_for_file_name(&name).with_context(|| {
            format!(
                "{name} has no extension this pass covers. The corpus holds .pptx, .docx and .xlsx \
                 only; see tests/office-authored/README.md for the naming convention and the \
                 redistribution rule"
            )
        })?;
        let area = super::ingest::area_for_file_name(&name);
        files.push(CorpusFile {
            path,
            name,
            format,
            area,
        });
    }
    files.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(files)
}
