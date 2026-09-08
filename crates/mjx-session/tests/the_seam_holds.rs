//! Three gates over this crate: **the journal has never heard of OOXML**, **nothing in it panics**,
//! and **the machinery actually runs on a document that is not a `.pptx`**.
//!
//! # What the rank does, and what it cannot
//!
//! `xtask/tests/layering.rs` puts this crate at 3.5 and that buys exactly one thing: nothing in the
//! format tier, and nothing below it, can depend on a session. It buys **nothing at all** in the
//! other direction — 3.5 is above 3.0, so `mjx-session → mjx-pptx` is a legal downward edge and
//! always will be. That is not a defect in the rank; it is what the rank is for, and the same
//! sentence is written in `mjx-paint`'s gate about its own.
//!
//! So the property this crate actually has to hold — *the record, the schedule, the undo units and
//! the recovery format are document-agnostic* — is held here and by nothing else:
//!
//! 1. [`no_format_crate_is_named_outside_the_ooxml_module`] scans the source, skipping `src/ooxml/`,
//!    which is the one directory allowed to know what a package is.
//! 2. [`the_manifest_gates_every_format_crate_behind_the_ooxml_feature`] asserts the exact
//!    dependency set **and** that each format crate is `optional`, so
//!    `cargo build -p mjx-session --no-default-features` is a build in which they are not there.
//! 3. [`the_whole_machinery_runs_on_a_document_that_is_not_ooxml`] drives edits, coalescing, undo,
//!    redo, every commit trigger and recovery through `PlainDocument`, which names no package.
//!
//! The third is the one that matters most, and it is the reason `PlainDocument` is not a two-line
//! stub. A trait with a single implementation is a trait nothing has ever been swapped for: it
//! compiles, it is documented as a seam, and the first genuine second implementation discovers that
//! half the contract was written against the first one's habits. This crate has two from the day it
//! landed, and four counting the three real ones.
//!
//! # Why the scanner is local rather than `mjx-paint`'s
//!
//! `crates/mjx-paint/tests/support/manifest.rs` is the shared one, and `mjx-render-oracle` and
//! `mjx-canvas-harness` include it by `#[path]` — reaching **down**, since both sit above rank 5.5.
//! This crate sits at 3.5, below it, so the same include would reach *up* out of the graph for a
//! test helper. `mjx-layout`'s gate keeps its own for the same reason. What is not duplicated is the
//! assertion: the interesting one here — that a dependency is `optional` and named by a feature —
//! is not something the shared scanner answers at all.

use std::path::{Path, PathBuf};

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_session::{
    CommitPolicy, CommitTrigger, ManualClock, MemoryDocument, MemoryJournal, Operation, Recovery,
    Session, UndoPolicy, Value,
};

#[path = "support/mod.rs"]
mod support;

use support::PlainDocument;

/// How many `.rs` files `src/` holds, counting `src/ooxml/`. Exact rather than a floor, so adding a
/// module is a deliberate act that touches this number; a `>=` would pass on a walk that stopped.
const SOURCE_FILE_COUNT: usize = 14;

/// The directory allowed to name a format crate, relative to `src/`.
const OOXML_DIRECTORY: &str = "ooxml";

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
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

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Whether `path` is inside `src/ooxml/`.
fn is_a_residency(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == OOXML_DIRECTORY)
}

