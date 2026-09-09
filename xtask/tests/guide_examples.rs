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
//! So the guide holds **markers, not code**. Each example is three real files — with one exception
//! that says so out loud, [below](#the-fourth-marker-form-and-why-it-is-a-claim-rather-than-a-suppression-mjxoff-261):
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
//! * **The three halves offer the packages the Rust half declares** (MJXOFF-262, MJXOFF-260). The
//!   fact is stated once, by the Rust half, as a *list* of binding names or the word `none` — so an
//!   example that stops producing a package fails against its own statement instead of turning its
//!   comparison into a skip in one binding, and an example that authors two can offer two.
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
//! | stop the JavaScript half exporting its package | [`the_three_halves_offer_the_packages_the_rust_half_declares`] |
//! | delete `saved` from **all three** halves of an example that declares one | that test, three times — the mutation the boolean form could not see (MJXOFF-262) |
//! | drop a Rust half's `guide-example:packages` line | the declaration parse itself, before any comparison |
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

/// **The three halves of an example offer the packages its Rust half declares** (MJXOFF-262,
/// MJXOFF-260).
///
/// # What this replaced, and why the replacement is not the same test
///
/// Until MJXOFF-262 this asked whether the three halves *agreed with each other* about producing a
/// package, by looking for a binding named `saved` in each. That is a real check and it caught one
/// half falling behind — which is the shape MJXOFF-239 found in the Word walkthrough — but it could
/// not see all three stopping together, which is exactly what an author who "simplified" an example
/// would do. Seven of the seventeen tri-language examples produce no package, five of them because
/// they are about a refusal the library reports, so **more than a third of the corpus took the
/// harnesses' skip path** and a skip that is correct that often is a skip nobody reads.
///
/// So the fact is now *stated*, once, by the Rust half — see
/// [`guide_examples::PACKAGES_DECLARATION`] — and all three halves are held to the statement rather
/// than to each other. An example that declares `none` and then binds something fails; one that
/// declares `saved` and binds nothing fails, in every language at once.
///
/// # And it is a set rather than a boolean (MJXOFF-260)
///
/// Two examples author two packages, because that is what their guide section claims. The mechanism
/// could carry one, so the second one's bytes were compared by nothing and the choice of which to
/// offer was explained in prose — the shape this whole mechanism exists to replace. A declaration is
/// a list, the halves are held to the whole list, and both harnesses loop over it.
#[test]
fn the_three_halves_offer_the_packages_the_rust_half_declares() {
    let root = repository_root();
    let rust_only = guide_examples::rust_only_examples(&root).expect("the pages parse");
    let mut failures: Vec<String> = Vec::new();
    let mut declaring = 0usize;
    let mut offering = 0usize;
    let mut packages = 0usize;

    for name in marked_examples(&root) {
        // Every Rust half declares, this one included: a rust-only example has no harness to compare
        // against, but a declaration that some halves may skip is a declaration that can be
        // forgotten. What is skipped below is the comparison against the other two, which do not
        // exist.
        let rust_path = Language::Rust.source_path(&name);
        let declared = match guide_examples::declared_packages(&read(&rust_path), &rust_path) {
            Ok(declared) => declared,
            Err(error) => {
                failures.push(format!("{error:#}"));
                continue;
            }
        };
        declaring += 1;
        packages += declared.len();
        if !declared.is_empty() {
            offering += 1;
        }
        if rust_only.contains_key(&name) {
            continue;
        }

        for language in Language::ALL {
            let relative = language.source_path(&name);
            let Ok(source) = std::fs::read_to_string(root.join(&relative)) else {
                continue;
            };
            let bound = packages_bound_by(language, &source);
            let declared_set: BTreeSet<&str> = declared.iter().map(String::as_str).collect();
            for missing in declared_set.difference(&bound) {
                failures.push(format!(
                    "{relative}: `{name}` declares it offers `{missing}` and this half binds no \
                     such package. A half that stops producing one used to turn its comparison \
                     into a skip in that binding and nowhere else."
                ));
            }
            for stray in bound.difference(&declared_set) {
                failures.push(format!(
                    "{relative}: this half binds `{stray}`, which `{rust_path}`'s \
                     `{}` line does not declare. A package no harness knows about is a package \
                     nothing compares.",
                    guide_examples::PACKAGES_DECLARATION
                ));
            }
        }
    }

    // Floors phrased as *the walk is still matching*, never as totals. Both are needed: a
    // declaration parser that matched nothing would leave `offering` at zero, and one that found
    // only the `none` declarations would leave `packages` there.
    assert!(
        declaring > 0,
        "no example declared its packages at all — the declaration parser has stopped matching"
    );
    assert!(
        packages > 0,
        "every example declared `{}` — the declaration parser has stopped reading names",
        guide_examples::NO_PACKAGES_TOKEN
    );
    assert!(
        failures.is_empty(),
        "these half(s) disagree with the packages their example declares:\n  {}",
        failures.join("\n  ")
    );
    println!(
        "{declaring} example(s) declare their packages; {offering} of them offer {packages} \
         package(s), each compared in all three languages"
    );
}

