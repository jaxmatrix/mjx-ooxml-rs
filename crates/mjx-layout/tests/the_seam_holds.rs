//! Two source gates over this crate: **nothing here has heard of OOXML**, and **nothing here
//! panics**.
//!
//! # Why a source gate as well as the layering test
//!
//! `xtask/tests/layering.rs` refuses a dependency edge from `mjx-layout` to any format crate, and
//! that is the primary defence — without the edge, no OOXML type can be named. This suite is a
//! second one at a different level, and it fails for a different reason: it names the identifiers,
//! so the failure message says *which* file reached for OOXML rather than only that the graph is
//! wrong. A gate that catches a mistake one commit earlier, in the crate that made it, is worth its
//! twenty lines.
//!
//! It scans **code**, not prose: this crate's documentation names `mjx-dml` repeatedly, because
//! explaining why `Emu` was lifted down out of it rather than duplicated is exactly the reasoning
//! that stops the next person duplicating it. A comment cannot contain a `use`, a path or a type, so
//! skipping comment lines loses nothing.
//!
//! # The walk is recursive and counts what it read
//!
//! MJXOFF-155 §8 lists *"the non-recursive walk — a source gate reading one directory level"* as a
//! recurring defect, and *"a gate phrased 'X is covered and green' is green precisely when X is
//! skipped"*. So both gates below walk the tree recursively and assert an exact file count: a scan
//! that found nothing to scan fails, and adding a module without noticing this file fails too.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so that adding a module is a
/// deliberate act that touches this number — a `>=` would pass on a scan that quietly stopped.
const SOURCE_FILE_COUNT: usize = 9;

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `directory`, recursively, sorted so a failure names them in a stable
/// order.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current)
            .unwrap_or_else(|error| panic!("reading {}: {error}", current.display()));
        for entry in entries {
            let entry = entry.expect("a directory entry");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Read a file, or fail naming it.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

#[test]
fn no_ooxml_crate_is_named_anywhere_in_this_crates_source() {
    // Every crate above `mjx-text` in the workspace, plus the packaging tier. Naming one here would
    // mean an OOXML type had reached the seam — which is what the whole client platform is
    // organised to prevent.
    const FORBIDDEN: &[&str] = &[
        "mjx_dml",
        "mjx-dml",
        "mjx_sml",
        "mjx-sml",
        "mjx_chart",
        "mjx-chart",
        "mjx_omml",
        "mjx-omml",
        "mjx_vml",
        "mjx-vml",
        "mjx_pptx",
        "mjx-pptx",
        "mjx_docx",
        "mjx-docx",
        "mjx_xlsx",
        "mjx-xlsx",
        "mjx_ooxml_types",
        "mjx-ooxml-types",
        "mjx_opc",
        "mjx-opc",
        "mjx_mce",
        "mjx-mce",
        "mjx_xml",
        "mjx-xml",
    ];

    let files = rust_files(&source_root());
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "the walk found {} source file(s); it is recursive and this count is exact, so either a \
         module was added or the walk stopped early: {files:?}",
        files.len()
    );

    for path in &files {
        let text = read(path);
        for (number, line) in text.lines().enumerate() {
            // Prose may name them and does: this crate's documentation explains at length *why*
            // `Emu` was lifted out of `mjx-dml` rather than duplicated, and why no format crate is
            // reachable. Code may not. Skipping comment lines is what makes the difference, and it
            // is safe because a `use`, a path or a type name is never inside one.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for forbidden in FORBIDDEN {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} names `{forbidden}`. Nothing above the `FragmentTree` seam may know what \
                     a `.docx` is, and nothing below it may reach upward; if this crate genuinely \
                     needs something from there, the thing moves down rather than the rule bending.",
                    path.display(),
                    number + 1
                );
            }
        }
    }

    // And the manifest declares exactly the three dependencies the rank table allows.
    let manifest = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
    let mut in_dependencies = false;
    let mut declared: Vec<&str> = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // `[package]` inherits `version`, `edition` and the rest from the workspace with the
            // same spelling, so the section has to be tracked rather than the whole file scanned.
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once(".workspace = true") {
            declared.push(name.trim());
        }
    }
    assert_eq!(
        declared,
        vec!["mjx-ooxml-core", "mjx-text", "thiserror"],
        "this crate depends on `mjx-ooxml-core` (for `Emu`), `mjx-text` (to measure) and \
         `thiserror`, and on nothing else"
    );
}

#[test]
fn nothing_on_a_layout_path_panics() {
    // A pathological document produces a bad-looking page, never a crash — so no `unwrap`, no
    // `expect`, no `panic!`, no `todo!` and no `unreachable!` anywhere in the library.
    //
    // Indexing is not caught by a text scan, and is not pretended to be: what holds it is that every
    // slice access in this crate is a `get`, which a reader can check and which `clippy::indexing_slicing`
    // would enforce if the workspace turned it on. This gate catches the four constructs that *are*
    // textual, which is the four that a moment's inattention introduces.
    const FORBIDDEN: &[&str] = &[
        ".unwrap()",
        ".expect(",
        "panic!(",
        "todo!(",
        "unimplemented!(",
        "unreachable!(",
    ];

    let files = rust_files(&source_root());
    assert_eq!(files.len(), SOURCE_FILE_COUNT);

    let mut scanned_lines = 0_usize;
    for path in &files {
        let text = read(path);
        for (number, line) in text.lines().enumerate() {
            scanned_lines += 1;
            let code = line.trim_start();
            // Documentation may name them; code may not.
            if code.starts_with("//") {
                continue;
            }
            for forbidden in FORBIDDEN {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} uses `{forbidden}`: {}. Inputs to a box model are untrusted, and a \
                     layout that can panic turns a malformed document into a crashed application.",
                    path.display(),
                    number + 1,
                    code
                );
            }
        }
    }
    assert!(
        scanned_lines > 2_000,
        "only {scanned_lines} lines were scanned, which cannot be this crate"
    );
}