#[test]
fn no_format_crate_is_named_outside_the_ooxml_module() {
    /// Everything a journal, a scheduler and a set of undo units may not know exists.
    ///
    /// `mjx-layout` and `mjx-ooxml-core` are of course absent — they are what the crate is written
    /// in. Everything from the packaging tier upward is here, because a session that reached a
    /// `PartName` outside its residencies would be a session with a package in its schedule.
    const FORBIDDEN: &[&str] = &[
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
        // The facade, spelled with its followers rather than bare: a bare `mjx_ooxml` is a prefix
        // of `mjx_ooxml_core`, which this crate names on nearly every page.
        "mjx_ooxml::",
        "mjx_ooxml;",
        "mjx_ooxml}",
    ];

    let files = rust_files(&source_root());
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "the walk found {} source file(s); it is recursive and this count is exact, so either a \
         module was added or the walk stopped early: {files:?}",
        files.len()
    );

    let mut scanned = 0_usize;
    let mut skipped = 0_usize;
    for path in &files {
        if is_a_residency(path) {
            skipped += 1;
            continue;
        }
        scanned += 1;
        let text = read(path);
        for (number, line) in text.lines().enumerate() {
            // Prose may name them and does at length: explaining *why* a session is written in
            // `mjx-layout`'s address vocabulary is exactly the reasoning that stops the next person
            // reaching for a `PartName`. A `use`, a path or a type name is never inside a comment.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for forbidden in FORBIDDEN {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} names `{forbidden}` outside `src/{OOXML_DIRECTORY}/`. The journal, the \
                     schedule, the undo units and the recovery format are document-agnostic; a \
                     format crate reaching one of them is the seam going, not a convenience.",
                    path.display(),
                    number + 1
                );
            }
        }
    }
    assert_eq!(
        skipped, 4,
        "`src/{OOXML_DIRECTORY}/` holds the three residencies and their module root; a different \
         number means a file moved into or out of the one directory this gate exempts"
    );
    assert!(
        scanned >= 9,
        "only {scanned} files outside the residencies were scanned, which cannot be this crate"
    );
}

#[test]
fn the_manifest_gates_every_format_crate_behind_the_ooxml_feature() {
    let manifest = read(&manifest_path());

    let mut table: Option<&str> = None;
    let mut required: Vec<String> = Vec::new();
    let mut optional: Vec<String> = Vec::new();
    let mut development: Vec<String> = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            // `[package]` inherits `version`, `edition` and the rest from the workspace with the
            // very spelling a dependency uses, so the table has to be tracked rather than the whole
            // file scanned.
            table = match header {
                "dependencies" => Some("dependencies"),
                "dev-dependencies" => Some("dev-dependencies"),
                "build-dependencies" => Some("build-dependencies"),
                _ => None,
            };
            continue;
        }
        let Some(table) = table else { continue };
        let Some(name) = declares_a_workspace_dependency(trimmed) else {
            continue;
        };
        assert_ne!(
            table, "build-dependencies",
            "`{name}` is a build-dependency; this crate has no build script and must not grow one"
        );
        let name = name.to_owned();
        if table == "dev-dependencies" {
            development.push(name);
        } else if trimmed.contains("optional = true") {
            optional.push(name);
        } else {
            required.push(name);
        }
    }

    assert_eq!(
        required,
        vec!["mjx-layout", "mjx-ooxml-core", "thiserror"],
        "the *unconditional* dependencies are the address vocabulary (`mjx-layout`), the unit its \
         rectangles are in (`mjx-ooxml-core`) and typed errors. Anything else here is something the \
         journal and the schedule would then be built on."
    );
    assert_eq!(
        optional,
        vec![
            "mjx-pptx",
            "mjx-docx",
            "mjx-xlsx",
            "mjx-sml",
            "mjx-ooxml-types",
        ],
        "every format-side dependency is optional, so `--no-default-features` is a build in which \
         it is genuinely absent rather than merely unused"
    );
    assert_eq!(
        development,
        vec!["mjx-fixtures", "mjx-allocation-counter"],
        "the suites read their corpus from `mjx-fixtures` and their memory bound from the counting \
         allocator, and neither adds an edge to the shipped graph"
    );

    // And every optional dependency is named by the one feature, so none of them can be switched on
    // by a path this file does not know about.
    let feature = manifest
        .split_once("ooxml = [")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(body, _)| body.to_owned())
        .expect("an `ooxml` feature");
    for name in &optional {
        assert!(
            feature.contains(&format!("\"dep:{name}\"")),
            "the `ooxml` feature does not name `dep:{name}`, so that dependency is reachable \
             without it"
        );
    }
    assert_eq!(
        feature.matches("dep:").count(),
        optional.len(),
        "the `ooxml` feature names a number of dependencies that is not the number of optional ones"
    );
}

/// The name a line declares as a workspace dependency, in either of Cargo's two spellings.
fn declares_a_workspace_dependency(line: &str) -> Option<&str> {
    if let Some((name, _)) = line.split_once(".workspace = true") {
        return Some(name.trim());
    }
    let (name, rest) = line.split_once('=')?;
    let rest = rest.trim();
    if !rest.starts_with('{') || !rest.contains("workspace = true") {
        return None;
    }
    Some(name.trim())
}