/// **`package_output_path` says the same thing the two two-package examples say when they write.**
///
/// The rule — the first declared package takes the harness's output path, a later one has its
/// suffix inserted before the extension — is stated in `xtask` and restated inside each Rust half,
/// because a `cargo` example under `mjx-ooxml` may not depend on `xtask`: the layering rule points
/// downward only and `xtask` is outside the ranked graph. A restatement that nothing compares is a
/// second copy of a decision, which is this repository's most-repaired defect, so the two are held
/// against each other here.
#[test]
fn the_output_path_rule_is_the_one_the_examples_restate() {
    let base = Path::new("/tmp/facade_guide_x.pkg");
    assert_eq!(
        guide_examples::package_output_path(base, "saved"),
        base,
        "the first declared package takes the path the harness passed"
    );
    assert_eq!(
        guide_examples::package_output_path(base, "saved_document"),
        Path::new("/tmp/facade_guide_x.document.pkg"),
        "a later one has its suffix inserted before the extension"
    );
    assert_eq!(
        guide_examples::package_output_path(Path::new("/tmp/out"), "saved_deck"),
        Path::new("/tmp/out.deck"),
        "a path with no extension still gets the suffix"
    );

    // And the restatement inside every Rust half that has one is textually the same rule. Compared
    // by behaviour rather than by text would need `xtask` inside the example, which is the edge the
    // restatement exists to avoid; compared by text, a divergence is visible in a diff.
    let root = repository_root();
    let mut restating = 0usize;
    for name in marked_examples(&root) {
        let relative = Language::Rust.source_path(&name);
        let source = read(&relative);
        let declared =
            guide_examples::declared_packages(&source, &relative).expect("the half declares");
        let needs_restatement = declared.len() > 1;
        let restates = source.contains("fn output_path_for(");
        assert_eq!(
            needs_restatement,
            restates,
            "{relative} declares {} package(s) and {} the output-path rule. A half with one \
             package writes where it always did and needs no rule; a half with two cannot do \
             without one.",
            declared.len(),
            if restates {
                "restates"
            } else {
                "does not restate"
            }
        );
        if restates {
            restating += 1;
        }
    }
    assert!(
        restating > 0,
        "no Rust half restates the output-path rule — the scan has stopped matching, and the \
         comparison above checked nothing"
    );
    println!("{restating} Rust half(s) restate the output-path rule");
}

/// The packages one half binds, in that language's own spelling.
///
/// Rust and Python bind theirs inside the region a reader sees; JavaScript exports them below it,
/// which is why an export may rename — the block keeps `savedDocument` and the harness protocol
/// keeps `saved_document`, and neither language has to write the other's casing.
fn packages_bound_by(language: Language, source: &str) -> BTreeSet<&str> {
    let mut bound = BTreeSet::new();
    match language {
        Language::Rust | Language::Python => {
            for line in source.lines() {
                let trimmed = line.trim_start();
                let rest = match language {
                    Language::Rust => trimmed.strip_prefix("let "),
                    _ => Some(trimmed),
                };
                let Some(rest) = rest else { continue };
                let Some(name) = rest.split(['=', ':', ' ']).next() else {
                    continue;
                };
                if is_package_binding(name) && rest[name.len()..].trim_start().starts_with('=') {
                    bound.insert(name);
                }
            }
        }
        Language::JavaScript => {
            for line in source.lines() {
                let Some(rest) = line.trim_start().strip_prefix("export {") else {
                    continue;
                };
                let Some(inner) = rest.split('}').next() else {
                    continue;
                };
                for entry in inner.split(',') {
                    // `savedDocument as saved_document` exports under the protocol name; a bare
                    // `saved` exports under its own.
                    let exported = entry.rsplit(" as ").next().unwrap_or(entry).trim();
                    if is_package_binding(exported) {
                        bound.insert(exported);
                    }
                }
            }
        }
    }
    bound
}

/// Whether a name is one of the package bindings this protocol reserves.
fn is_package_binding(name: &str) -> bool {
    name == guide_examples::PACKAGE_BINDING_PREFIX
        || name
            .strip_prefix(guide_examples::PACKAGE_BINDING_PREFIX)
            .is_some_and(|rest| rest.starts_with('_'))
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

// ===============================================================================================
// Every block the guide shows in one of the three languages is a marked copy (MJXOFF-256/263)
// ===============================================================================================

/// The facade guide — the pages MJXOFF-254's mechanism governs.
///
/// Scoped deliberately, and the scope is the interesting part. Three other guides and both binding
/// READMEs also hold `python` and `js` blocks; those illustrate installation and the TypeScript
/// surface rather than the facade, and requiring a tri-language runner behind each would be a rule
/// about a different thing. What they are *not* any longer is unchecked: since MJXOFF-256
/// `xtask/tests/doc_gate.rs` reads every fence rustdoc does not compile, so the paths and symbols
/// in one are resolved like any other prose.
const FACADE_GUIDE: &str = "crates/mjx-ooxml/docs/guide";

/// Every page of the facade guide, derived from the directory rather than listed.
fn facade_guide_pages() -> Vec<String> {
    let directory = repository_root().join(FACADE_GUIDE);
    let mut pages: Vec<String> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {FACADE_GUIDE}: {error}"))
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().into_owned();
            name.ends_with(".md")
                .then(|| format!("{FACADE_GUIDE}/{name}"))
        })
        .collect();
    pages.sort();
    assert!(
        !pages.is_empty(),
        "no page under {FACADE_GUIDE} — the directory walk has stopped matching"
    );
    pages
}

