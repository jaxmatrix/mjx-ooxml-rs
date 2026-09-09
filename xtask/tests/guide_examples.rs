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
//! * **A Rust-only declaration is held to the names it makes the claim with** — see below.
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
//! # The second limit was real, and `the_round_trip` closed it
//!
//! `saving_validates` starts from [`Deck::blank`], so **every part in the package it compares was
//! authored by this library and none was preserved from an input file.** The comparison is a real
//! one — three languages, one package, part by part — but on its own it says nothing about
//! copy-on-write or about verbatim re-emission, which is the whole point of the round-trip contract.
//! It is the same blind spot `crates/mjx-ooxml/examples/build_a_document.rs` has.
//!
//! `the_round_trip` (MJXOFF-254, this backlog's first item for that reason rather than by alphabet)
//! is the example that closes it. It **opens `tests/fixtures/sample.xlsx`**, which this project did
//! not author, and each of its three halves asserts preservation *inside the block a reader sees*:
//! the same part names before and after, and byte-identical payloads for every one of them. So the
//! property is established by an assertion in the example rather than by the harness's agreement —
//! three languages agreeing on a wrong answer would not pass it, because each one is checked against
//! the input file rather than against the other two.
//!
//! The harness comparison on top of that is a *second* fact: that all three preserved the fixture
//! **the same way**. Both are needed and neither implies the other.
//!
//! [`Deck::blank`]: https://docs.rs/mjx-ooxml
//!
//! # The fourth marker form, and why it is a claim rather than a suppression (MJXOFF-261)
//!
//! A little of this facade is **Rust-only by decision** — the three `*_mut` escape hatches, the
//! typed cause under an [`Error`] — and a guide block about it can never have a Python or a
//! JavaScript half. Until MJXOFF-261 the only way to say so was to write no marker at all, which is
//! indistinguishable from having forgotten. MJXOFF-257 filed the same hole a second time, from the
//! other end of the backlog.
//!
//! The spelling is a marker that names the Rust symbols making the claim true:
//!
//! ```text
//! <!-- guide-example: the_escape_hatches rust-only presentation_mut document_mut workbook_mut -->
//! ```
//!
//! [`a_rust_only_declaration_is_held_to_the_names_it_makes_the_claim_with`] then asks the
//! repository whether the claim is still true, on every run:
//!
//! * **every declared name occurs in the region the block shows** — so the reason is about *this*
//!   block. A declaration listing names the example never calls would be true of the language and
//!   vacuous about the example;
//! * **every declared name is reachable from neither binding**, read out of the committed `.pyi`
//!   and the committed `#[wasm_bindgen]` declarations by [`xtask::binding_surface`] — the module
//!   `xtask/tests/binding_projection.rs` and this file now share, because two parsers of the same
//!   two surfaces would disagree with no way to say which was wrong.
//!
//! And the population test above inverts, rather than drops, its expectation: a Rust-only example
//! must have a Rust half and **no** Python or JavaScript half, and must be shown by no marker in
//! either. So the declaration cannot become the place a binding half goes to be forgotten — which
//! is the property that separates this from an exemption list. `xtask/tests/facade_curation.rs`'s
//! ledger is the same shape one layer down.
//!
//! **What is deliberately not attempted**: a check that the *prose* beside such a block says so in
//! words. That is a sentence, and a gate that grepped for one would be satisfied by any sentence.
//! The marker carries the part a test can check; the page carries the part a reader needs.
//!
//! # The hidden prelude, and why only Rust has one
//!
//! An example that starts from a file needs bytes, and reading a file is the caller's job — every
//! guide page says so. A Python or JavaScript half gets that for free: whatever it does above its
//! sentinel is simply not in the block. A Rust half cannot, because its block is *also* a compiled
//! doctest and a doctest that names `original` without binding it does not compile. So a Rust half
//! may carry an earlier, hidden region, emitted into the block as rustdoc's `#` lines.
//! [`a_prelude_is_a_rust_only_device_and_every_one_of_them_extracts`] holds it to that: a prelude in
//! another language would be a no-op nobody notices.
//!
//! # The mutation register
//!
//! Every test below was made to fail by a reachable mutation; the verbatim output is in the pull
//! request for MJXOFF-254.
//!
//! | Mutation | Fails |
//! |---|---|
//! | reword a comment inside a committed block by hand | [`every_committed_block_is_a_current_copy_of_the_file_a_runner_executes`] |
//! | move a hidden prelude into the Python half | [`a_prelude_is_a_rust_only_device_and_every_one_of_them_extracts`] |
//! | add a fourth marker naming an example with no files | that test, and [`every_guide_example_exists_in_all_three_languages_and_is_shown_in_all_three`] on all three missing halves |
//! | add a Python half no page marks | the population test, on the stray |
//! | stop the JavaScript half exporting its package | [`the_three_halves_of_an_example_agree_about_whether_it_produces_a_package`] |
//! | have a harness return a hand-written list of examples | [`each_binding_harness_runs_the_rust_example_and_reads_both_packages_through_the_shared_reader`] |
//! | `SlideSize::widescreen` → `SlideSize::standard` in one half | the binding's own comparison, naming `ppt/presentation.xml` and `ppt/slideMasters/slideMaster1.xml` — **not** anything in this file, which is the limit stated above |
//! | declare a projected example Rust-only (`addressing_a_workbook rust-only write_cells`) | [`a_rust_only_declaration_is_held_to_the_names_it_makes_the_claim_with`], naming both surfaces that still have it |
//! | give a Rust-only example a Python half | the population test, on the half a Rust-only block may not have |
//! | write `rust-only` with no names after it | the parse itself, before any test runs |
//! | one `rename_sheet` inserted into `the_round_trip`'s Python half | **the example's own assertion**, `AssertionError: /xl/workbook.xml changed`, before the harness comparison is even reached — which is what makes preservation a property of the block rather than of the three languages agreeing |
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

