//! **A resolver reads no document, and paints no pixels** — held here, because rank 3.7 cannot hold
//! either.
//!
//! # What rank 3.7 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate at 3.7, and that buys exactly one thing: nothing at or
//! below it can depend on it. `mjx-layout-pptx` cannot grow a display-list builder, `mjx-scene`
//! cannot learn what a `.pptx` is, and `mjx-view` (3.8) can reach this crate while remaining
//! unable to be reached *by* it.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points up or sideways, so `mjx-scene-pptx -> mjx-pptx` (3.0) and `-> mjx-session` (3.5) are legal
//! downward edges and always will be. So the two properties this crate has to hold are held here:
//!
//! 1. **It never opens a package.** Everything it answers comes from the catalogue the box model
//!    handed it. A resolver that read the document would be a *second* reader beside the one that
//!    laid the page out, and the two would disagree the first time an edit landed between them —
//!    silently, because both would answer.
//! 2. **It never paints.** `mjx-paint` is absent: this crate produces the vocabulary a painter
//!    consumes and knows nothing about how a triangle reaches a screen.
//!
//! `mjx-pptx` is permitted in `[dev-dependencies]` and nowhere else, deliberately: a suite that
//! proves a real deck's fills resolve has to open a real deck, and a test build that opens one links
//! no graphics stack and reads no document at run time in the shipped crate.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number.
const SOURCE_FILE_COUNT: usize = 5;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-dml",
    "mjx-geometry",
    "mjx-layout",
    "mjx-layout-pptx",
    "mjx-ooxml-core",
    "mjx-ooxml-types",
    "mjx-scene",
];

/// The crates this one must never name in `[dependencies]`.
///
/// `mjx-pptx` is the one this gate exists for, and it is the one that is *permitted below*, in
/// `[dev-dependencies]`. The rest would each move a stage of the pipeline into the wrong crate.
const FORBIDDEN_IN_DEPENDENCIES: &[&str] = &[
    "mjx-pptx",
    "mjx-docx",
    "mjx-xlsx",
    "mjx-session",
    "mjx-view",
    "mjx-ooxml",
    "mjx-paint",
    "mjx-render-oracle",
    "mjx-reference-pack",
    "mjx-canvas-harness",
];

/// The crates this one must never name in **either** section.
///
/// A dev-dependency on `mjx-paint` would link a graphics stack into this crate's test build, and a
/// test build that links Vulkan is still a build that links Vulkan.
const FORBIDDEN_EVERYWHERE: &[&str] = &["mjx-paint", "mjx-render-oracle", "mjx-canvas-harness"];

fn manifest() -> String {
    read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Every `.rs` file under `directory`, recursively, sorted.
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

/// The lines of `section`, up to the next section header.
fn section_lines<'a>(text: &'a str, section: &str) -> Vec<&'a str> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == section;
            continue;
        }
        if inside && !trimmed.is_empty() && !trimmed.starts_with('#') {
            lines.push(trimmed);
        }
    }
    lines
}

/// The crate a manifest line names, or `None` for a line that names none.
fn dependency_name(line: &str) -> Option<&str> {
    let name = line.split(['=', '.']).next()?.trim();
    (!name.is_empty()).then_some(name)
}

#[test]
fn the_dependency_list_is_exactly_what_it_should_be() {
    let manifest = manifest();
    let mut named: Vec<&str> = section_lines(&manifest, "[dependencies]")
        .into_iter()
        .filter_map(dependency_name)
        .collect();
    named.sort_unstable();
    named.dedup();

    assert_eq!(
        named, PERMITTED_DEPENDENCIES,
        "`mjx-scene-pptx`'s `[dependencies]` changed. Every entry has a written reason in the \
         manifest and a rank below 3.7; adding one is a decision, and this list is where it is \
         recorded."
    );
}

#[test]
fn a_resolver_never_opens_a_package() {
    let manifest = manifest();
    for forbidden in FORBIDDEN_IN_DEPENDENCIES {
        let named = section_lines(&manifest, "[dependencies]")
            .into_iter()
            .filter_map(dependency_name)
            .any(|name| name == *forbidden);
        assert!(
            !named,
            "`mjx-scene-pptx` names `{forbidden}` in `[dependencies]`. A resolver answers from the \
             catalogue the box model handed it and never reads a document itself: a second reader \
             beside the one that laid the page out would disagree with it the first time an edit \
             landed between them, and both would answer."
        );
    }
}

#[test]
fn a_resolver_never_paints() {
    let manifest = manifest();
    for forbidden in FORBIDDEN_EVERYWHERE {
        for section in ["[dependencies]", "[dev-dependencies]"] {
            let named = section_lines(&manifest, section)
                .into_iter()
                .filter_map(dependency_name)
                .any(|name| name == *forbidden);
            assert!(
                !named,
                "`mjx-scene-pptx` names `{forbidden}` in `{section}`. This crate produces the \
                 vocabulary a painter consumes and knows nothing about how a triangle reaches a \
                 screen — and `mjx-paint` links the platform's graphics API, so even a test build \
                 that named it would link Vulkan."
            );
        }
    }
}

#[test]
fn the_source_names_no_format_crate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files(&root);
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "`src/` holds {} `.rs` files rather than {SOURCE_FILE_COUNT}. Adding a module is fine; \
         update this number so that it stays a deliberate act.",
        files.len()
    );

    for file in &files {
        let code = read(file);
        // Doc comments name `mjx-pptx` deliberately — the crate-level example opens a deck — so
        // only *code* is scanned. A `use` or a path is what would make the edge real.
        let stripped: String = code
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in ["mjx_pptx", "mjx_paint", "mjx_view"] {
            assert!(
                !stripped.contains(forbidden),
                "{} names `{forbidden}` outside a comment. See this file's own documentation for \
                 which of the two seams that breaks.",
                file.display()
            );
        }
    }
}