#[test]
fn nothing_in_the_library_panics() {
    // This crate holds the user's unsaved work. A panic here loses it, so no `unwrap`, no `expect`,
    // no `panic!` and none of the three markers that stand in for unwritten code.
    //
    // Each file is scanned only as far as its `#[cfg(test)]` module, because a unit test asserting
    // through `expect` is exactly what a unit test should do and is not a library path. Indexing is
    // not caught by a text scan and is not pretended to be.
    const FORBIDDEN: &[&str] = &[
        ".unwrap()",
        ".expect(",
        "panic!(",
        "todo!(",
        "unimplemented!(",
    ];

    let files = rust_files(&source_root());
    assert_eq!(files.len(), SOURCE_FILE_COUNT);

    let mut scanned_lines = 0_usize;
    for path in &files {
        let text = read(path);
        let library = text.split("#[cfg(test)]").next().unwrap_or(&text);
        for (number, line) in library.lines().enumerate() {
            scanned_lines += 1;
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for forbidden in FORBIDDEN {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} uses `{forbidden}`: {code}. This crate holds unsaved work; a panic here \
                     loses it.",
                    path.display(),
                    number + 1
                );
            }
        }
    }
    assert!(
        scanned_lines > 1_500,
        "only {scanned_lines} library lines were scanned, which cannot be this crate"
    );
}

#[test]
fn the_whole_machinery_runs_on_a_document_that_is_not_ooxml() {
    let journal = MemoryJournal::new();
    let committed = MemoryDocument::new();
    let mut session = Session::new(
        PlainDocument::new(3, 4),
        ManualClock::new(),
        journal.clone(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::interactive())
    .with_undo_policy(UndoPolicy::default());

    let first = PlainDocument::address(0, 1);
    let second = PlainDocument::address(2, 3);

    // A burst of typing into one node: twenty operations, one dirty part, one undo unit.
    for keystroke in 0..20 {
        session
            .edit(Operation::set_value(
                first.clone(),
                Value::text(format!("h{keystroke}")),
            ))
            .expect("an edit");
        session.clock().advance(30);
        assert!(session.poll().expect("a poll").is_none());
    }
    assert_eq!(session.undo_units().undoable(), 1);
    assert_eq!(session.document().dirty_parts(), 1);

    // A drag on another node, in another part.
    session.begin_gesture();
    for step in 0..10_i64 {
        session
            .edit(Operation::set_bounds(
                second.clone(),
                LayoutRect::from_edges(
                    Emu::from_emu(step),
                    Emu::from_emu(0),
                    Emu::from_emu(step + 100),
                    Emu::from_emu(100),
                ),
            ))
            .expect("an edit");
        session.clock().advance(16);
    }
    // The gesture defers the idle commit however long the drag takes.
    session.clock().advance(10_000);
    assert!(
        session.poll().expect("a poll").is_none(),
        "a commit landing mid-drag costs a dropped frame"
    );
    session.end_gesture();

    let outcome = session
        .poll()
        .expect("a poll")
        .expect("the deferred commit lands as soon as the gesture ends");
    assert_eq!(outcome.trigger, CommitTrigger::Idle);
    assert_eq!(
        outcome.parts_serialised, 2,
        "thirty operations touched two parts, so the commit serialises two"
    );
    assert_eq!(outcome.operations_committed, 30);
    assert_eq!(session.stats().parts_serialised, 2);

    // Undo takes back a whole unit, whichever side of the commit it began on.
    assert_eq!(session.undo_units().undoable(), 2);
    session.undo().expect("an undo").expect("a unit");
    assert_eq!(
        session.document().boxed(2, 3),
        Some(LayoutRect::ZERO),
        "one undo took back the whole drag"
    );
    session.redo().expect("a redo").expect("a unit");
    assert_ne!(session.document().boxed(2, 3), Some(LayoutRect::ZERO));

    // The journal reached its sink and reads back.
    session.flush_journal().expect("a flush");
    let recovered = Recovery::of(&journal.contents()).expect("a readable journal");
    assert!(!recovered.was_torn());
    assert_eq!(
        recovered.undo_records(),
        20,
        "one undo and one redo of a ten-step drag are twenty records"
    );
    assert!(committed.contents().is_some());
}
