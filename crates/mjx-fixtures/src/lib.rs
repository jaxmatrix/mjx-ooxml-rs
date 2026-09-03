//! The committed OOXML fixture corpus, derived from the directory rather than written down.
//!
//! **Test-only, and deliberately dependency-free.** Every byte-identity suite and every schema gate
//! in this workspace reads its corpus from here, including `mjx-opc`'s — which sits *below*
//! `mjx-schema-gate` and could not dev-depend on it without an upward edge. A crate with no
//! dependencies at all can be reached from anywhere.
//!
//! `tests/fixtures/` at the workspace root is shared by every crate. Before this crate, four
//! separate suites each carried a hand-maintained `const FIXTURES` list, and a fixture added in a
//! later phase and omitted from one of them sat silently outside the byte-identity contract — the
//! project's core promise. Every corpus is now a `read_dir` sweep over this module.
//!
//! A file whose extension is on neither list below fails
//! [`assert_every_fixture_has_a_known_kind`], naming it. That is the point: a `.dotx` or a `.xlsm`
//! dropped in here joins the corpora by being *classified*, never by being ignored.

use std::path::PathBuf;

/// The extensions that name an OOXML package. A fixture with one of these joins **every** corpus:
/// the three byte-identity suites and the schema gate.
pub const PACKAGE_EXTENSIONS: &[&str] = &["pptx", "docx", "xlsx"];

/// Fixtures that are deliberately not OOXML packages, each with the reason it is here.
///
/// Empty today. An entry is a claim that a file belongs in `tests/fixtures/` while belonging to no
/// package corpus, and it has to be written down for the sweep to accept it.
pub const NON_PACKAGE_FIXTURES: &[(&str, &str)] = &[];

/// The workspace root, from this crate's manifest directory.
#[must_use]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// The shared fixtures directory.
#[must_use]
pub fn fixtures_dir() -> PathBuf {
    workspace_root().join("tests/fixtures")
}

/// Reads one fixture.
///
/// # Panics
/// If the fixture cannot be read.
#[must_use]
pub fn fixture(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// Every file in the fixtures directory, sorted.
///
/// # Panics
/// If the directory cannot be read.
#[must_use]
pub fn all_fixture_files() -> Vec<String> {
    let dir = fixtures_dir();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("a fixture directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Every committed OOXML package fixture, of every extension, sorted.
///
/// This is the corpus. `cargo test` cases iterate it; nothing lists it.
#[must_use]
pub fn package_fixtures() -> Vec<String> {
    all_fixture_files()
        .into_iter()
        .filter(|name| {
            PACKAGE_EXTENSIONS
                .iter()
                .any(|extension| name.ends_with(&format!(".{extension}")))
        })
        .collect()
}

/// Every committed package fixture with the given extension (`"docx"`, `"xlsx"`, `"pptx"`), sorted.
///
/// # Panics
/// If `extension` is not one of [`PACKAGE_EXTENSIONS`] — a typo there would silently return an
/// empty corpus, which is the class of defect this crate exists to close.
#[must_use]
pub fn package_fixtures_with_extension(extension: &str) -> Vec<String> {
    assert!(
        PACKAGE_EXTENSIONS.contains(&extension),
        "{extension} is not a package extension; add it to PACKAGE_EXTENSIONS"
    );
    let suffix = format!(".{extension}");
    package_fixtures()
        .into_iter()
        .filter(|name| name.ends_with(&suffix))
        .collect()
}

/// Asserts that every file in the fixtures directory is either an OOXML package or a declared
/// non-package fixture.
///
/// # Panics
/// Naming any file that is neither.
pub fn assert_every_fixture_has_a_known_kind() {
    let packages = package_fixtures();
    let unclassified: Vec<String> = all_fixture_files()
        .into_iter()
        .filter(|name| {
            !packages.contains(name)
                && !NON_PACKAGE_FIXTURES
                    .iter()
                    .any(|(declared, _)| declared == name)
        })
        .collect();
    assert!(
        unclassified.is_empty(),
        "these fixtures are on no list: {unclassified:?}. Add the extension to PACKAGE_EXTENSIONS \
         so the file joins every corpus, or add the file to NON_PACKAGE_FIXTURES with the reason \
         it belongs to none"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corpus_is_read_from_disk_and_is_not_empty() {
        assert_every_fixture_has_a_known_kind();
        let packages = package_fixtures();
        assert!(
            packages.len() >= 15,
            "the committed corpus shrank to {} fixtures — it held fifteen when this crate was \
             written, and a corpus that quietly empties is a gate that quietly passes",
            packages.len()
        );
        for extension in PACKAGE_EXTENSIONS {
            assert!(
                !package_fixtures_with_extension(extension).is_empty(),
                "no .{extension} fixture — the corpus for that format is empty, so every case \
                 iterating it passes vacuously"
            );
        }
    }
}
