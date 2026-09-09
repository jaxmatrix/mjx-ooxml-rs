//! **A guide example exists in all three languages, and every block a reader sees is a copy of a
//! file a test runner ran.** (MJXOFF-254.)
//!
//! # The failure this closes
//!
//! §4 of the Phase H brief asks for every example in Rust, Python and TypeScript. Its own warning is
//! the reason this file exists rather than nineteen transcribed blocks: *three code blocks that
//! drift apart are worse than one, because two of them become confidently wrong.* A transcription is
//! held together by whoever last remembered to update all three. Phase G's whole lesson is that
//! prose is not checked, and a code block in a `.md` is prose that looks like code.
//!
//! So the guide holds **markers, not code**. Each example is three real files:
//!
//! | Language | File | Runner |
//! |---|---|---|
//! | Rust | `crates/mjx-ooxml/examples/guide_<name>.rs` | `cargo run --example`, and the same region again as a doctest under `cargo test --doc -p mjx-ooxml` |
//! | Python | `bindings/mjx-python/tests/guide_examples/<name>.py` | `pytest`, through `bindings/mjx-python/tests/test_guide_examples.py` |
//! | JavaScript | `bindings/mjx-wasm/tests/node/guide_examples/<name>.mjs` | `node --test`, through `bindings/mjx-wasm/tests/node/guide_examples.mjs` |
//!
//! `cargo run -p xtask -- guide-examples` copies each file's sentinel-delimited region into the
//! block that marks it, and everything below asks whether that was done and whether the three files
//! are still three halves of one example. The extraction itself lives in
//! [`xtask::guide_examples`] and is the *same code* the command runs: a gate that re-implemented it
//! would be comparing a second extractor against the first.
//!
//! # What this file checks, in which directions
//!
//! * **Both directions of the population**, four ways: every example named by a marker has all
//!   three halves, and every half has its markers. A half with no marker is a file nothing shows; a
//!   marker with no half is a block nothing runs.
//! * **Content equality** — every committed block byte-equals the region of its source today.
//! * **Sentinels** — every half really has a region, and the region is not empty.
//! * **The three halves agree about producing a package**, so the output comparison is present in
//!   both bindings or absent from all three by construction, never missing from one.
//! * **Each binding harness runs the Rust example and reads both packages through that binding's
//!   one shared payload reader** — `part_payloads` in `bindings/mjx-python/tests/opc.py`,
//!   `partPayloads` in `bindings/mjx-wasm/tests/node/zip.mjs`. The same two ingredients
//!   `xtask/tests/walkthrough_triples.rs` checks, for the same reason.
//! * **Neither harness names an individual example.** A harness that listed them would be a roster
//!   over a population the filesystem already enumerates, which is
//!   `xtask/tests/derived_rosters.rs`'s subject; here the inverse is cheap to state directly.
//!
//! # What it cannot check — the same limit `walkthrough_triples.rs` states
//!
//! That the two payload maps a harness reads are then **actually asserted equal**. That is an
//! assertion, in a language this crate cannot execute, and a textual gate claiming to verify it
//! would be exactly the nominal check this file exists to prevent. What establishes it is the same
//! discipline: break one half by one argument and confirm the comparison reddens and names the
//! parts. For `saving_validates` that was done in MJXOFF-254 — `SlideSize::widescreen` swapped for
//! `SlideSize::standard` in one language at a time — and both failures name the same two,
//! `ppt/presentation.xml` and `ppt/slideMasters/slideMaster1.xml`.
//!
//! So this file makes it impossible for an example to arrive in fewer than three languages, or for
//! a block to drift from the file it copies. It does not make it impossible to write a comparison
//! that compares nothing.
//!
//! # A second limit, and it belongs to the example rather than to the mechanism
//!
//! `saving_validates` starts from [`Deck::blank`], so **every part in the package it compares was
//! authored by this library and none was preserved from an input file.** The comparison is a real
//! one — three languages, one package, part by part — but it says nothing about copy-on-write or
//! about verbatim re-emission, which is the whole point of the round-trip contract. It is the same
//! blind spot `crates/mjx-ooxml/examples/build_a_document.rs` has, and for the same reason. An
//! example that *opens a committed fixture* would close it, and it is the first item of the backlog
//! in MJXOFF-254 for that reason rather than by alphabet.
//!
//! [`Deck::blank`]: https://docs.rs/mjx-ooxml
//!
//! # The mutation register
//!
//! Every test below was made to fail by a reachable mutation; the verbatim output is in the pull
//! request for MJXOFF-254.
//!
//! | Mutation | Fails |
//! |---|---|
//! | reword a comment inside a committed block by hand | [`every_committed_block_is_a_current_copy_of_the_file_a_runner_executes`] |
//! | add a fourth marker naming an example with no files | that test, and [`every_guide_example_exists_in_all_three_languages_and_is_shown_in_all_three`] on all three missing halves |
//! | add a Python half no page marks | the population test, on the stray |
//! | stop the JavaScript half exporting its package | [`the_three_halves_of_an_example_agree_about_whether_it_produces_a_package`] |
//! | have a harness return a hand-written list of examples | [`each_binding_harness_runs_the_rust_example_and_reads_both_packages_through_the_shared_reader`] |
//! | `SlideSize::widescreen` → `SlideSize::standard` in one half | the binding's own comparison, naming `ppt/presentation.xml` and `ppt/slideMasters/slideMaster1.xml` — **not** anything in this file, which is the limit stated above |
//!
//! # Anti-vacuity
//!
//! A scanner that has stopped matching finds no markers and passes everything.
//! [`the_region_extractor_still_matches_the_sentinels_it_is_written_against`] runs the extractor
//! over samples held in this file and depends on no corpus at all, every population floor is
//! phrased as *the walk has stopped matching* rather than as a total, and every test prints its
//! counts.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use xtask::guide_examples::{self, Language};

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// One file, read. Panics with the path named if it is missing.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {relative}: {error}"))
}

