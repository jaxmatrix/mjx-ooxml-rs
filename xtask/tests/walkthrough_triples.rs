//! **Every facade walkthrough is compared against both bindings — and the set of compared
//! walkthroughs is the set of walkthroughs.** (MJXOFF-239.)
//!
//! # The instance, and the class it belongs to
//!
//! `CLAUDE.md` has said for a long time that each of the walkthroughs under
//! `crates/mjx-ooxml/examples/` "exists a second time under `bindings/mjx-python/tests/` and a
//! third under `bindings/mjx-wasm/tests/node/`", and that **every one compares its output against
//! the Rust one part by part, byte for byte**. It was false for Word in both bindings.
//! `test_build_a_document.py` and `build_a_document.mjs` transcribed
//! `crates/mjx-ooxml/examples/build_a_document.rs` call for call and each wrote its *own* `.docx`.
//! Nothing ran the Rust example; nothing compared. Both files passed, which is why nobody saw it —
//! a gate phrased *"X is covered and green"* is green precisely when X is skipped.
//!
//! MJXOFF-239 fixed that instance. This file owns the class: **a fourth walkthrough cannot arrive
//! uncompared, and a comparison cannot be deleted from an existing one**, because the population is
//! read out of `crates/mjx-ooxml/examples/` rather than listed here. The same reasoning as
//! `xtask/tests/derived_rosters.rs`, applied to a population that spans three languages.
//!
//! # What this file can check, and what it cannot
//!
//! A comparison has two ingredients that are visible in the text of a test written in a language
//! this crate cannot execute:
//!
//! 1. it **runs the Rust example of its own name** as a subprocess — `--example` and the
//!    walkthrough's name in the same argument list; and
//! 2. it **reads both packages through the shared payload reader** — `part_payloads` on the Python
//!    side, `partPayloads` on the Node side — at least twice, once per side.
//!
//! Both are checked below. What is *not* checked here is that the two payload maps are then
//! actually asserted equal: that is an assertion, in another language, and a textual gate that
//! claimed to verify it would be the same kind of nominal check this file exists to prevent. What
//! establishes it is the ⚠ discipline every unit of this programme follows — break the walkthrough
//! by one argument and confirm the comparison reddens **and names the part**. For the Word pair
//! that was done in MJXOFF-239 and both failures name `word/document.xml`.
//!
//! So: this file makes it impossible for a walkthrough to arrive with *no* comparison, which is the
//! failure that actually happened. It does not make it impossible to write a comparison that
//! compares nothing.
//!
//! # The shared payload reader
//!
//! [`the_package_payload_reader_exists_once_in_each_binding_suite`] is the other half. Word's hole
//! was invisible partly because `_part_payloads` had been *copied* into each Python suite that
//! needed it, so "this suite has no comparison" and "this suite has no copy of the helper" were the
//! same fact and neither was anomalous. One definition per binding suite, imported everywhere else,
//! makes an absent comparison an absent import.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Where the facade's walkthroughs live.
const EXAMPLES: &str = "crates/mjx-ooxml/examples";

/// Where the Python copies live.
const PYTHON_TESTS: &str = "bindings/mjx-python/tests";

/// Where the Node copies live.
const NODE_TESTS: &str = "bindings/mjx-wasm/tests/node";

/// The prefix that makes an example a *walkthrough* rather than a demonstration of one crate.
///
/// `crates/mjx-pptx/examples/` and the rest hold plenty of examples; only the facade's `build_a_*`
/// programs are the three-language acceptance triples this file is about.
const WALKTHROUGH_PREFIX: &str = "build_a_";

/// Every file name in a directory, sorted. Panics with the directory named if it cannot be read.
fn file_names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
        .map(|entry| {
            entry
                .expect("a readable directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// One file, read. Panics with the path named if it is missing.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {relative}: {error}"))
}

/// The walkthroughs, **derived** from `crates/mjx-ooxml/examples/` rather than listed.
///
/// A `build_a_*.rs` added there joins this set with no edit to this file, which is the whole point:
/// the population and the assertion cannot drift apart.
fn walkthroughs() -> Vec<String> {
    let directory = repository_root().join(EXAMPLES);
    let found: Vec<String> = file_names(&directory)
        .into_iter()
        .filter_map(|name| name.strip_suffix(".rs").map(str::to_owned))
        .filter(|stem| stem.starts_with(WALKTHROUGH_PREFIX))
        .collect();
    // A floor, never a total: it says the walk is still matching, and cannot pass in place of the
    // comparison it guards.
    assert!(
        found.len() >= 3,
        "only {} `{WALKTHROUGH_PREFIX}*.rs` example(s) under {EXAMPLES} — the walk has stopped \
         matching",
        found.len()
    );
    found
}

/// One binding's half of a triple: where its copy of a walkthrough lives, and how it names the
/// shared payload reader.
struct BindingSuite {
    /// What a failure calls it.
    language: &'static str,
    /// The directory its walkthrough copies live in.
    directory: &'static str,
    /// The file name of its copy of `build_a_<name>`.
    file_of: fn(&str) -> String,
    /// The inverse: the walkthrough a file in this directory is a copy of, if it is one at all.
    name_of: fn(&str) -> Option<&str>,
    /// The function every comparison in this suite reads packages with.
    payload_reader: &'static str,
    /// The one file that is allowed to define it, and the text that defines it there.
    reader_home: (&'static str, &'static str),
    /// What a file that uses the reader without defining it must contain.
    reader_import: &'static str,
    /// The archive machinery only `reader_home` may name. Open-coding one of these is the same
    /// duplication as copying the reader, wearing a different costume: it reaches a package's parts
    /// without going through the one function that knows what "a part's payload" means.
    archive_machinery: &'static [&'static str],
}