use xtask::binding_surface;
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
    let rust_only = guide_examples::rust_only_examples(&root).expect("the pages parse");
    let mut failures: Vec<String> = Vec::new();

    for language in Language::ALL {
        let present =
            guide_examples::halves_present(&root, language).expect("the directory is readable");

        // An example declared Rust-only is expected in Rust and expected *nowhere else*. The
        // expectation is still two-directional — it has simply been inverted for two of the three
        // languages, which is what stops the declaration being a place a half goes to be forgotten.
        let expected: BTreeSet<String> = match language {
            Language::Rust => marked.clone(),
            Language::Python | Language::JavaScript => marked
                .iter()
                .filter(|name| !rust_only.contains_key(*name))
                .cloned()
                .collect(),
        };

        // ---- Both directions of the population -------------------------------------------------
        // A marker with no file is a block nothing runs; a file with no marker is an example no
        // reader ever sees. Stating only the first is how a language quietly falls behind.
        for missing in expected.difference(&present) {
            failures.push(format!(
                "{language}: a marker names `{missing}`, but {} does not exist",
                language.source_path(missing)
            ));
        }
        for stray in present.difference(&expected) {
            if rust_only.contains_key(stray) {
                failures.push(format!(
                    "{language}: {} exists, but `{stray}` is marked `{}` — a block declared \
                     Rust-only has no half here, and if this one now can, delete the declaration \
                     and mark all three",
                    language.source_path(stray),
                    guide_examples::RUST_ONLY_TOKEN
                ));
            } else {
                failures.push(format!(
                    "{language}: {} exists, but no page marks `{stray}` in any language",
                    language.source_path(stray)
                ));
            }
        }

        // ---- Every example is shown in this language --------------------------------------------
        for name in expected.intersection(&present) {
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

        // ---- And a Rust-only example is shown in *no* other language ----------------------------
        if language != Language::Rust {
            for name in rust_only.keys() {
                let shown = markers
                    .iter()
                    .filter(|(_, marker)| marker.name == *name && marker.language == language)
                    .count();
                if shown != 0 {
                    failures.push(format!(
                        "`{name}` is declared Rust-only and yet is shown by {shown} {language} \
                         marker(s); a page cannot both claim a binding has no half and show one"
                    ));
                }
            }
        }
    }

    println!(
        "{} example(s), {} of them Rust-only, {} marker(s) across {} language(s)",
        marked.len(),
        rust_only.len(),
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
fn a_rust_only_declaration_is_held_to_the_names_it_makes_the_claim_with() {
    // The whole difference between this form and a suppression. A marker that merely turned the
    // three-language rule off would be the nominal gate this repository keeps deleting; this one
    // states *why*, in names, and the repository is asked to confirm it on every run.
    //
    // Two things are confirmed, and they fail in opposite directions:
    //
    //   * every declared name occurs in the region the block shows — so the reason is about *this*
    //     block. A declaration listing names the example never calls would be true of the language
    //     and vacuous about the example.
    //   * every declared name is reachable from **neither** binding. The day one is projected, the
    //     claim has stopped being true and the block owes a Python and a JavaScript half.
    let root = repository_root();
    let declared = guide_examples::rust_only_examples(&root).expect("the pages parse");
    let python = binding_surface::python_names(&root);
    let wasm = binding_surface::wasm_names(&root);
    let mut failures: Vec<String> = Vec::new();
    let mut names = 0usize;

    for (name, symbols) in &declared {
        let relative = Language::Rust.source_path(name);
        let source = read(&relative);
        let region =
            guide_examples::region(&source, &relative).expect("the Rust half has a region");
        for symbol in symbols {
            names += 1;
            if !region.contains(symbol.as_str()) {
                failures.push(format!(
                    "`{name}` is declared Rust-only because of `{symbol}`, but the block a reader \
                     sees never names it — see {relative}"
                ));
            }
            if python.contains(symbol.as_str()) {
                failures.push(format!(
                    "`{name}` claims `{symbol}` is Rust-only, but the committed Python stub \
                     declares it; the claim has stopped being true and the block owes a Python half"
                ));
            }
            if wasm.contains(symbol.as_str()) {
                failures.push(format!(
                    "`{name}` claims `{symbol}` is Rust-only, but `bindings/mjx-wasm/src/` exports \
                     it; the claim has stopped being true and the block owes a JavaScript half"
                ));
            }
        }
    }

    println!(
        "{} Rust-only example(s) declaring {names} name(s), checked against {} Python and {} \
         JavaScript declaration(s)",
        declared.len(),
        python.len(),
        wasm.len()
    );
    assert!(
        failures.is_empty(),
        "these Rust-only declarations are no longer true:\n  {}\n\nA `{}` marker is a claim about \
         the two binding surfaces, not a way to skip writing two halves.",
        failures.join("\n  "),
        guide_examples::RUST_ONLY_TOKEN
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
fn a_prelude_is_a_rust_only_device_and_every_one_of_them_extracts() {
    // A hidden prelude is what lets a Rust block open a file it did not author without showing the
    // reader how this repository finds its fixtures. In the other two languages the same lines are
    // already invisible — they sit above the sentinel — so a prelude there is a no-op that reads
    // like a feature. Saying so is cheaper than discovering it.
    let root = repository_root();
    let mut failures: Vec<String> = Vec::new();
    let mut preludes = 0usize;

    for name in marked_examples(&root) {
        for language in Language::ALL {
            let relative = language.source_path(&name);
            let Ok(source) = std::fs::read_to_string(root.join(&relative)) else {
                continue; // the population test above owns a missing file, and names it better.
            };
            match guide_examples::prelude(&source, &relative) {
                Ok(None) => {}
                Ok(Some(_)) if language == Language::Rust => preludes += 1,
                Ok(Some(_)) => failures.push(format!(
                    "{relative}: only the Rust half hides a prelude, because only its block is also \
                     a doctest; in {language} everything above the sentinel is already hidden"
                )),
                Err(failure) => failures.push(failure.to_string()),
            }
        }
    }

    println!("{preludes} Rust half/halves hide a prelude");
    assert!(
        failures.is_empty(),
        "the hidden preludes are not whole:\n  {}\n\nA prelude is delimited by `{}` and `{}` in a \
         comment, and belongs to the Rust half alone.",
        failures.join("\n  "),
        guide_examples::PRELUDE_START,
        guide_examples::PRELUDE_END
    );
}

#[test]
fn the_three_halves_of_an_example_agree_about_whether_it_produces_a_package() {
    // An example that saves a package binds `saved`, in each language's own spelling, and its two
    // binding harnesses then compare that package against the Rust one part by part. If one half
    // stopped producing one, the comparison would go missing from that binding *and from nowhere
    // else*, which is precisely the shape MJXOFF-239 found in the Word walkthrough.
    let root = repository_root();
    let rust_only = guide_examples::rust_only_examples(&root).expect("the pages parse");
    let mut failures: Vec<String> = Vec::new();
    let mut producing = 0usize;

    for name in marked_examples(&root) {
        if rust_only.contains_key(&name) {
            // One half cannot disagree with itself, and there is no harness to compare against.
            // Counting it as an agreement would inflate the figure this test prints.
            continue;
        }
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

    // ---- The hidden prelude, over samples held here --------------------------------------------
    // Absence is the ordinary answer and must not be an error; a half-open pair must be.
    assert_eq!(
        guide_examples::prelude(sample, "sample").expect("a file with no prelude has none"),
        None,
        "most examples need no setup, and `Ok(None)` is what says so"
    );
    let with_prelude = concat!(
        "fn main() {\n",
        "    // guide-example:prelude-start\n",
        "    let original = fixture();\n",
        "    // guide-example:prelude-end\n",
        "    // guide-example:start\n",
        "    open(&original);\n",
        "    // guide-example:end\n",
        "}\n"
    );
    assert_eq!(
        guide_examples::prelude(with_prelude, "sample").expect("the sample has a prelude"),
        Some("let original = fixture();".to_owned()),
        "the prelude is the lines between its own two sentinels, dedented like any other region"
    );
    assert_eq!(
        guide_examples::region(with_prelude, "sample").expect("the sample has a region"),
        "open(&original);",
        "neither prelude sentinel contains a region sentinel, so the two pairs cannot collide"
    );
    assert_eq!(
        Language::Rust.block_body("open(&original);", Some("let original = fixture();")),
        concat!(
            "# fn main() -> Result<(), Box<dyn std::error::Error>> {\n",
            "# let original = fixture();\n",
            "open(&original);\n",
            "# Ok(())\n# }"
        ),
        "a prelude line is emitted hidden, so it compiles and runs and no reader sees it"
    );
    assert_eq!(
        Language::Python.block_body("open(original)", Some("original = fixture()")),
        "open(original)",
        "the other two languages hide their setup by leaving it above the sentinel"
    );
    assert!(
        guide_examples::prelude("// guide-example:prelude-start\nlet a = 1;\n", "sample").is_err(),
        "an unclosed prelude would silently vanish from the block it is there to make compile"
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

    // ---- The Rust-only form, over pages held here ----------------------------------------------
    // It renders as Rust and carries the names that make the claim; a claim with no names is
    // refused where it is parsed, because that spelling would be a plain suppression.
    let rust_only = concat!(
        "<!-- guide-example: some_name rust-only presentation_mut workbook_mut -->\n",
        "<!-- guide-example end -->\n"
    );
    let markers = guide_examples::markers_in(rust_only, "sample.md").expect("the page parses");
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].language, Language::Rust, "it renders as Rust");
    assert!(markers[0].is_rust_only());
    assert_eq!(markers[0].rust_only, ["presentation_mut", "workbook_mut"]);
    assert!(
        guide_examples::markers_in(
            "<!-- guide-example: some_name rust-only -->\n<!-- guide-example end -->\n",
            "sample.md"
        )
        .is_err(),
        "`rust-only` with no names is a suppression wearing a marker's clothes, and is refused \
         where it is parsed rather than passed on to the gate"
    );
    assert!(
        !guide_examples::markers_in(
            "<!-- guide-example: some_name rust -->\n<!-- guide-example end -->\n",
            "sample.md"
        )
        .expect("the page parses")[0]
            .is_rust_only(),
        "an ordinary `rust` marker declares nothing, and must not be read as a Rust-only one"
    );
}