/// Every example a marker names, derived from the pages rather than listed.
fn marked_examples(root: &Path) -> BTreeSet<String> {
    let markers = guide_examples::all_markers(root).expect("the pages parse");
    let names: BTreeSet<String> = markers
        .iter()
        .map(|(_, marker)| marker.name.clone())
        .collect();
    // A floor, never a total: it says the marker scanner is still matching, and cannot pass in
    // place of the comparisons it guards.
    assert!(
        !names.is_empty(),
        "no `{}…` marker anywhere in the repository's pages — the marker scanner has stopped \
         matching",
        guide_examples::MARKER_PREFIX
    );
    names
}

/// One binding's harness: the file that runs every guide example and compares it against Rust.
struct Harness {
    /// What a failure calls it.
    language: &'static str,
    /// The harness, relative to the repository root.
    path: &'static str,
    /// The function every comparison in that suite reads packages with.
    payload_reader: &'static str,
    /// What a file using the reader must contain, since the reader is imported and never copied.
    reader_import: &'static str,
    /// The directory it derives its examples from, so a failure can say where to look.
    examples_directory: &'static str,
}

/// The two harnesses, in the order a failure lists them.
fn harnesses() -> [Harness; 2] {
    [
        Harness {
            language: "Python",
            path: "bindings/mjx-python/tests/test_guide_examples.py",
            payload_reader: "part_payloads(",
            reader_import: "from opc import part_payloads",
            examples_directory: Language::Python.directory(),
        },
        Harness {
            language: "Node",
            path: "bindings/mjx-wasm/tests/node/guide_examples.mjs",
            payload_reader: "partPayloads(",
            reader_import: "from \"./zip.mjs\"",
            examples_directory: Language::JavaScript.directory(),
        },
    ]
}

#[test]
fn every_guide_example_exists_in_all_three_languages_and_is_shown_in_all_three() {
    let root = repository_root();
    let marked = marked_examples(&root);
    let markers = guide_examples::all_markers(&root).expect("the pages parse");
    let mut failures: Vec<String> = Vec::new();

    for language in Language::ALL {
        let present =
            guide_examples::halves_present(&root, language).expect("the directory is readable");

        // ---- Both directions of the population -------------------------------------------------
        // A marker with no file is a block nothing runs; a file with no marker is an example no
        // reader ever sees. Stating only the first is how a language quietly falls behind.
        for missing in marked.difference(&present) {
            failures.push(format!(
                "{language}: a marker names `{missing}`, but {} does not exist",
                language.source_path(missing)
            ));
        }
        for stray in present.difference(&marked) {
            failures.push(format!(
                "{language}: {} exists, but no page marks `{stray}` in any language",
                language.source_path(stray)
            ));
        }

        // ---- Every example is shown in this language --------------------------------------------
        for name in marked.intersection(&present) {
            let shown = markers
                .iter()
                .filter(|(_, marker)| marker.name == *name && marker.language == language)
                .count();
            if shown != 1 {
                failures.push(format!(
                    "{language}: `{name}` is shown by {shown} marker(s); each example is shown \
                     exactly once per language, so a reader who arrives in one language is never \
                     sent to another"
                ));
            }
        }
    }

    println!(
        "{} example(s), {} marker(s) across {} language(s)",
        marked.len(),
        markers.len(),
        Language::ALL.len()
    );
    assert!(
        failures.is_empty(),
        "the guide examples are not whole:\n  {}\n\nEvery example is three files and three \
         blocks. See this file's module comment.",
        failures.join("\n  ")
    );
}