/// The two bindings, in the order a failure lists them.
fn binding_suites() -> [BindingSuite; 2] {
    [
        BindingSuite {
            language: "Python",
            directory: PYTHON_TESTS,
            file_of: |name| format!("test_{name}.py"),
            name_of: |file| file.strip_suffix(".py")?.strip_prefix("test_"),
            payload_reader: "part_payloads(",
            reader_home: ("opc.py", "def part_payloads("),
            reader_import: "from opc import part_payloads",
            archive_machinery: &["zipfile"],
        },
        BindingSuite {
            language: "Node",
            directory: NODE_TESTS,
            file_of: |name| format!("{name}.mjs"),
            name_of: |file| file.strip_suffix(".mjs"),
            payload_reader: "partPayloads(",
            reader_home: ("zip.mjs", "export function partPayloads("),
            reader_import: "from \"./zip.mjs\"",
            archive_machinery: &["node:zlib", "inflateRaw"],
        },
    ]
}

#[test]
fn every_facade_walkthrough_is_compared_in_both_bindings() {
    let walkthroughs = walkthroughs();
    let expected: BTreeSet<String> = walkthroughs.iter().cloned().collect();
    let mut failures: Vec<String> = Vec::new();

    for suite in binding_suites() {
        // ---- Both directions of the population -------------------------------------------------
        // A walkthrough with no copy fails, and a copy with no walkthrough fails. Stating only the
        // first is how Word stayed hidden: the file existed, so nothing looked at what was in it.
        let present: BTreeSet<String> = file_names(&repository_root().join(suite.directory))
            .iter()
            .filter_map(|file| (suite.name_of)(file))
            .filter(|stem| stem.starts_with(WALKTHROUGH_PREFIX))
            .map(str::to_owned)
            .collect();
        for missing in expected.difference(&present) {
            failures.push(format!(
                "{}: {EXAMPLES}/{missing}.rs has no copy at {}/{}",
                suite.language,
                suite.directory,
                (suite.file_of)(missing)
            ));
        }
        for stray in present.difference(&expected) {
            failures.push(format!(
                "{}: {}/{} is a walkthrough copy of {EXAMPLES}/{stray}.rs, which does not exist",
                suite.language,
                suite.directory,
                (suite.file_of)(stray)
            ));
        }

        // ---- What is in each copy ---------------------------------------------------------------
        for name in expected.intersection(&present) {
            let relative = format!("{}/{}", suite.directory, (suite.file_of)(name));
            let text = read(&relative);
            if !(text.contains("--example") && text.contains(&format!("\"{name}\""))) {
                failures.push(format!(
                    "{relative} never runs {EXAMPLES}/{name}.rs: a comparison needs `--example` \
                     and \"{name}\" in one argument list, so this walkthrough has nothing to be \
                     compared against"
                ));
            }
            let reads = text.matches(suite.payload_reader).count();
            // Two: one side each. The definition lives elsewhere, so no occurrence here is a
            // declaration.
            if reads < 2 {
                failures.push(format!(
                    "{relative} calls `{}` {reads} time(s); a part-by-part comparison reads both \
                     packages, so it calls it at least twice",
                    suite.payload_reader.trim_end_matches('(')
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "the walkthrough triples are not whole:\n  {}\n\nEvery `{EXAMPLES}/{WALKTHROUGH_PREFIX}*.rs`\
         \nis copied into both bindings and each copy runs the Rust one and compares its output\
         \npart by part. See this file's module comment.",
        failures.join("\n  ")
    );
}

#[test]
fn the_package_payload_reader_exists_once_in_each_binding_suite() {
    let mut failures: Vec<String> = Vec::new();

    for suite in binding_suites() {
        let directory = repository_root().join(suite.directory);
        let (home, definition) = suite.reader_home;
        let mut definitions: Vec<String> = Vec::new();

        for file in file_names(&directory) {
            let path = directory.join(&file);
            if !path.is_file() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue; // a `.wasm`, a `__pycache__` entry: not a source file.
            };
            if text.contains(definition) {
                definitions.push(file.clone());
            }
            if file != home
                && text.contains(suite.payload_reader)
                && !text.contains(suite.reader_import)
            {
                failures.push(format!(
                    "{}/{file} calls `{}` without `{}`: the reader lives in {}/{home} and is \
                     imported, never copied",
                    suite.directory,
                    suite.payload_reader.trim_end_matches('('),
                    suite.reader_import,
                    suite.directory
                ));
            }
            if file != home {
                for machinery in suite.archive_machinery {
                    if text.contains(machinery) {
                        failures.push(format!(
                            "{}/{file} names `{machinery}`: a package's parts are read through \
                             `{}` in {}/{home}, never opened a second way",
                            suite.directory,
                            suite.payload_reader.trim_end_matches('('),
                            suite.directory
                        ));
                    }
                }
            }
        }

        if definitions != vec![home.to_owned()] {
            failures.push(format!(
                "{}: `{definition}…` is defined in {definitions:?}; it must be defined exactly \
                 once, in {home}",
                suite.directory
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "the shared package-payload reader has been copied:\n  {}\n\nA helper copied per file is a \
         comparison that can go missing from a file\nwithout anything noticing — which is how \
         MJXOFF-239's hole stayed invisible.",
        failures.join("\n  ")
    );
}
