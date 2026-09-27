//! **A viewport has never heard of OOXML** — held here, because the rank cannot hold it.
//!
//! # What rank 3.8 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate at 3.8, and that buys exactly one thing: nothing in the
//! format tier, nothing in shared markup, nothing at `mjx-session`'s own rank and nothing below any
//! of them can depend on a viewport. A `.pptx` reader with a scroll position inside it would be a
//! batch library with a window manager in it, and at 3.8 it is structurally impossible.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points *up or sideways*, so `mjx-view → mjx-pptx` (3.0), `mjx-view → mjx-dml` (2.0) and
//! `mjx-view → mjx-geometry` (2.5) are legal *downward* edges and always will be. That is not a
//! defect in the rank; it is what a rank is. The same sentence is written in `mjx-session`'s gate
//! about 3.5 and in `mjx-paint`'s about 5.5.
//!
//! So the property this crate actually has to hold is held by three things and by nothing else:
//!
//! 1. [`the_manifest_names_no_format_crate_and_reaches_the_session_without_its_residencies`] — the
//!    dependency set is exact, and `mjx-session` is named with its default features **off**, so a
//!    plain `cargo test -p mjx-view` is a build in which the three format crates are not present;
//! 2. [`no_format_crate_is_named_anywhere_in_the_source`] — a source scan, which catches a `use`
//!    that a feature-gated build would have compiled;
//! 3. [`nothing_in_this_crate_spawns_a_thread_or_reads_a_wall_clock`] — the two things that would
//!    quietly make the `wasm32` path a lie.
//!
//! # Why the scanner is local rather than `mjx-paint`'s
//!
//! `crates/mjx-paint/tests/support/manifest.rs` is the shared one, and `mjx-render-oracle` and
//! `mjx-canvas-harness` include it by `#[path]` — reaching **down**, since both sit above rank 5.5.
//! This crate sits at 3.8, below it, so the same include would reach *up* out of the graph for a
//! test helper. `mjx-session`'s gate and `mjx-layout`'s keep their own for the same reason.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number; a `>=` would pass on a walk that stopped early.
const SOURCE_FILE_COUNT: usize = 7;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-ooxml-core",
    "mjx-layout",
    "mjx-scene",
    "mjx-session",
    "thiserror",
];

/// The crates a viewport must never name — the format tier, shared markup, and the two crates that
/// would drag a document or a graphics stack in behind them.
const FORBIDDEN: &[&str] = &[
    "mjx-pptx",
    "mjx-docx",
    "mjx-xlsx",
    "mjx-dml",
    "mjx-sml",
    "mjx-chart",
    "mjx-omml",
    "mjx-vml",
    "mjx-geometry",
    "mjx-opc",
    "mjx-mce",
    "mjx-ooxml-types",
    "mjx-ooxml",
    "mjx-paint",
];

/// Whether `code` names the crate `needle` as a whole identifier.
///
/// A bare `contains` would report `mjx_ooxml_core` as `mjx_ooxml`, so the character after the match
/// has to end the identifier — a `:`, a `;`, a space, the end of the line.
fn names_crate(code: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(at) = code[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let before_ends_identifier = start == 0
            || !code[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
        let after_ends_identifier = code[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
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

/// The lines of `[dependencies]`, up to the next section header.
fn dependency_lines(manifest: &str) -> Vec<&str> {
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[dependencies]")
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
fn the_manifest_names_no_format_crate_and_reaches_the_session_without_its_residencies() {
    let manifest = read(&manifest_path());
    let mut named: Vec<&str> = dependency_lines(&manifest)
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
        "the dependency set changed. A viewport's edges are the whole of what keeps it \
         document-agnostic, so widening them is a deliberate act and this list moves with it.",
    );

    // The one that matters, and the one a name alone would not catch: a session with its default
    // features on brings `mjx-pptx`, `mjx-docx` and `mjx-xlsx` with it, and `cargo test -p mjx-view`
    // would then be a build in which a `use mjx_pptx::…` compiled.
    assert!(
        manifest.contains(r#"mjx-session = { path = "../mjx-session", default-features = false }"#),
        "`mjx-session` must be named with `default-features = false`, or the format crates are \
         present in this crate's own build and the seam is a comment",
    );

    // And the feature that turns them back on forwards rather than declaring anything of its own.
    assert!(
        manifest.contains(r#"ooxml = ["mjx-session/ooxml"]"#),
        "the `ooxml` feature must forward to the session and nothing else",
    );
    assert!(
        manifest.contains("default = []"),
        "the default feature set must be empty, so the format crates are absent by default",
    );
}

#[test]
fn no_format_crate_is_named_anywhere_in_the_source() {
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
            // **Code only, and by whole identifier.** Two reasons, both learned here:
            //
            // * this crate's own documentation *names* the format tier — the whole point of
            //   `lib.rs`'s rank section is that `mjx-view → mjx-pptx` is a legal downward edge that
            //   nothing but this gate refuses — so a scan that read comments would forbid saying so;
            // * `mjx_ooxml_core` contains `mjx_ooxml`, and a substring match would report the
            //   crate this one is built on as a forbidden facade.
            let code = line.split("//").next().unwrap_or(line);
            for forbidden in FORBIDDEN {
                if names_crate(code, &forbidden.replace('-', "_")) {
                    offences.push(format!(
                        "{}:{}: names `{forbidden}`",
                        file.display(),
                        number + 1
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a viewport named a document:\n{}",
        offences.join("\n"),
    );
}

#[test]
fn nothing_in_this_crate_spawns_a_thread_or_reads_a_wall_clock() {
    // Two things that would make the `wasm32` claim false without failing to compile.
    //
    // `std::thread` — `wasm32` without cross-origin isolation has no threads, so correctness may
    // not depend on having them. `Instant::now()` — it *compiles* on
    // `wasm32-unknown-unknown` and **panics at run time**, which is the worst of both: the
    // cross-build matrix would stay green and the browser would not.
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = read(&file);
        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or(line);
            for needle in [
                "std::thread",
                "thread::spawn",
                "Instant::now",
                "SystemTime::now",
                "static mut",
            ] {
                if code.contains(needle) {
                    offences.push(format!("{}:{}: {needle}", file.display(), number + 1));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "the single-threaded, no-wall-clock claim is not true:\n{}",
        offences.join("\n"),
    );
}

#[test]
fn no_library_path_can_panic_on_a_document_it_does_not_like() {
    // `unwrap`, `expect` and `panic!` on a path that sees untrusted input. The whole crate is such a
    // path: every page it lays out came from a file somebody opened.
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = read(&file);
        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or(line);
            for needle in [
                ".unwrap()",
                ".expect(",
                "panic!(",
                "unreachable!(",
                "todo!(",
            ] {
                if code.contains(needle) {
                    offences.push(format!("{}:{}: {needle}", file.display(), number + 1));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a library path can panic:\n{}",
        offences.join("\n"),
    );
}