#[test]
fn every_committed_block_is_a_current_copy_of_the_file_a_runner_executes() {
    let root = repository_root();
    let stale = guide_examples::plan(&root).expect("the pages and sources parse");
    println!(
        "{} page(s) hold markers; {} of them are out of date",
        guide_examples::all_markers(&root)
            .expect("the pages parse")
            .iter()
            .map(|(page, _)| page.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        stale.len()
    );
    assert!(
        stale.is_empty(),
        "these page(s) show a block that is not what its source file says today:\n  {}\n\nA block \
         is a copy, never a transcription: run `cargo run -p xtask -- guide-examples`.",
        stale
            .iter()
            .map(|update| update.path.clone())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn every_half_of_every_example_carries_a_region_with_something_in_it() {
    let root = repository_root();
    let mut failures: Vec<String> = Vec::new();
    let mut regions = 0usize;

    for name in marked_examples(&root) {
        for language in Language::ALL {
            let relative = language.source_path(&name);
            let Ok(source) = std::fs::read_to_string(root.join(&relative)) else {
                continue; // the population test above owns a missing file, and names it better.
            };
            match guide_examples::region(&source, &relative) {
                Ok(_) => regions += 1,
                Err(failure) => failures.push(failure.to_string()),
            }
        }
    }

    println!("{regions} region(s) extracted");
    assert!(
        regions > 0,
        "no region was extracted from any half — the sentinel walk has stopped matching"
    );
    assert!(
        failures.is_empty(),
        "these half/halves have no usable region:\n  {}\n\nEach is delimited by `{}` and `{}` in \
         a comment.",
        failures.join("\n  "),
        guide_examples::REGION_START,
        guide_examples::REGION_END
    );
}

#[test]
fn the_three_halves_of_an_example_agree_about_whether_it_produces_a_package() {
    // An example that saves a package binds `saved`, in each language's own spelling, and its two
    // binding harnesses then compare that package against the Rust one part by part. If one half
    // stopped producing one, the comparison would go missing from that binding *and from nowhere
    // else*, which is precisely the shape MJXOFF-239 found in the Word walkthrough.
    let root = repository_root();
    let mut failures: Vec<String> = Vec::new();
    let mut producing = 0usize;

    for name in marked_examples(&root) {
        let mut produces: Vec<(Language, bool)> = Vec::new();
        for language in Language::ALL {
            let relative = language.source_path(&name);
            let Ok(source) = std::fs::read_to_string(root.join(&relative)) else {
                continue;
            };
            let signal = match language {
                Language::Rust => "let saved",
                Language::Python => "saved =",
                Language::JavaScript => "export { saved }",
            };
            produces.push((language, source.contains(signal)));
        }
        let agreed = produces.iter().all(|(_, yes)| *yes)
            || produces.iter().all(|(_, yes)| !*yes)
            || produces.is_empty();
        if !agreed {
            failures.push(format!(
                "`{name}`: {}",
                produces
                    .iter()
                    .map(|(language, yes)| format!(
                        "{language} {}",
                        if *yes { "saves" } else { "does not save" }
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        } else if produces.iter().all(|(_, yes)| *yes) && !produces.is_empty() {
            producing += 1;
        }
    }

    println!("{producing} example(s) produce a package in all three languages");
    assert!(
        failures.is_empty(),
        "these example(s) disagree across languages about producing a package:\n  {}\n\nOne half \
         that stops saving takes a comparison with it, and only that binding notices.",
        failures.join("\n  ")
    );
}

#[test]
fn each_binding_harness_runs_the_rust_example_and_reads_both_packages_through_the_shared_reader() {
    let root = repository_root();
    let marked = marked_examples(&root);
    let mut failures: Vec<String> = Vec::new();

    for harness in harnesses() {
        let text = read(harness.path);
        if !text.contains("--example") {
            failures.push(format!(
                "{}: {} never passes `--example`, so nothing it compares against was produced by \
                 the Rust half",
                harness.language, harness.path
            ));
        }
        if !text.contains(harness.reader_import) {
            failures.push(format!(
                "{}: {} does not contain `{}`; the payload reader is imported, never copied",
                harness.language, harness.path, harness.reader_import
            ));
        }
        let reads = text.matches(harness.payload_reader).count();
        // Two: one side each. The definition lives elsewhere, so no occurrence here declares it.
        if reads < 2 {
            failures.push(format!(
                "{}: {} calls `{}` {reads} time(s); a part-by-part comparison reads both packages, \
                 so it calls it at least twice",
                harness.language,
                harness.path,
                harness.payload_reader.trim_end_matches('(')
            ));
        }
        // A harness that named its examples would be a hand-written roster over a population the
        // filesystem already enumerates — `xtask/tests/derived_rosters.rs`'s subject exactly.
        for name in &marked {
            if text.contains(name.as_str()) {
                failures.push(format!(
                    "{}: {} names `{name}`; a harness derives its examples from {}, so an example \
                     added there joins it with no edit",
                    harness.language, harness.path, harness.examples_directory
                ));
            }
        }
    }

    println!("{} harness(es) checked", harnesses().len());
    assert!(
        failures.is_empty(),
        "the guide-example harnesses are not whole:\n  {}\n\nSee this file's module comment for \
         what a harness has to do and what this gate cannot see.",
        failures.join("\n  ")
    );
}

#[test]
fn the_region_extractor_still_matches_the_sentinels_it_is_written_against() {
    // Depends on no corpus: if every source file lost its sentinels tomorrow, the tests above would
    // have nothing to iterate and this one would still fail.
    let sample = concat!(
        "fn main() {\n",
        "    // guide-example:start\n",
        "    let deck = blank();\n",
        "\n",
        "    deck.save();\n",
        "    // guide-example:end\n",
        "}\n"
    );
    let extracted = guide_examples::region(sample, "sample").expect("the sample has a region");
    assert_eq!(
        extracted, "let deck = blank();\n\ndeck.save();",
        "the extractor takes the lines between the sentinels, dedents them, and keeps the blank \
         line in the middle"
    );

    // ---- And the near-misses it must reject -------------------------------------------------
    // Each of these would otherwise render an empty or a wrong block, silently.
    let no_sentinels = "fn main() {\n    let deck = blank();\n}\n";
    assert!(
        guide_examples::region(no_sentinels, "sample").is_err(),
        "a file with no sentinels has no region, and saying so is the whole of the check"
    );
    let empty = concat!(
        "fn main() {\n",
        "    // guide-example:start\n",
        "    // guide-example:end\n",
        "}\n"
    );
    assert!(
        guide_examples::region(empty, "sample").is_err(),
        "an empty region renders an empty block, which reads as an example rather than as a hole"
    );
    let twice = concat!(
        "// guide-example:start\n",
        "let a = 1;\n",
        "// guide-example:end\n",
        "// guide-example:start\n",
        "let b = 2;\n",
        "// guide-example:end\n"
    );
    assert!(
        guide_examples::region(twice, "sample").is_err(),
        "two regions in one file means the block shows one of them and nothing says which"
    );

    // ---- The marker scanner, over a page held here ---------------------------------------------
    let page = concat!(
        "prose\n",
        "<!-- guide-example: some_name python -->\n",
        "```python\n",
        "x = 1\n",
        "```\n",
        "<!-- guide-example end -->\n"
    );
    let markers = guide_examples::markers_in(page, "sample.md").expect("the page parses");
    assert_eq!(markers.len(), 1, "the marker scanner must find the marker");
    assert_eq!(markers[0].name, "some_name");
    assert_eq!(markers[0].language, Language::Python);
    assert!(
        guide_examples::markers_in(
            "<!-- guide-example: some_name cobol -->\n<!-- guide-example end -->\n",
            "sample.md"
        )
        .is_err(),
        "a marker naming a language the API does not ship in is an error, not a marker skipped"
    );
    assert!(
        guide_examples::markers_in("<!-- guide-example: some_name python -->\n", "sample.md")
            .is_err(),
        "an unclosed marker is an error: everything after it would otherwise be swallowed"
    );
}
