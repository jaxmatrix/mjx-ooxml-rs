//! **A box model resolves no geometry, never paints, and has never heard of a slide** — held here,
//! because rank 3.6 cannot hold any of it.
//!
//! # What rank 3.6 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate at 3.6, **beside `mjx-layout-pptx`**, and that buys
//! exactly two things:
//!
//! 1. Nothing at or below `mjx-session`'s rank can depend on it. `mjx-sml` cannot grow a layout
//!    engine, `mjx-xlsx` cannot reach a `FragmentTree`, and — the one that matters most —
//!    **`mjx-layout` itself cannot**, so the contract at 1.6 stays a contract rather than becoming a
//!    grid's shape.
//! 2. `mjx-layout-pptx` is at the same rank, so an edge between the two box models is *sideways* and
//!    the layering gate refuses it by name. A spreadsheet's box model must not know what a slide is.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points up or sideways, so `mjx-layout-xlsx → mjx-geometry` (2.5), `→ mjx-chart` (2.2) and
//! `→ mjx-vml` (2.2) are legal *downward* edges and always will be — and so is
//! `mjx-layout-xlsx → mjx-pptx` (3.0), which is worse: a legal edge to another format entirely. That
//! is not a defect in the rank; it is what a rank is. The same sentence is written in
//! `mjx-layout-pptx`'s gate about 3.6, `mjx-session`'s about 3.5, `mjx-view`'s about 3.8 and
//! `mjx-paint`'s about 5.5.
//!
//! So the three properties this crate actually has to hold are held here and by nothing else:
//!
//! 1. **It never resolves a shape's outline.** `docs/UI_PLATFORM_PLAN.md` §4 L4 puts the
//!    `GeometryProvider` above the box model.
//! 2. **It never paints.** `mjx-paint` and `mjx-scene` are both absent: a box model that built a
//!    display list would have merged two stages the architecture separates on purpose.
//! 3. **It reads one format.** `mjx-pptx` and `mjx-docx` are refused in both sections, because a box
//!    model that could open two documents would be two box models sharing a name.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number; a `>=` would pass on a walk that stopped early.
///
/// Thirteen at MJXOFF-171. Nineteen at MJXOFF-172, which added `numfmt/` — the `numFmt` evaluator,
/// six files of its own: the parser, the numeric renderer, the date arithmetic, `General` and the
/// fifteen-digit clamp, the two caches, and the module that joins them. It is a **sub-project**
/// rather than a feature, which is why it is a directory and why the count moves by six at once.
const SOURCE_FILE_COUNT: usize = 19;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-layout",
    "mjx-ooxml-core",
    "mjx-ooxml-types",
    "mjx-sml",
    "mjx-text",
    "mjx-xlsx",
    "thiserror",
];

/// The crates a box model must never name — in **either** dependency section.
const FORBIDDEN: &[&str] = &[
    "mjx-geometry",
    "mjx-scene",
    "mjx-scene-pptx",
    // Its own companion (MJXOFF-244). Cargo would refuse the cycle before this file spoke, and it
    // is named anyway: the day the companion stops depending on the box model — a resolver that
    // answered from a table rather than from a catalogue would do exactly that — the cycle check
    // goes quiet on the day the seam needs it most.
    "mjx-scene-xlsx",
    "mjx-paint",
    "mjx-view",
    "mjx-session",
    "mjx-ooxml",
    "mjx-render-oracle",
    "mjx-reference-pack",
    "mjx-canvas-harness",
    "mjx-layout-pptx",
    "mjx-pptx",
    "mjx-docx",
];

/// Identifiers whose appearance anywhere in `src/` would mean this crate had started doing a job
/// that belongs above it.
const FORBIDDEN_IDENTIFIERS: &[&str] = &[
    "mjx_geometry",
    "mjx_scene",
    "mjx_paint",
    "mjx_view",
    "mjx_session",
    "mjx_pptx",
    "mjx_docx",
    "GeometryProvider",
    "ResourceResolver",
    "DisplayList",
    "Painter",
];

/// Whether `code` names `needle` as a whole identifier.
///
/// A bare `contains` would report `mjx_ooxml_core` as `mjx_ooxml`, so the character on each side of
/// the match has to end the identifier.
fn names(code: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(at) = code[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let before_ends = start == 0
            || !code[..start]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_alphanumeric() || character == '_');
        let after_ends = code[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        if before_ends && after_ends {
            return true;
        }
        from = end;
    }
    false
}

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
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
fn section_lines<'a>(manifest: &'a str, section: &str) -> Vec<&'a str> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
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

/// The crate each dependency line names.
fn declared(manifest: &str, section: &str) -> Vec<String> {
    section_lines(manifest, section)
        .into_iter()
        .filter_map(|line| line.split(['=', '.']).next())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn the_manifest_declares_exactly_the_permitted_dependencies() {
    let manifest = read(&manifest_path());
    let mut found = declared(&manifest, "[dependencies]");
    found.sort();
    found.dedup();
    let mut expected: Vec<String> = PERMITTED_DEPENDENCIES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();
    assert_eq!(
        found, expected,
        "a box model's dependency list is part of its contract; changing it is a decision, not an \
         edit"
    );
}

#[test]
fn no_forbidden_crate_appears_in_either_dependency_section() {
    // **Both sections**, because a dev-dependency on `mjx-geometry` would let a test resolve a
    // preset path and call it proof that the box model does — and a dev-dependency on `mjx-paint`
    // would make a test build link Vulkan, which is still a build that links Vulkan.
    let manifest = read(&manifest_path());
    for section in ["[dependencies]", "[dev-dependencies]"] {
        for name in declared(&manifest, section) {
            assert!(
                !FORBIDDEN.contains(&name.as_str()),
                "{section} names `{name}`, which a box model may never reach"
            );
        }
    }
}

#[test]
fn no_forbidden_identifier_appears_in_the_source() {
    // The manifest gate is the strong one; this catches the case where a crate arrives transitively
    // — through a re-export, say — and is used without ever being declared.
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let code = read(&file);
        for line in code.lines() {
            let trimmed = line.trim_start();
            // The module documentation *names* these crates deliberately, in order to say what this
            // crate does not do. A comment is prose and a use is code.
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            for identifier in FORBIDDEN_IDENTIFIERS {
                if names(line, identifier) {
                    offences.push(format!("{}: {}", file.display(), line.trim()));
                }
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn the_source_tree_is_the_size_it_is_declared_to_be() {
    let files = rust_files(&source_root());
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "adding a module to a box model is a decision; update this number with the reason: {files:?}"
    );
}

#[test]
fn the_crate_forbids_unsafe_code() {
    let lib = read(&source_root().join("lib.rs"));
    assert!(
        lib.contains("#![forbid(unsafe_code)]"),
        "a box model reads untrusted files and has no reason to reach for `unsafe`"
    );
}

#[test]
fn the_two_box_models_do_not_know_about_each_other() {
    // The reason this crate shares rank 3.6 with `mjx-layout-pptx` rather than sitting above it. The
    // layering gate refuses the sideways edge by name; this asserts the other half — that
    // PowerPoint's box model has not grown an edge to Excel's either, which the gate would also
    // refuse but which nobody would notice until it was written.
    let theirs = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-layout-pptx/Cargo.toml"));
    for section in ["[dependencies]", "[dev-dependencies]"] {
        for name in declared(&theirs, section) {
            assert_ne!(
                name, "mjx-layout-xlsx",
                "PowerPoint's box model {section} names Excel's"
            );
        }
    }
}