/// One fenced block found in a page.
struct Fence {
    /// The one-based line the opening fence sits on.
    line: usize,
    /// Its info string: `rust`, `python`, `js`, `sh`, or empty.
    info: String,
}

/// Every fenced block a page opens, in order.
///
/// A fence closes only with at least as many backticks as opened it, which is CommonMark's rule and
/// the reason this repository can document the marker syntax inside a ```` ```` ```` block that
/// itself contains ``` ``` ``` ones.
fn fences(text: &str) -> Vec<Fence> {
    let mut found = Vec::new();
    let mut open: Option<usize> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|c| *c == '`').count();
        if ticks < 3 {
            continue;
        }
        let info = trimmed[ticks..].trim();
        match open {
            Some(opened) if ticks >= opened && info.is_empty() => open = None,
            // A shorter fence, or one carrying an info string, inside a longer block: content.
            Some(_) => {}
            None => {
                open = Some(ticks);
                found.push(Fence {
                    line: index + 1,
                    info: info.to_owned(),
                });
            }
        }
    }
    found
}

/// **A block the facade guide shows in Rust, Python or JavaScript is a copy of a file a runner
/// executes** — never a snippet somebody typed into the page.
///
/// MJXOFF-254 made that true of nineteen examples. It did not make it a *rule*: a twentieth block
/// could be written straight into a page in any of the three languages, and nothing would run it,
/// compare it, or notice. MJXOFF-256 is that hole and MJXOFF-263 is the same hole restated at the
/// size it reached — thirty-four `python` and `js` blocks, whose only defence was that every one of
/// them happened to be marked.
///
/// The rule is stated over the three languages rather than over "not Rust", because `rust` blocks
/// have exactly the same property and the guide's own README claims it in prose: *"Every snippet on
/// every page here is a compiled doctest that `cargo test` runs."* A `sh` block listing three
/// `cargo run` lines is not an example of the API and is left alone.
#[test]
fn every_block_the_facade_guide_shows_in_one_of_the_three_languages_is_a_marked_copy() {
    let root = repository_root();
    let markers = guide_examples::all_markers(&root).expect("the pages parse");
    let marked: BTreeSet<(String, usize)> = markers
        .iter()
        .map(|(page, marker)| (page.clone(), marker.line))
        .collect();

    let mut considered = 0usize;
    let mut by_language: BTreeSet<String> = BTreeSet::new();
    let mut failures: Vec<String> = Vec::new();
    for page in facade_guide_pages() {
        for fence in fences(&read(&page)) {
            let Some(language) = Language::from_token(&fence.info) else {
                continue;
            };
            considered += 1;
            by_language.insert(language.to_string());
            // The block a marker owns opens on the line after it — that is how the renderer emits
            // one, and holding the fence to it is what makes "this block is generated" checkable
            // rather than "a marker appears somewhere on this page".
            if !marked.contains(&(page.clone(), fence.line - 1)) {
                failures.push(format!(
                    "{page}:{} opens a `{language}` block that no `{}` marker owns. Every block \
                     this guide shows in one of the three languages is copied out of a file a \
                     runner executes; a block written straight into the page is run by nothing, \
                     compared against nothing, and cannot go stale visibly. Add the three halves \
                     under {}, {} and {}, then `cargo run -p xtask -- guide-examples`.",
                    fence.line,
                    guide_examples::MARKER_PREFIX.trim(),
                    Language::Rust.directory(),
                    Language::Python.directory(),
                    Language::JavaScript.directory(),
                ));
            }
        }
    }

    // A floor, never a total: it says the fence scanner is still matching. Pinning it to the real
    // number would fire before the comparison above and hide the mutation meant to prove it.
    assert!(
        considered > 0,
        "no `rust`, `python` or `js` block was found anywhere under {FACADE_GUIDE} — the fence \
         scanner has stopped matching, and the comparison below would pass on nothing"
    );
    assert_eq!(
        by_language.len(),
        Language::ALL.len(),
        "the facade guide shows blocks in {} of the three languages ({}); a language the scanner \
         has stopped recognising would make every block in it unchecked while the total stayed \
         plausible",
        by_language.len(),
        by_language.into_iter().collect::<Vec<_>>().join(", ")
    );
    assert!(
        failures.is_empty(),
        "{} block(s) in the facade guide are shown in one of the three languages and copied from \
         nothing:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    println!(
        "facade guide: {considered} block(s) across {} page(s), every one a marked copy of a file \
         a runner executes",
        facade_guide_pages().len()
    );
}
