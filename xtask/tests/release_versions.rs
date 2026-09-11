//! **A release states the version once, and every file in the tree that repeats it agrees**
//! (MJXOFF-286, MJXOFF-290).
//!
//! # The hole this closes
//!
//! `bindings/mjx-wasm/npm/package.json` carries the version a second time, because npm has no way
//! to read Cargo's. `bindings/mjx-wasm/build-npm.sh` refuses to build when the two disagree — and
//! **a release never runs it**. So 0.0.167 bumped `Cargo.toml` and not `package.json`, and the
//! npm package and every artefact wasm-pack writes beside it could not be rebuilt from `main` at
//! all until the next unit tripped over the refusal two versions later. The symptom was a build script's
//! complaint on somebody's laptop, which is the slowest possible way to learn it.
//!
//! A release commit is written by hand. The invariant it can break therefore has to fail where a
//! release *is* checked, which is `cargo test --workspace`.
//!
//! # What is checked
//!
//! * [`the_npm_package_states_the_workspace_version`] — the one equality the build script already
//!   demands, now demanded before the build.
//! * [`the_newest_changelog_entry_is_the_workspace_version`] — the other file a release edits by
//!   hand. A bump with no entry, or an entry with no bump, is the same mistake in the other
//!   direction.
//! * [`every_file_that_states_the_version_is_one_this_file_knows_about`] — the sweep, and the
//!   reason the two checks above cannot quietly become incomplete. The set of files carrying the
//!   version is **derived** from the tree rather than listed, so a fifth one added tomorrow fails
//!   here and forces a decision instead of being bumped by whoever notices first.
//! * [`every_carrier_row_still_names_a_file_that_states_the_version`] — and the ledger cannot rot
//!   the other way either.
//!
//! # What is deliberately *not* checked, and why
//!
//! * **`bindings/mjx-python/pyproject.toml`.** It declares `dynamic = ["version"]` and maturin
//!   takes the value from the crate, which takes it from `version.workspace = true`. There is no
//!   second statement of the version to disagree with, which is the shape `package.json` would
//!   have if npm allowed it.
//! * **Every member manifest.** All twenty-one say `version.workspace = true`; none states a
//!   number. `xtask/tests/layering.rs` is what holds the membership itself.
//! * **`tests/fixtures/` and every non-UTF-8 file.** The corpus is real Office packages, whose
//!   bytes are not ours to reason about. Nothing else is excluded: the sweep's corpus is
//!   `xtask::repository_files::WorkingTree`, so build output, the git-ignored schema tree and the
//!   Python virtualenv are absent by being **ignored** rather than by being listed. Stated here
//!   rather than left silent, because an unstated exclusion is how a sweep becomes vacuous without
//!   anyone deciding it should.
//! * **Nothing at all on the grounds of being uncommitted** (MJXOFF-290). The corpus was the Git
//!   index until then, which made the one file this sweep most needs to see — a *new* fifth carrier,
//!   written minutes ago — the one file it could not. A hazard a release commit introduces has to be
//!   reportable before that commit exists.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use xtask::repository_files::WorkingTree;

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// One file in this tree that states the workspace version, and why it does.
struct Carrier {
    /// Its path from the repository root, in `/` form.
    path: &'static str,
    /// Why this file repeats a number `Cargo.toml` already states.
    reason: &'static str,
}

/// The four files that state the version, and nothing else may.
const CARRIERS: [Carrier; 4] = [
    Carrier {
        path: "Cargo.toml",
        reason: "`[workspace.package] version` — the one source every member inherits with \
                 `version.workspace = true`, and what every check in this file compares against",
    },
    Carrier {
        path: "Cargo.lock",
        reason: "Cargo writes each workspace member's resolved version here; a release that bumps \
                 the manifest and commits a stale lock leaves the two disagreeing",
    },
    Carrier {
        path: "CHANGELOG.md",
        reason: "the newest numbered heading is the release being made, checked by \
                 `the_newest_changelog_entry_is_the_workspace_version`",
    },
    Carrier {
        path: "bindings/mjx-wasm/npm/package.json",
        reason: "npm cannot read Cargo's version, so the package states its own; this is the file \
                 0.0.167 forgot, and `bindings/mjx-wasm/build-npm.sh` refuses to build while it \
                 disagrees",
    },
];

/// The `[workspace.package]` version, read from `Cargo.toml`.
fn workspace_version(root: &Path) -> String {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest");
    let section = text
        .split_once("[workspace.package]")
        .expect("`Cargo.toml` declares `[workspace.package]`")
        .1;
    for line in section.lines() {
        if line.starts_with('[') {
            break;
        }
        if let Some(rest) = line.strip_prefix("version = \"") {
            return rest.split_once('"').expect("a quoted version").0.to_owned();
        }
    }
    panic!("`[workspace.package]` states no `version`");
}

