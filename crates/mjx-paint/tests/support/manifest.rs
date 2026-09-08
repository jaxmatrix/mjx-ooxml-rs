//! **The manifest scanner every seam gate reads, and the only copy of it.**
//!
//! # Why this file exists, and why it lives here
//!
//! Three crates in this workspace sit where `xtask/tests/layering.rs` cannot help them, and each
//! needs the same instrument: an assertion over the *exact* set of dependencies a `Cargo.toml`
//! declares.
//!
//! * `mjx-paint`, at rank 5.5, because every crate in the workspace is a legal dependency of it.
//! * `mjx-render-oracle` and `mjx-canvas-harness`, because they have **no rank at all** — they are
//!   above the whole graph, the layering test's downward rule has nothing to compare them against,
//!   and it would therefore accept `mjx-canvas-harness -> mjx-pptx` without complaint. `CLAUDE.md`
//!   states the property for both as though something held it; until audit pass 10 nothing did.
//!
//! It lives under `mjx-paint`'s tests because that is where its three defects were found and fixed
//! (see `the_seam_holds.rs`), and because `mjx-paint` is the lowest of the three: a `#[path]`
//! include reaches *down* from the other two, the way `xtask/tests/layering.rs` reaches down into
//! `xtask/src/json.rs`. It is a **source include and not a dependency edge** — nothing appears in
//! `cargo metadata`, so no crate acquires a link to a painter by reading this file.
//!
//! `tests/support/` is not a directory cargo auto-discovers a test target in (that needs
//! `tests/*.rs` or `tests/*/main.rs`), so this compiles once into each including binary and never
//! as a target of its own.
//!
//! # Three holes it had to be taught, and why a copy would not carry them
//!
//! MJXOFF-163 wrote the first version and MJXOFF-164's audit found all three:
//!
//! 1. **It read one dependency table.** `[dependencies]` alone, so `mjx-paint`'s own
//!    `[target.'cfg(target_arch = "wasm32")'.dependencies]` — a table *in the very file the gate is
//!    about* — was invisible, and so was `[dev-dependencies]`.
//! 2. **It read one spelling.** Only `name.workspace = true`, never `name = { workspace = true }`,
//!    which Cargo treats identically and which any dependency with an extra key has to use.
//! 3. **A `[package]`'s own `version.workspace = true` looked like a dependency**, and a `[[test]]`
//!    section's keys looked like one too.
//!
//! Together the first two were a one-commit escape to the facade that neither the gate nor the
//! layering test could see. That history is the argument against a second copy: what is worth
//! sharing is not the fifty lines, it is the fifty lines *as corrected*.
//!
//! [`the_scanner_reads_every_table_and_both_spellings`] is the instrument's own test, and it
//! compiles into **every** including binary — so each gate proves the scanner it is using rather
//! than trusting one that another crate proved.

// This file is a `mod` inside three different test binaries and is never a crate root, so the
// workspace's `unreachable_pub` lint is right about every item in it and wrong about what to do:
// `pub(crate)` would be the correct visibility if this were one crate's module, and it is three
// crates' module. `dead_code` is here for the same reason — each gate uses the part of the scanner
// its own seam needs, and `mjx-paint` has a font-engine exemption that the two crates above the
// graph have nothing like.
#![allow(unreachable_pub, dead_code)]

use std::path::{Path, PathBuf};

/// Every workspace dependency `manifest` declares, in **every** table, in **both** spellings.
///
/// Each entry is tagged with the table it came from — `"wgpu (dev)"`,
/// `"wgpu (target cfg(target_arch = \"wasm32\"))"` — so a dependency that *moved* between tables
/// changes the assertion rather than passing silently.
pub fn workspace_dependencies(manifest: &str) -> Vec<String> {
    let mut table: Option<String> = None;
    let mut declared = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            table = classify(header);
            continue;
        }
        let Some(suffix) = table.as_deref() else {
            continue;
        };
        let Some(name) = declares_a_workspace_dependency(trimmed) else {
            continue;
        };
        declared.push(if suffix.is_empty() {
            name.to_owned()
        } else {
            format!("{name} ({suffix})")
        });
    }
    declared
}

/// What a table header means for this scan, or `None` for a table that declares no dependencies.
///
/// A `[target.'...'.dependencies]` keeps its condition in the tag, because *"`wgpu` on `wasm32`"*
/// and *"`wgpu` everywhere"* are different declarations, and an assertion that could not tell them
/// apart would accept a dependency moved from one to the other.
pub fn classify(header: &str) -> Option<String> {
    match header {
        "dependencies" => return Some(String::new()),
        "dev-dependencies" => return Some("dev".to_owned()),
        "build-dependencies" => return Some("build".to_owned()),
        _ => {}
    }
    let rest = header.strip_prefix("target.")?;
    let (condition, kind) = rest.rsplit_once('.')?;
    let condition = condition.trim_matches('\'').trim_matches('"');
    match kind {
        "dependencies" => Some(format!("target {condition}")),
        "dev-dependencies" => Some(format!("dev, target {condition}")),
        _ => None,
    }
}

