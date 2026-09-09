//! **A box model resolves no geometry, never paints, and has never heard of a slide or a
//! worksheet** — held here, because rank 3.6 cannot hold any of it.
//!
//! # What rank 3.6 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate at 3.6, **beside `mjx-layout-pptx` and
//! `mjx-layout-xlsx`**, and that buys exactly two things:
//!
//! 1. Nothing at or below `mjx-session`'s rank can depend on it. `mjx-docx` cannot grow a layout
//!    engine, and — the one that matters most — **`mjx-layout` itself cannot**, so the contract at
//!    1.6 stays a contract rather than becoming a document's shape.
//! 2. The other two box models are at the same rank, so an edge between any two of them is
//!    *sideways* and the layering gate refuses it by name. A document's box model must not know what
//!    a slide is.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points up or sideways, so `mjx-layout-docx -> mjx-geometry` (2.5), `-> mjx-chart` (2.2) and
//! `-> mjx-vml` (2.2) are legal *downward* edges and always will be — and so is
//! `mjx-layout-docx -> mjx-pptx` (3.0), which is worse: a legal edge to another format entirely.
//! That is not a defect in the rank; it is what a rank is. The same sentence is written in
//! `mjx-layout-pptx`'s gate and `mjx-layout-xlsx`'s about 3.6, `mjx-session`'s about 3.5,
//! `mjx-view`'s about 3.8 and `mjx-paint`'s about 5.5.
//!
//! So the three properties this crate actually has to hold are held here and by nothing else:
//!
//! 1. **It never resolves a shape's outline.** `docs/UI_PLATFORM_PLAN.md` §4 L4 puts the
//!    `GeometryProvider` above the box model.
//! 2. **It never paints.** `mjx-paint` and `mjx-scene` are both absent: a box model that built a
//!    display list would have merged two stages the architecture separates on purpose.
//! 3. **It reads one format.** `mjx-pptx` and `mjx-xlsx` are refused in both sections, because a box
//!    model that could open two documents would be two box models sharing a name.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number; a `>=` would pass on a walk that stopped early.
///
/// Twelve at MJXOFF-174, and the split is the crate's own shape: `address`, `checkpoint`,
/// `decoration`, `error`, `flow`, `justify`, `lib`, `measure`, `model`, `paginate`, `style`, `tabs`,
/// `text` — thirteen with `lib.rs`.
const SOURCE_FILE_COUNT: usize = 13;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-docx",
    "mjx-layout",
    "mjx-ooxml-core",
    "mjx-ooxml-types",
    "mjx-text",
    "thiserror",
];

/// The crates a box model must never name — in **either** dependency section.
const FORBIDDEN: &[&str] = &[
    "mjx-geometry",
    "mjx-scene",
    "mjx-scene-pptx",
    "mjx-scene-xlsx",
    // Its own companion, when one exists (see the crate documentation: `mjx-scene-docx` is the
    // ticket that has to follow this one). Cargo would refuse the cycle before this file spoke, and
    // it is named anyway — the day the companion stops depending on the box model, the cycle check
    // goes quiet on the day the seam needs it most.
    "mjx-scene-docx",
    "mjx-paint",
    "mjx-view",
    "mjx-session",
    "mjx-ooxml",
    "mjx-render-oracle",
    "mjx-reference-pack",
    "mjx-canvas-harness",
    "mjx-layout-pptx",
    "mjx-layout-xlsx",
    "mjx-pptx",
    "mjx-xlsx",
    // `mjx-dml` is refused rather than merely absent, and that is a decision: MJXOFF-174's own
    // ticket lists it as a dependency and the tree does not need it. `mjx-docx` resolves every
    // `themeColor` and `asciiTheme` reference against the theme *before* a value reaches this
    // crate, so an `EffectiveColor` is a concrete `RRGGBB` and an `EffectiveFonts` slot is a font
    // name. Declaring the edge would buy an unused import and one more crate a flow engine could
    // accidentally start resolving colours in.
    "mjx-dml",
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
    "mjx_xlsx",
    "mjx_dml",
    "GeometryProvider",
    "ResourceResolver",
    "DisplayList",
    "Painter",
];

/// Whether `code` names `needle` as a whole identifier.
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

fn declared(manifest: &str, section: &str) -> Vec<String> {
    section_lines(manifest, section)
        .into_iter()
        .filter_map(|line| line.split(['=', '.']).next())
        .map(|name| name.trim().to_owned())
        .collect::<Vec<_>>()
        .into_iter()
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
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let code = read(&file);
        for line in code.lines() {
            let trimmed = line.trim_start();
            // The module documentation *names* these crates deliberately, in order to say what this
            // crate does not do. A comment is prose and a use is code.
            if trimmed.starts_with("//") {
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

/// The reason this crate shares rank 3.6 with the other two rather than sitting above or below them.
/// The layering gate refuses the sideways edge by name; this asserts the other half — that neither
/// of them has grown an edge to this one either, which the gate would also refuse but which nobody
/// would notice until it was written.
#[test]
fn the_three_box_models_do_not_know_about_each_other() {
    for other in ["mjx-layout-pptx", "mjx-layout-xlsx"] {
        let theirs =
            read(&Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../{other}/Cargo.toml")));
        for section in ["[dependencies]", "[dev-dependencies]"] {
            for name in declared(&theirs, section) {
                assert_ne!(
                    name, "mjx-layout-docx",
                    "{other}'s {section} names Word's box model"
                );
            }
        }
    }
}