/// `bindings/mjx-wasm/npm/package.json` states the workspace version.
///
/// The same equality `bindings/mjx-wasm/build-npm.sh` refuses to build without — asked here, where
/// a release meets it, rather than only there, where a release never goes.
#[test]
fn the_npm_package_states_the_workspace_version() {
    let root = repository_root();
    let expected = workspace_version(&root);
    let manifest = root.join("bindings/mjx-wasm/npm/package.json");
    let text = std::fs::read_to_string(&manifest).expect("the npm manifest");
    let stated = text
        .split_once("\"version\":")
        .expect("`npm/package.json` states a `version`")
        .1
        .split_once('"')
        .expect("a quoted version")
        .1
        .split_once('"')
        .expect("a closed quoted version")
        .0
        .to_owned();
    assert_eq!(
        stated, expected,
        "bindings/mjx-wasm/npm/package.json says {stated} but the workspace is at {expected} — \
         the npm package cannot be rebuilt from this commit until the two agree, and \
         `bindings/mjx-wasm/build-npm.sh` will refuse rather than publish a package nobody can \
         trace back here"
    );
    println!("the npm package and the workspace both state {expected}");
}

/// `CHANGELOG.md`'s newest numbered entry is the version being released.
///
/// The `## [Unreleased — …]` heading at the top names the *next milestone* rather than a release,
/// so the newest numbered heading is the first one under it.
#[test]
fn the_newest_changelog_entry_is_the_workspace_version() {
    let root = repository_root();
    let expected = workspace_version(&root);
    let text = std::fs::read_to_string(root.join("CHANGELOG.md")).expect("the changelog");
    let newest = text
        .lines()
        .filter_map(|line| line.strip_prefix("## ["))
        .map(|rest| rest.split_once(']').map_or(rest, |(name, _)| name))
        .find(|name| name.starts_with(|character: char| character.is_ascii_digit()))
        .expect("`CHANGELOG.md` holds at least one numbered entry");
    assert_eq!(
        newest, expected,
        "the newest numbered `CHANGELOG.md` entry is {newest} and the workspace is at {expected} \
         — a release bumps both, in one commit, or the history stops saying what shipped"
    );
    println!("the newest changelog entry and the workspace both state {expected}");
}

/// The one tree this sweep will not read, and which the module docs gives a reason for.
const UNSWEPT: &str = "tests/fixtures/";

/// Every file in this tree stating the workspace version verbatim, derived from the tree.
///
/// The corpus comes from Git rather than from a directory walk, because Git is the only thing that
/// knows which files are ours: a walk would have to read `bindings/mjx-python/.venv` — ninety-four
/// megabytes of git-ignored virtualenv — to answer a question about four files.
///
/// It is the **working tree** and not the index (MJXOFF-290). A fifth carrier is a release hazard
/// from the moment somebody writes it, and until this changed the sweep could not see the file
/// until the commit that shipped the hazard already existed.
///
/// # Panics
/// If `git` cannot be run, or reports a failure.
fn carriers(root: &Path) -> BTreeSet<String> {
    let version = workspace_version(root);
    let tree = WorkingTree::read(root);
    println!("{}", tree.census());
    tree.paths()
        .iter()
        .map(String::as_str)
        .filter(|path| !path.starts_with(UNSWEPT))
        .filter(|path| {
            std::fs::read(root.join(path))
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .is_some_and(|text| text.contains(&version))
        })
        .map(str::to_owned)
        .collect()
}

/// **The sweep**: no file in this tree states the version except the four that are supposed to.
///
/// This is what keeps the two equalities above from going quietly incomplete. The day a fifth file
/// repeats the version — a README installation snippet, a second package manifest, a generated
/// header — it fails here, and somebody decides whether it belongs on [`CARRIERS`] with a check
/// beside it or should be reading the version instead of restating it.
#[test]
fn every_file_that_states_the_version_is_one_this_file_knows_about() {
    let root = repository_root();
    let known: BTreeSet<String> = CARRIERS.iter().map(|row| row.path.to_owned()).collect();
    let found = carriers(&root);
    let unknown: Vec<&String> = found.difference(&known).collect();
    assert!(
        unknown.is_empty(),
        "a file in this tree states the workspace version {} and is on no `CARRIERS` row, so \
         nothing holds it to the release: {unknown:?}",
        workspace_version(&root)
    );
    assert!(
        found.len() >= CARRIERS.len(),
        "only {} file(s) found stating the version — the sweep has stopped matching, and the claim \
         above would be made over almost nothing",
        found.len()
    );
    println!(
        "version carriers: {} file(s) state {}, all four of them known",
        found.len(),
        workspace_version(&root)
    );
}

/// Every [`CARRIERS`] row still names a file that states the version.
///
/// A row kept after its file stopped carrying the number is a standing claim nobody checks, and the
/// reason written beside it would be describing something that is no longer there.
#[test]
fn every_carrier_row_still_names_a_file_that_states_the_version() {
    let root = repository_root();
    let found = carriers(&root);
    let stale: Vec<String> = CARRIERS
        .iter()
        .filter(|row| !found.contains(row.path))
        .map(|row| format!("{} — {}", row.path, row.reason))
        .collect();
    assert!(
        stale.is_empty(),
        "a `CARRIERS` row names a file that no longer states the workspace version:\n  {}",
        stale.join("\n  ")
    );
}
