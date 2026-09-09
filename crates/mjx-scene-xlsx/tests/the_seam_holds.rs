//! **A resolver reads no document, and paints no pixels** — held here, because rank 3.7 cannot hold
//! either.
//!
//! # What rank 3.7 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate at 3.7, and that buys exactly two things. First,
//! nothing at or below it can depend on it: `mjx-layout-xlsx` cannot grow a display-list builder,
//! `mjx-scene` cannot learn what a `.xlsx` is, and `mjx-view` (3.8) can reach this crate while
//! remaining unable to be reached *by* it. Second — and this is the half that is specific to there
//! being two companions — it shares the rank with `mjx-scene-pptx`, so an edge between the two is
//! **sideways** and the layering gate refuses it by name. A spreadsheet's resolver has no business
//! knowing what a slide is, and vice versa.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points up or sideways, so `mjx-scene-xlsx -> mjx-xlsx` (3.0), `-> mjx-session` (3.5) and
//! `-> mjx-geometry` (2.5) are legal downward edges and always will be. So the two properties this
//! crate has to hold are held here:
//!
//! 1. **It never opens a package.** Everything it answers comes from the catalogue the box model
//!    handed it, plus the palette the caller built. A resolver that read the document would be a
//!    *second* reader beside the one that laid the band out, and the two would disagree the first
//!    time an edit landed between them — silently, because both would answer.
//! 2. **It never paints.** `mjx-paint` is absent: this crate produces the vocabulary a painter
//!    consumes and knows nothing about how a triangle reaches a screen.
//!
//! `mjx-xlsx` is permitted in `[dev-dependencies]` and nowhere else, deliberately: a suite that
//! proves a real workbook's fills resolve has to open a real workbook, and a test build that opens
//! one links no graphics stack and reads no document at run time in the shipped crate.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number.
const SOURCE_FILE_COUNT: usize = 5;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-dml",
    "mjx-layout",
    "mjx-layout-xlsx",
    "mjx-ooxml-core",
    "mjx-ooxml-types",
    "mjx-scene",
    "mjx-sml",
];

/// The crates this one must never name in `[dependencies]`.
///
/// `mjx-xlsx` is the one this gate exists for, and it is the one that is *permitted below*, in
/// `[dev-dependencies]`. `mjx-layout-pptx` and `mjx-scene-pptx` are here for a different reason: the
/// second is refused by the layering gate as a sideways edge, and stating both here says that the
/// refusal is intended rather than incidental. The rest would each move a stage of the pipeline into
/// the wrong crate.
const FORBIDDEN_IN_DEPENDENCIES: &[&str] = &[
    "mjx-xlsx",
    "mjx-pptx",
    "mjx-docx",
    "mjx-layout-pptx",
    "mjx-scene-pptx",
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
        "`mjx-scene-xlsx`'s `[dependencies]` changed. Every entry has a written reason in the \
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
            "`mjx-scene-xlsx` names `{forbidden}` in `[dependencies]`. A resolver answers from the \
             catalogue the box model handed it and never reads a document itself: a second reader \
             beside the one that laid the band out would disagree with it the first time an edit \
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
                "`mjx-scene-xlsx` names `{forbidden}` in `{section}`. This crate produces the \
                 vocabulary a painter consumes and knows nothing about how a triangle reaches a \
                 screen — and `mjx-paint` links the platform's graphics API, so even a test build \
                 that named it would link Vulkan."
            );
        }
    }
}

/// The sideways edge, asserted in the direction the layering gate cannot see from here.
///
/// `xtask/tests/layering.rs` refuses `mjx-scene-xlsx -> mjx-scene-pptx` because the two share rank
/// 3.7, and that refusal is real. It is repeated here because the reason is a *design* one rather
/// than an arithmetic one: the day either companion is renumbered — say a Word companion arrives and
/// somebody decides the three should ladder — the gate would go quiet and the property would not.
#[test]
fn the_two_companions_never_name_each_other() {
    let manifest = manifest();
    for section in ["[dependencies]", "[dev-dependencies]"] {
        let named = section_lines(&manifest, section)
            .into_iter()
            .filter_map(dependency_name)
            .any(|name| name == "mjx-scene-pptx" || name == "mjx-layout-pptx");
        assert!(
            !named,
            "`mjx-scene-xlsx` names PowerPoint's box model or its companion in `{section}`. The \
             two formats meet at `mjx-scene`, which is the whole point of there being a display \
             list: a spreadsheet's resolver that could read a slide's would have made the display \
             list a place where two formats negotiate rather than a vocabulary both answer in."
        );
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
        // Doc comments name `mjx-xlsx` deliberately — the crate-level example opens a workbook — so
        // only *code* is scanned. A `use` or a path is what would make the edge real.
        let stripped: String = code
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in ["mjx_xlsx", "mjx_paint", "mjx_view", "mjx_scene_pptx"] {
            assert!(
                !stripped.contains(forbidden),
                "{} names `{forbidden}` outside a comment. See this file's own documentation for \
                 which of the seams that breaks.",
                file.display()
            );
        }
    }
}
