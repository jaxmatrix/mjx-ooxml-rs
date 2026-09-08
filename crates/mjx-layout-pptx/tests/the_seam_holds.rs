//! **A box model resolves no geometry paths** — held here, because rank 3.6 cannot hold it.
//!
//! # What rank 3.6 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate at 3.6, and that buys exactly one thing: nothing at or
//! below `mjx-session`'s rank can depend on it. `mjx-pptx` cannot grow a layout engine, `mjx-dml`
//! cannot reach a `FragmentTree`, and — the one that matters most — **`mjx-layout` itself cannot**,
//! so the contract at 1.6 stays a contract rather than becoming PowerPoint's own shape.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points up or sideways, so `mjx-layout-pptx → mjx-geometry` (2.5), `→ mjx-chart` (2.2) and
//! `→ mjx-vml` (2.2) are legal *downward* edges and always will be. That is not a defect in the
//! rank; it is what a rank is. The same sentence is written in `mjx-session`'s gate about 3.5,
//! `mjx-view`'s about 3.8 and `mjx-paint`'s about 5.5.
//!
//! So the two properties this crate actually has to hold are held here and by nothing else:
//!
//! 1. **It never resolves a shape's outline.** `docs/UI_PLATFORM_PLAN.md` §4 L4 puts the
//!    `GeometryProvider` above the box model, and a box model that reached `mjx-geometry` would
//!    make the provider unswappable and would put a preset path table inside a `FragmentTree`. The
//!    manifest gate and the source scan below refuse the edge and the identifier.
//! 2. **It never paints.** `mjx-paint` and `mjx-scene` are both absent: a box model that built a
//!    display list would have merged two stages the architecture separates on purpose.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number; a `>=` would pass on a walk that stopped early.
const SOURCE_FILE_COUNT: usize = 9;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-dml",
    "mjx-layout",
    "mjx-ooxml-core",
    "mjx-pptx",
    "mjx-text",
    "thiserror",
];

/// The crates a box model must never name — in **either** dependency section.
///
/// `mjx-geometry` is the one this gate exists for. The others are here because each would move a
/// stage of the pipeline into the wrong crate: a display list, a painter, a viewport, a facade.
const FORBIDDEN: &[&str] = &[
    "mjx-geometry",
    "mjx-scene",
    "mjx-paint",
    "mjx-view",
    "mjx-session",
    "mjx-ooxml",
    "mjx-render-oracle",
    "mjx-reference-pack",
    "mjx-canvas-harness",
    "mjx-docx",
    "mjx-xlsx",
];

/// Whether `code` names the crate `needle` as a whole identifier.
///
/// A bare `contains` would report `mjx_ooxml_core` as `mjx_ooxml`, so the character on each side of
/// the match has to end the identifier.
fn names_crate(code: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(at) = code[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let before_ends_identifier = start == 0
            || !code[..start]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_alphanumeric() || character == '_');
        let after_ends_identifier = code[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        if before_ends_identifier && after_ends_identifier {
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
    manifest
        .lines()
        .skip_while(|line| line.trim() != section)
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// The crate a dependency line names.
fn named_crate(line: &str) -> &str {
    let name = line.split('=').next().unwrap_or(line).trim();
    name.strip_suffix(".workspace").unwrap_or(name).trim()
}

#[test]
fn the_manifest_names_no_geometry_provider_and_no_painter() {
    let manifest = read(&manifest_path());
    let mut named: Vec<&str> = section_lines(&manifest, "[dependencies]")
        .iter()
        .copied()
        .map(named_crate)
        .collect();
    named.sort_unstable();
    named.dedup();
    let mut permitted = PERMITTED_DEPENDENCIES.to_vec();
    permitted.sort_unstable();
    assert_eq!(
        named, permitted,
        "the dependency set changed. A box model's edges are the whole of what keeps the geometry \
         provider swappable, so widening them is a deliberate act and this list moves with it.",
    );

    // **Both sections.** A dev-dependency on `mjx-geometry` would let a test resolve a preset path
    // and call it proof that the box model does, which is exactly the confusion the rank cannot
    // prevent and this gate must.
    let development: Vec<&str> = section_lines(&manifest, "[dev-dependencies]")
        .iter()
        .copied()
        .map(named_crate)
        .collect();
    for forbidden in FORBIDDEN {
        assert!(
            !named.contains(forbidden),
            "`{forbidden}` is a dependency of this crate",
        );
        assert!(
            !development.contains(forbidden),
            "`{forbidden}` is a dev-dependency of this crate; a test build that resolves a preset \
             path is still a build that resolves a preset path",
        );
    }
}

#[test]
fn no_forbidden_crate_is_named_anywhere_in_the_source() {
    let files = rust_files(&source_root());
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "the source file count changed: {files:?}",
    );

    let mut offences = Vec::new();
    for file in &files {
        let text = read(file);
        for (number, line) in text.lines().enumerate() {
            // **Code only, and by whole identifier.** This crate's own documentation *names*
            // `mjx-geometry` — the whole point of the rank section is that the edge would be legal
            // and that nothing but this gate refuses it — so a scan that read comments would forbid
            // saying so.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
                continue;
            }
            for forbidden in FORBIDDEN {
                let underscored = forbidden.replace('-', "_");
                if names_crate(line, &underscored) {
                    offences.push(format!(
                        "{}:{}: {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a box model says *this shape, at this size* and stops; resolving it is the geometry \
         provider's, above this crate:\n{}",
        offences.join("\n")
    );
}

#[test]
fn the_crate_forbids_unsafe_outright() {
    // Nothing in the document graph needs `unsafe`, and a box model over untrusted files least of
    // all. `#![forbid]` rather than the workspace's `deny`, because a local `#[allow]` must not be
    // able to switch it back on.
    let lib = read(&source_root().join("lib.rs"));
    assert!(
        lib.contains("#![forbid(unsafe_code)]"),
        "`lib.rs` must forbid `unsafe_code` outright",
    );
}