/// The name a line declares as a workspace dependency, in either spelling.
pub fn declares_a_workspace_dependency(line: &str) -> Option<&str> {
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

/// Every `.rs` file under `directory`, recursively, sorted.
///
/// Recursive on purpose: MJXOFF-155 §8 lists the non-recursive walk as a recurring defect in this
/// project, and a gate that stopped at the top level would pass forever over a `scenes/` directory.
pub fn rust_files(directory: &Path) -> Vec<PathBuf> {
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

/// One file, or a panic naming it.
pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Every line of every file under `root` that names one of `forbidden`, outside a comment.
///
/// Comments are skipped because prose *may* name a forbidden crate, and does at length: explaining
/// why a display list may not reach a format crate is exactly the reasoning that stops the next
/// person doing it. A `use`, a path or a type name is never inside a comment.
///
/// Returns `file:line — the line` for each offence, in the order the walk found them.
pub fn lines_naming(root: &Path, forbidden: &[&str]) -> Vec<String> {
    let mut offences = Vec::new();
    for path in rust_files(root) {
        let text = read(&path);
        for (number, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in forbidden {
                if line.contains(name) {
                    offences.push(format!(
                        "{}:{} names `{name}` — {}",
                        path.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    offences
}

/// **The instrument's own test**, compiled into every gate that includes this file.
///
/// A parser that saw one table would pass its crate's manifest assertion forever while a dependency
/// sat unread in another — which is exactly what happened before MJXOFF-164. A scanner that
/// silently stops discriminating passes forever, so each of the three defects above is refused by
/// name here.
#[test]
fn the_scanner_reads_every_table_and_both_spellings() {
    const SAMPLE: &str = concat!(
        "[package]\n",
        "name = \"x\"\n",
        "version.workspace = true\n",
        "[dependencies]\n",
        "mjx-scene.workspace = true\n",
        "thiserror = { workspace = true }\n",
        "serde = \"1\"\n",
        "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\n",
        "wgpu = { workspace = true, features = [\"webgl\"] }\n",
        "[dev-dependencies]\n",
        "mjx-fixtures.workspace = true\n",
        "[[test]]\n",
        "name = \"harnessless\"\n"
    );
    assert_eq!(
        workspace_dependencies(SAMPLE),
        vec![
            "mjx-scene",
            "thiserror",
            "wgpu (target cfg(target_arch = \"wasm32\"))",
            "mjx-fixtures (dev)",
        ],
        "the scanner must read every dependency table and both spellings, and must not mistake \
         `[package]`'s inherited `version.workspace = true` or a `[[test]]` section for one"
    );
    assert_eq!(declares_a_workspace_dependency("serde = \"1\""), None);
    assert_eq!(
        declares_a_workspace_dependency("wgpu = { version = \"30\" }"),
        None,
        "a table without `workspace = true` in it is not a workspace dependency"
    );
    assert_eq!(classify("package"), None);
    assert_eq!(classify("dependencies"), Some(String::new()));
    assert_eq!(classify("dev-dependencies"), Some("dev".to_owned()));
}

/// **The source scanner's own test**, for the same reason.
///
/// A manifest scanner that silently stopped seeing a category would pass forever, and so would a
/// *source* scanner — the more so because its normal answer is the empty vector, which is
/// indistinguishable from the answer of a scan that read nothing. So it is run over a synthetic
/// tree that contains one offence and one comment, and required to report exactly the first.
#[test]
fn the_source_scanner_finds_an_offence_and_ignores_a_comment() {
    let root = std::env::temp_dir().join(format!(
        "mjx-seam-instrument-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested).expect("a temporary directory");
    std::fs::write(
        root.join("clean.rs"),
        "// mjx_pptx is named here in prose, deliberately, and this line must be ignored.\nfn a() {}\n",
    )
    .expect("a file");
    // In a subdirectory, so the walk's recursion is what finds it. A non-recursive scan would
    // report nothing and pass.
    std::fs::write(nested.join("dirty.rs"), "use mjx_pptx::Presentation;\n").expect("a file");

    let offences = lines_naming(&root, &["mjx_pptx"]);
    assert_eq!(
        offences.len(),
        1,
        "the scan must find the `use` in the subdirectory and must not report the comment: \
         {offences:?}"
    );
    assert!(
        offences[0].contains("dirty.rs:1") && offences[0].contains("use mjx_pptx::Presentation;"),
        "the offence must name the file, the line and the line's text: {}",
        offences[0]
    );
    assert!(
        lines_naming(&root, &["mjx_docx"]).is_empty(),
        "a name nothing uses must produce nothing"
    );
    let _ = std::fs::remove_dir_all(&root);
}
