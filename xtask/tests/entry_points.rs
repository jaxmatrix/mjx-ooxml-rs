//! The entry points: **a number on the front page is derived, or it is absent** (MJXOFF-230).
//!
//! `xtask/tests/doc_gate.rs` made a document's *paths and symbols* able to fail. This file makes its
//! *numbers* able to fail, and it makes the front page's own job — being the one place a reader
//! lands and reaching everything from there — able to fail too.
//!
//! # The trap this file is written against, in its own terms
//!
//! > *A landing page is where counts go to die: it is the most-read and least-tested document in a
//! > repository, and exactly the place a figure is typed once and quoted for a year.*
//!
//! Phase G corrected that class of error in ten of its fourteen units. Both binding READMEs claimed
//! **257** `Deck` methods when there are 255. `README.md` claimed **twenty-six** runnable examples
//! when Cargo builds 28, and its own list omitted two of them. Its Excel table said *"the five to
//! start with"* over six rows. `PLAN.md` said the Excel guide was **thirteen** pages when it is
//! seventeen, and **six** examples when there are seven. The generated-vocabulary figures —
//! **84,107** generated lines, **85,296** crate lines, **59,512** child-order lines — were wrong
//! when they were written and had been copied into seven documents by the time anyone counted.
//! `docs/api/README.md` said **185** value classes where the guide it indexes said **186** and the
//! committed stub says 181, and **100** enumerations where the module projects 102.
//!
//! Not one of those was found by a test, because not one of them *was* a test. Every one is now
//! [`CLAIMS`]: the number in the prose is compared against a value derived from the repository, and
//! the failure message says what to write instead.
//!
//! # What is checked
//!
//! * [`every_count_an_entry_point_states_is_derived_from_the_repository`] — every row of
//!   [`CLAIMS`] renders to a sentence that occurs, exactly once, in the document that states it.
//! * [`every_number_an_entry_point_states_is_derived_or_deliberately_not_a_count`] — the sweep. A
//!   number of five or more written anywhere in the three entry-point documents must be covered by
//!   a claim or listed in [`NOT_A_COUNT`] with its reason. This is what stops a *new* count being
//!   typed into the front page tomorrow with nothing behind it.
//! * [`every_not_a_count_entry_is_still_needed`] — the escape hatch cannot rot silently.
//! * [`the_front_page_reaches_every_guide_set_in_one_hop`] — `README.md` links the index page of
//!   every guide set the repository holds, and the set is derived, never listed.
//! * [`the_front_page_names_every_runnable_example`] — the `cargo run … --example` block on the
//!   front page and the examples Cargo would build are the same set, in both directions.
//! * [`every_crate_root_points_a_reader_at_a_guide`] — every workspace member's crate-level doc
//!   comment sends a reader somewhere, whether or not that crate hosts a guide of its own.
//!
//! # What is deliberately *not* checked, and why
//!
//! Stated here rather than left as a silent hole, because an unstated exclusion is how a gate
//! becomes vacuous without anyone deciding that it should.
//!
//! * **Numbers below five.** Below five, a number in prose is a structural fact the sentence beside
//!   it makes checkable by reading — *"two crates"*, *"all three formats"*, *"the four removals"* —
//!   and a reader who doubts it can settle it from the same paragraph. At five and above a reader
//!   cannot, and **every count this phase found stale was in that range**: 257 methods, 26
//!   examples, 74 enumerations, 100 enumerations, 185 value classes, 84,107 lines, 59,512 lines,
//!   *"the five to start with"*. Sweeping the small ones too would need an exemption beside every
//!   *"the two bindings"* in the repository, and a table of fifty exemptions is where a reviewer
//!   stops reading — which is the failure this whole file is about.
//! * **Fenced code blocks, inline code spans and link targets.** `from_inches(7.5, 0.3, …)` and
//!   `docs/validation/06-the-office-pass.md` contain digits that are not claims about anything.
//! * **Table rows in `docs/api/README.md`.** Each is a one-line description of a page, and the page
//!   is what owns and checks the fact — *"the sixteen plot types"* belongs to
//!   `crates/mjx-chart/docs/guide/reading_a_chart.md`. The index's own **prose** is swept, because
//!   that is where it makes claims about the documentation set itself. A figure in a row is still
//!   gated when it is one a claim covers: [`CLAIMS`] matches the whole document, tables included.
//! * **`CHANGELOG.md`.** A dated historical record; `doc_gate.rs` excludes it for the same reason.
//!
//! # Why the numbers stay in the prose
//!
//! The alternative is to delete every figure, and it is worse. *"Twenty-eight runnable programs"*
//! tells a reader whether to expect a handful or a hundred; *"some runnable programs"* tells them
//! nothing and cannot be wrong, which is `doc_gate.rs`'s own warning — **a guide that names nothing
//! checkable passes trivially and stays passing forever.** So the rule is the one the ticket
//! states: derived, or absent. A figure nobody can derive is deleted; a figure worth printing is
//! printed *and* held to a derivation.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Every file this repository tracks, repository-relative.
///
/// Derived rather than listed, for `doc_gate.rs`'s reason: a page is inside the corpus the moment it
/// is committed. A failure to run `git` is a hard failure and never a skip — an empty corpus would
/// make every assertion below pass.
fn tracked_files() -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .arg("ls-files")
        .output()
        .expect("running `git ls-files` — every corpus here is derived from it");
    assert!(
        output.status.success(),
        "`git ls-files` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("`git ls-files` emits UTF-8");
    let files: Vec<String> = text.lines().map(str::to_owned).collect();
    assert!(
        files.len() > 500,
        "`git ls-files` returned {} paths, which is not this repository — every check below would \
         pass vacuously",
        files.len()
    );
    files
}

/// A tracked file's contents.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {relative}: {error}"))
}

/// `text` with every run of whitespace collapsed to one space, so a claim written on one line here
/// still matches a sentence a document wrapped over two.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ===============================================================================================
// The derivations
// ===============================================================================================

/// The index page of every guide set, derived from the tree.
///
/// A guide set is a `docs`/`guide` directory pair beside a crate — see
/// `crates/mjx-pptx/docs/guide/README.md` — and its index is the `README.md` in it. Ten crates host
/// one today; three more host a single page of somebody else's set (`mjx-mce`, `mjx-omml`,
/// `mjx-vml`) and are covered by the index of the set that owns them.
fn guide_set_indexes() -> Vec<String> {
    let mut sets: Vec<String> = tracked_files()
        .into_iter()
        .filter(|path| path.ends_with("/docs/guide/README.md"))
        .collect();
    sets.sort();
    assert!(
        sets.len() > 5,
        "the guide-set walk found {} indexes, so it has stopped matching",
        sets.len()
    );
    sets
}

/// The pages of one guide set, its own index aside.
fn guide_pages_beside_the_index(directory: &str) -> usize {
    let prefix = format!("{directory}/");
    let index = format!("{directory}/README.md");
    tracked_files()
        .into_iter()
        .filter(|path| path.starts_with(&prefix) && path.ends_with(".md") && *path != index)
        .count()
}

/// Every example Cargo would build, as `(crate directory, example name)`.
///
/// Cargo discovers an example as `examples/<name>.rs` or `examples/<name>/main.rs`, which is why
/// `crates/mjx-pptx/examples/support/mod.rs` is a shared module and not a twenty-ninth program.
/// Applying that rule to the tracked files answers the same question `cargo metadata` does without
/// a second JSON parser in this workspace's tests.
fn example_targets() -> Vec<(String, String)> {
    let mut examples = Vec::new();
    for path in tracked_files() {
        let Some((crate_directory, rest)) = path.split_once("/examples/") else {
            continue;
        };
        let name = if let Some(stem) = rest.strip_suffix(".rs") {
            if stem.contains('/') {
                continue; // `examples/support/mod.rs` — a module, not a target.
            }
            stem
        } else if let Some(directory) = rest.strip_suffix("/main.rs") {
            directory
        } else {
            continue;
        };
        examples.push((crate_directory.to_owned(), name.to_owned()));
    }
    examples.sort();
    assert!(
        examples.len() > 10,
        "the example walk found {} targets, so it has stopped matching",
        examples.len()
    );
    examples
}

/// The variants of `mjx_ooxml::ErrorCode` — the stable codes the facade promises.
fn error_codes() -> usize {
    let source = read("crates/mjx-ooxml/src/error.rs");
    let body = source
        .split_once("pub enum ErrorCode {")
        .expect("`ErrorCode` is declared in crates/mjx-ooxml/src/error.rs")
        .1;
    let body = body
        .split_once("\n}")
        .expect("`ErrorCode`'s declaration is closed")
        .0;
    let codes = body
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.ends_with(',')
                && !line.starts_with("///")
                && !line.starts_with("//")
                && !line.starts_with('#')
                && line
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_uppercase())
        })
        .count();
    assert!(
        codes > 3,
        "the `ErrorCode` variant walk found {codes} codes, so it has stopped matching"
    );
    codes
}

/// How the committed Python stub's classes divide.
///
/// The stub is the surface's own statement in Python and
/// `bindings/mjx-python/tests/test_stub_parity.py` holds it to the compiled module in both
/// directions, so counting it needs no heuristics: a class opens at column zero, and a member is
/// indented by exactly four.
struct StubShape {
    /// Classes with at least one member annotated as the class itself — `Center: TextAlignment`.
    /// That shape is a projected enumeration's member and nothing else has it.
    enumerations: BTreeSet<String>,
    /// `Exception` and its subclasses.
    exceptions: BTreeSet<String>,
    /// Everything that is neither, `Deck` / `Document` / `Workbook` aside.
    value_classes: usize,
    /// Public methods on each of the three handles, dunders aside.
    handle_methods: Vec<(String, usize)>,
}

/// The three handle classes, in the order the documents name them.
const HANDLES: [&str; 3] = ["Deck", "Document", "Workbook"];

/// Reads [`StubShape`] out of `bindings/mjx-python/python/mjx_ooxml/__init__.pyi`.
fn stub_shape() -> StubShape {
    let stub = read("bindings/mjx-python/python/mjx_ooxml/__init__.pyi");
    let mut enumerations = BTreeSet::new();
    let mut exceptions = BTreeSet::new();
    let mut value_classes = 0;
    let mut handle_methods: Vec<(String, usize)> = Vec::new();

    let mut class: Option<(String, bool)> = None; // (name, is an exception)
    let mut is_enumeration = false;
    let mut methods = 0usize;
    let mut classes = 0usize;

    // Closes the class just walked past, filing it under the one category it belongs to.
    let mut close = |class: &Option<(String, bool)>, is_enumeration: bool, methods: usize| {
        let Some((name, is_exception)) = class else {
            return;
        };
        if let Some(handle) = HANDLES.iter().find(|handle| *handle == name) {
            handle_methods.push(((*handle).to_owned(), methods));
        } else if is_enumeration {
            enumerations.insert(name.clone());
        } else if *is_exception {
            exceptions.insert(name.clone());
        } else {
            value_classes += 1;
        }
    };

    for line in stub.lines() {
        if let Some(rest) = line.strip_prefix("class ") {
            close(&class, is_enumeration, methods);
            let name: String = rest
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            let bases = rest.split_once('(').map_or("", |(_, bases)| bases);
            let is_exception = bases.contains("Exception") || bases.contains("Error");
            class = Some((name, is_exception));
            is_enumeration = false;
            methods = 0;
            classes += 1;
            continue;
        }
        let Some((name, _)) = &class else { continue };
        let Some(member) = line.strip_prefix("    ") else {
            continue;
        };
        if member.starts_with(' ') {
            continue; // Deeper than a member: a body, a docstring, a continuation.
        }
        if let Some(rest) = member.strip_prefix("def ") {
            if !rest.starts_with("__") {
                methods += 1;
            }
            continue;
        }
        // `Center: TextAlignment` on class `TextAlignment` is an enumeration member.
        if let Some((attribute, annotation)) = member.split_once(": ") {
            if annotation.trim() == name && !attribute.contains(' ') {
                is_enumeration = true;
            }
        }
    }
    close(&class, is_enumeration, methods);

    assert!(
        classes > 100,
        "the stub walk found {classes} classes, so it has stopped matching"
    );
    assert_eq!(
        handle_methods.len(),
        HANDLES.len(),
        "the stub walk did not find all three handle classes, so a claim about one of them would \
         be silently unchecked: found {handle_methods:?}"
    );
    handle_methods.sort_by_key(|(name, _)| {
        HANDLES
            .iter()
            .position(|handle| handle == name)
            .expect("only the three handles are recorded")
    });
    StubShape {
        enumerations,
        exceptions,
        value_classes,
        handle_methods,
    }
}

/// The payload-free enumerations `bindings/mjx-python/src/enums.rs` projects.
///
/// This is the number both bindings share: `Format` and `FormatFamily` live beside them in
/// `format.rs` and are *not* projected the same way in TypeScript, where a wasm enumeration is a
/// number and cannot carry a getter.
fn projected_enumerations() -> BTreeSet<String> {
    let source = read("bindings/mjx-python/src/enums.rs");
    let names: BTreeSet<String> = source
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("    ")?;
            if rest.starts_with(' ') || !rest.starts_with(char::is_uppercase) {
                return None;
            }
            let name: String = rest
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            let follows = rest.get(name.len()..)?.trim_start();
            (!name.is_empty() && follows.starts_with(['=', '{', ':', '('])).then_some(name)
        })
        .collect();
    assert!(
        names.len() > 50,
        "the enumeration walk found {} entries, so it has stopped matching",
        names.len()
    );
    names
}

/// The two enumerations the bindings project from `format.rs` rather than from `enums.rs`.
///
/// They are apart because a wasm enumeration is a number in JavaScript and cannot carry a getter,
/// so `Format`'s accessors are free functions there. `CLAUDE.md` states that as one of the two
/// forced differences in the TypeScript shape.
const FORMAT_ENUMERATIONS: [&str; 2] = ["Format", "FormatFamily"];

/// `wc -l` over a set of tracked files — the only measure of "lines" this repository states.
fn lines_of(paths: &[String]) -> usize {
    paths
        .iter()
        .map(|path| read(path).lines().count())
        .sum::<usize>()
}

/// The generated line count, the whole crate's line count, and the child-order table's, in that
/// order. Each is `wc -l` over the tracked `.rs` files it names.
fn generated_vocabulary_lines() -> (usize, usize, usize) {
    let tracked = tracked_files();
    let crate_sources: Vec<String> = tracked
        .iter()
        .filter(|path| path.starts_with("crates/mjx-ooxml-types/src/") && path.ends_with(".rs"))
        .cloned()
        .collect();
    let generated: Vec<String> = crate_sources
        .iter()
        .filter(|path| path.starts_with("crates/mjx-ooxml-types/src/generated/"))
        .cloned()
        .collect();
    assert!(
        generated.len() > 5 && crate_sources.len() > generated.len(),
        "the generated-source walk found {} of {} files, so it has stopped matching",
        generated.len(),
        crate_sources.len()
    );
    let child_order = lines_of(&["crates/mjx-ooxml-types/src/generated/child_order.rs".to_owned()]);
    (lines_of(&generated), lines_of(&crate_sources), child_order)
}

/// The banner every dated July-2026 hand-off carries at its head, verbatim.
const DATED_BANNER: &str = "> **Historical hand-off — this describes the repository as it stood on";

/// The hand-off documents that carry [`DATED_BANNER`].
fn dated_handoffs() -> Vec<String> {
    tracked_files()
        .into_iter()
        .filter(|path| path.starts_with("docs/") && path.ends_with(".md"))
        .filter(|path| read(path).contains(DATED_BANNER))
        .collect()
}

/// The validation checks: one `####` heading per check, across the three per-check pages.
fn validation_checks() -> usize {
    let count: usize = ["03-presentations", "04-documents", "05-workbooks"]
        .into_iter()
        .map(|page| {
            read(&format!("docs/validation/{page}.md"))
                .lines()
                .filter(|line| line.starts_with("#### "))
                .count()
        })
        .sum();
    assert!(
        count > 20,
        "the validation-check walk found {count} headings, so it has stopped matching"
    );
    count
}

// ===============================================================================================
// The claims
// ===============================================================================================

/// How a document spells the number in a claim.
#[derive(Clone, Copy, Debug)]
enum Spelling {
    /// `twenty-eight`, `seventeen`, `eleven` — capitalised when the sentence starts with it.
    Word,
    /// `113`, `255`.
    Digits,
    /// `84,128` — grouped in threes, as the generated-vocabulary figures are written.
    Grouped,
}

/// One number a document states, and what it is a number *of*.
struct Claim {
    /// The document, repository-relative.
    document: &'static str,
    /// The sentence the number sits in, with `{}` where the number goes. Whitespace is collapsed
    /// on both sides before matching, so a claim written on one line here still matches a sentence
    /// the document wrapped over two.
    sentence: &'static str,
    /// How the document spells it.
    spelling: Spelling,
    /// The value derived from the repository.
    derived: usize,
}

/// `n` as the document writes it.
///
/// A word is rendered in lower case; [`renderings`] is what a document is matched against, and it
/// offers the capitalised form too, because a sentence that opens with the number capitalises it
/// and a line that merely *starts* with it does not.
fn render(n: usize, spelling: Spelling) -> String {
    match spelling {
        Spelling::Digits => n.to_string(),
        Spelling::Grouped => {
            let digits = n.to_string();
            let mut grouped = String::new();
            for (index, character) in digits.chars().enumerate() {
                if index > 0 && (digits.len() - index).is_multiple_of(3) {
                    grouped.push(',');
                }
                grouped.push(character);
            }
            grouped
        }
        Spelling::Word => spell(n),
    }
}

/// The sentence a claim states, in every spelling a document may legitimately use: as written, and
/// with the first letter capitalised.
fn renderings(claim: &Claim) -> Vec<String> {
    let number = render(claim.derived, claim.spelling);
    let plain = one_line(&claim.sentence.replace("{}", &number));
    let mut characters = plain.chars();
    let capitalised = match characters.next() {
        Some(first) if first.is_lowercase() => {
            first.to_uppercase().collect::<String>() + characters.as_str()
        }
        _ => return vec![plain],
    };
    vec![plain, capitalised]
}

/// The English word for `n`, for the range prose in this repository writes out.
fn spell(n: usize) -> String {
    const UNITS: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    assert!(
        n < 100,
        "no document here spells {n} as a word; state it in digits instead"
    );
    if n < 20 {
        return UNITS[n].to_owned();
    }
    let tens = TENS[n / 10];
    if n.is_multiple_of(10) {
        tens.to_owned()
    } else {
        format!("{tens}-{}", UNITS[n % 10])
    }
}

/// Every number an entry-point document states about this repository, and the derivation behind it.
///
/// **The failure message is the point.** When one of these fails it names the document, the
/// sentence as it is written, and the sentence as the repository says it should read — so
/// correcting it is a copy rather than an investigation.
fn claims() -> Vec<Claim> {
    let sets = guide_set_indexes().len();
    let examples = example_targets().len();
    let codes = error_codes();
    let stub = stub_shape();
    let enumerations = projected_enumerations().len();
    let (generated, crate_lines, child_order) = generated_vocabulary_lines();
    let handoffs = dated_handoffs().len();
    let checks = validation_checks();
    let xlsx_examples = example_targets()
        .into_iter()
        .filter(|(directory, _)| directory == "crates/mjx-xlsx")
        .count();
    let deck = stub.handle_methods[0].1;
    let document = stub.handle_methods[1].1;
    let workbook = stub.handle_methods[2].1;

    vec![
        // ---- README.md, the front page ------------------------------------------------------
        Claim {
            document: "README.md",
            sentence: "below are the {} sets it indexes",
            spelling: Spelling::Word,
            derived: sets,
        },
        Claim {
            document: "README.md",
            sentence: "one `Error` carries {} stable codes",
            spelling: Spelling::Word,
            derived: codes,
        },
        Claim {
            document: "README.md",
            sentence: "{} runnable programs.",
            spelling: Spelling::Word,
            derived: examples,
        },
        // ---- PLAN.md, the roadmap -----------------------------------------------------------
        Claim {
            document: "PLAN.md",
            sentence: "`&str` part names and {} stable error codes",
            spelling: Spelling::Word,
            derived: codes,
        },
        Claim {
            document: "PLAN.md",
            sentence: "collapsing every `PptxError` into {} stable codes",
            spelling: Spelling::Word,
            derived: codes,
        },
        Claim {
            document: "PLAN.md",
            sentence: "{} checks across the three formats",
            spelling: Spelling::Digits,
            derived: checks,
        },
        Claim {
            document: "PLAN.md",
            sentence: "{} pages of compiled doctests beside their index",
            spelling: Spelling::Word,
            derived: guide_pages_beside_the_index("crates/mjx-xlsx/docs/guide"),
        },
        Claim {
            document: "PLAN.md",
            sentence: "plus {} runnable examples under",
            spelling: Spelling::Word,
            derived: xlsx_examples,
        },
        // ---- docs/api/README.md, the index ---------------------------------------------------
        Claim {
            document: "docs/api/README.md",
            sentence: "design notes are two of the {} pages rather",
            spelling: Spelling::Word,
            derived: guide_pages_beside_the_index("crates/mjx-sml/docs/guide"),
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "{} pages, written for someone who has to decide",
            spelling: Spelling::Word,
            derived: guide_pages_beside_the_index("crates/mjx-ooxml-types/docs/guide"),
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "**{} of this crate's",
            spelling: Spelling::Grouped,
            derived: generated,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "{} lines are written by `xtask/src/codegen/`",
            spelling: Spelling::Grouped,
            derived: crate_lines,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "The {} lines that say where a child belongs",
            spelling: Spelling::Grouped,
            derived: child_order,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "the same {} `Deck`,",
            spelling: Spelling::Digits,
            derived: deck,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "{} `Document` and",
            spelling: Spelling::Digits,
            derived: document,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "{} `Workbook` methods",
            spelling: Spelling::Digits,
            derived: workbook,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "the same {} value classes",
            spelling: Spelling::Digits,
            derived: stub.value_classes,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "the same {} enumerations",
            spelling: Spelling::Digits,
            derived: enumerations,
        },
        Claim {
            document: "docs/api/README.md",
            sentence: "These {} predate the Phase A module split",
            spelling: Spelling::Word,
            derived: handoffs,
        },
        // ---- The pages that quote the same figures -------------------------------------------
        //
        // A figure copied into a second document drifts from the first; these are where each of
        // the numbers above is written a second time, and they are held to the same derivation.
        Claim {
            document: "crates/mjx-ooxml-types/docs/guide/README.md",
            sentence: "{} of its",
            spelling: Spelling::Grouped,
            derived: generated,
        },
        Claim {
            document: "crates/mjx-ooxml-types/docs/guide/README.md",
            sentence: "{} lines are emitted by `xtask/src/codegen/`",
            spelling: Spelling::Grouped,
            derived: crate_lines,
        },
        Claim {
            document: "crates/mjx-ooxml-types/docs/guide/README.md",
            sentence: "you want to know why {} of these lines exist",
            spelling: Spelling::Grouped,
            derived: child_order,
        },
        Claim {
            document: "crates/mjx-ooxml-types/docs/guide/child_order.md",
            sentence: "**{} of this crate's",
            spelling: Spelling::Grouped,
            derived: child_order,
        },
        Claim {
            document: "crates/mjx-ooxml-types/docs/guide/child_order.md",
            sentence: "{} generated lines are one table**",
            spelling: Spelling::Grouped,
            derived: generated,
        },
        Claim {
            document: "crates/mjx-ooxml-types/docs/guide/regenerating.md",
            sentence: "`rustfmt` invocation of {} lines",
            spelling: Spelling::Grouped,
            derived: generated,
        },
        Claim {
            document: "crates/mjx-ooxml-types/src/guide.rs",
            sentence: "{} of the crate's",
            spelling: Spelling::Grouped,
            derived: generated,
        },
        Claim {
            document: "crates/mjx-ooxml-types/src/guide.rs",
            sentence: "{} lines are emitted by",
            spelling: Spelling::Grouped,
            derived: crate_lines,
        },
        Claim {
            document: "crates/mjx-ooxml-types/src/guide.rs",
            sentence: "The {} lines that say where a child element belongs",
            spelling: Spelling::Grouped,
            derived: child_order,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/README.md",
            sentence: "Beside the handles sit **{} value classes**",
            spelling: Spelling::Digits,
            derived: stub.value_classes,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/README.md",
            sentence: "and **{} enumerations**",
            spelling: Spelling::Digits,
            derived: enumerations,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/the_mapping_rules.md",
            sentence: "The two bindings export the same {} value classes",
            spelling: Spelling::Digits,
            derived: stub.value_classes,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/the_mapping_rules.md",
            sentence: "and the same {} enumerations;",
            spelling: Spelling::Digits,
            derived: enumerations,
        },
        Claim {
            document: "bindings/mjx-python/README.md",
            sentence: "{} methods on `Deck`",
            spelling: Spelling::Digits,
            derived: deck,
        },
        Claim {
            document: "bindings/mjx-wasm/README.md",
            sentence: "{} methods on `Deck`",
            spelling: Spelling::Digits,
            derived: deck,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/what_is_not_projected.md",
            sentence: "on the facade and {} here",
            spelling: Spelling::Digits,
            derived: deck,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/how_much_is_exercised.md",
            sentence: "already holds all {} enumerations",
            spelling: Spelling::Digits,
            derived: enumerations,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/README.md",
            sentence: "| **{}** | `bindings/mjx-python/src/deck.rs`",
            spelling: Spelling::Digits,
            derived: deck,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/README.md",
            sentence: "| **{}** | `bindings/mjx-python/src/document.rs`",
            spelling: Spelling::Digits,
            derived: document,
        },
        Claim {
            document: "bindings/mjx-python/docs/guide/README.md",
            sentence: "| **{}** | `bindings/mjx-python/src/workbook.rs`",
            spelling: Spelling::Digits,
            derived: workbook,
        },
    ]
}

/// The two independent walks over the binding surface agree about what an enumeration is.
///
/// One reads the committed Python stub — a class with a member annotated as itself — and the other
/// reads `bindings/mjx-python/src/enums.rs`, the macro that projects them. **Comparing them in both
/// directions is what makes the value-class count trustworthy**, because a value class is defined
/// here as *what is left over*: an enumeration the stub walk failed to recognise would silently
/// become a value class and the total would stay plausible. That is `doc_gate.rs`'s
/// losing-one-crate trap in the shape this file can meet it.
#[test]
fn the_two_walks_over_the_binding_surface_agree_about_its_enumerations() {
    let stub = stub_shape();
    let projected = projected_enumerations();
    let expected: BTreeSet<String> = projected
        .iter()
        .cloned()
        .chain(FORMAT_ENUMERATIONS.iter().map(|name| (*name).to_owned()))
        .collect();
    println!(
        "binding surface: {} enumerations ({} from enums.rs plus {:?}), {} exceptions, {} value \
         classes",
        stub.enumerations.len(),
        projected.len(),
        FORMAT_ENUMERATIONS,
        stub.exceptions.len(),
        stub.value_classes
    );
    let stub_only: Vec<&String> = stub.enumerations.difference(&expected).collect();
    let projection_only: Vec<&String> = expected.difference(&stub.enumerations).collect();
    assert!(
        stub_only.is_empty() && projection_only.is_empty(),
        "the stub and the projection disagree about which classes are enumerations, so the value \
         class count — which is what is left over — cannot be trusted.\n  In the stub only: \
         {stub_only:?}\n  In the projection only: {projection_only:?}"
    );
    // The exception walk keys on the base class, so a class merely *named* `…Error` that is not one
    // would be filed as an exception and lost from the value classes.
    assert!(
        stub.exceptions.contains("OoxmlError") && stub.exceptions.len() > 5,
        "the exception walk found {:?}, which is not the facade's error hierarchy",
        stub.exceptions
    );
}

/// Every count on an entry-point document is a value this repository can produce.
#[test]
fn every_count_an_entry_point_states_is_derived_from_the_repository() {
    let claims = claims();
    println!("counts derived and checked: {}", claims.len());
    let mut wrong = Vec::new();
    for claim in &claims {
        let document = one_line(&read(claim.document));
        let expected = renderings(claim);
        let occurrences: usize = expected
            .iter()
            .map(|sentence| document.matches(sentence.as_str()).count())
            .sum();
        if occurrences == 1 {
            continue;
        }
        wrong.push(format!(
            "  {}\n    should read: …{}…\n    occurrences: {occurrences} (want exactly 1)",
            claim.document, expected[0]
        ));
    }
    assert!(
        wrong.is_empty(),
        "{} count(s) a document states no longer match what this repository holds. Each line is \
         the document, then the sentence as the derivation says it should read:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

// ===============================================================================================
// The sweep
// ===============================================================================================

/// The documents a person actually lands on, and the index they land on next.
const ENTRY_POINTS: [&str; 3] = ["README.md", "PLAN.md", "docs/api/README.md"];

/// The smallest number the sweep treats as a count. See this file's header for why five.
const SMALLEST_SWEPT: usize = 5;

/// Numbers of five or more on an entry-point document that are **not** counts of anything this
/// repository can enumerate, each with the reason it is here.
///
/// An entry is `(document, the text it occurs in, why it is not a count)`. Every entry is required
/// to still occur by [`every_not_a_count_entry_is_still_needed`], so this hatch cannot rot: an
/// exemption for a sentence somebody deleted fails rather than sitting here forever, which is the
/// shape `doc_gate.rs`'s retired-path register uses.
const NOT_A_COUNT: &[(&str, &str, &str)] = &[
    (
        "README.md",
        "Part 4 Transitional and Part 2 OPC",
        "ECMA-376 part numbers, not a tally of anything",
    ),
    (
        "PLAN.md",
        "**Every phase below through Phase 6 has shipped**",
        "a phase number",
    ),
    ("PLAN.md", "and rendering (Phase 7+)", "a phase number"),
    ("PLAN.md", "- **Phase 5 — Excel slice.**", "a phase number"),
    ("PLAN.md", "- **Phase 6 — Charts + VML.**", "a phase number"),
    ("PLAN.md", "- **Phase 7+ (deferred).**", "a phase number"),
    ("PLAN.md", "and C, deferred to Phase 7.", "a phase number"),
    (
        "PLAN.md",
        "Filling it is §5 of the hand-off document",
        "a section number in another document",
    ),
    (
        "docs/api/README.md",
        "for a reader touring a thousand types",
        "a figure of speech for the size of DrawingML, not a tally: the crate's item count is \
         printed by `crates/mjx-dml/tests/serialization_ledger.rs`",
    ),
    (
        "docs/api/README.md",
        "differing in five places the target language forces",
        "a count the page it describes owns and argues one by one — \
         `bindings/mjx-python/docs/guide/the_mapping_rules.md`",
    ),
    (
        "docs/api/README.md",
        "Three of them were written in July 2026",
        "a year, and a count of the section's own rows, which `doc_gate.rs` holds to the tracked \
         corpus in both directions",
    ),
    (
        "docs/api/README.md",
        "in all five, a status line",
        "a count of the section's own rows, which `doc_gate.rs` holds to the tracked corpus in \
         both directions",
    ),
    (
        "docs/api/README.md",
        "## Historical hand-offs — July 2026",
        "a year",
    ),
];

/// The English words for five and above that this repository's prose uses, longest first so
/// `twenty-eight` is matched before `twenty`.
const NUMBER_WORDS: [(&str, usize); 21] = [
    ("seventeen", 17),
    ("thirteen", 13),
    ("fourteen", 14),
    ("nineteen", 19),
    ("thousand", 1_000),
    ("eighteen", 18),
    ("fifteen", 15),
    ("sixteen", 16),
    ("hundred", 100),
    ("seventy", 70),
    ("twelve", 12),
    ("eleven", 11),
    ("twenty", 20),
    ("thirty", 30),
    ("eighty", 80),
    ("ninety", 90),
    ("seven", 7),
    ("eight", 8),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
];

/// The words below [`SMALLEST_SWEPT`], kept separate because they are never swept but *are* the
/// tail of a hyphenated compound: `twenty-eight`'s `eight` is swept as part of the compound, and
/// `twenty-six`'s `six` must not start a second match.
const SMALL_WORDS: [&str; 5] = ["five", "six", "nine", "ten", "one"];

/// `text` with everything the sweep does not read replaced by spaces of the same length.
///
/// **Replaced, never removed.** Every offset into the result is the same offset into the document,
/// which is what lets a claim's sentence and a number found inside it be compared by position. See
/// this file's header for what is masked and why.
fn masked(text: &str, skip_table_rows: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside_fence = false;
    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let newline = if line.ends_with('\n') { "\n" } else { "" };
        if body.trim_start().starts_with("```") {
            inside_fence = !inside_fence;
            out.push_str(&" ".repeat(body.chars().count()));
            out.push_str(newline);
            continue;
        }
        if inside_fence || (skip_table_rows && body.trim_start().starts_with('|')) {
            out.push_str(&" ".repeat(body.chars().count()));
            out.push_str(newline);
            continue;
        }
        // Inline code spans, then markdown link targets. Both hold digits that claim nothing.
        let mut characters: Vec<char> = body.chars().collect();
        let mut inside_span = false;
        for character in &mut characters {
            if *character == '`' {
                inside_span = !inside_span;
                *character = ' ';
            } else if inside_span {
                *character = ' ';
            }
        }
        let mut index = 0;
        while index + 1 < characters.len() {
            if characters[index] == ']' && characters[index + 1] == '(' {
                let mut close = index + 2;
                while close < characters.len() && characters[close] != ')' {
                    close += 1;
                }
                if close < characters.len() {
                    for character in &mut characters[index + 1..=close] {
                        *character = ' ';
                    }
                    index = close;
                }
            }
            index += 1;
        }
        out.extend(characters);
        out.push_str(newline);
    }
    out
}

/// One number the sweep found, by byte offset into the masked text.
struct Found {
    /// Byte offset of the number's first character.
    offset: usize,
    /// The number as written.
    token: String,
}

/// Every occurrence of `needle` in `haystack`, as `(start, end)` byte offsets, where a single space
/// in `needle` matches any run of whitespace in `haystack`.
///
/// Offsets are into `haystack` itself, which is why the masking above replaces rather than removes:
/// a sentence a document wrapped over two lines still matches, and the position of the number
/// inside it is still the position in the document.
fn find_flexible(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    let haystack: Vec<char> = haystack.chars().collect();
    let needle: Vec<char> = needle.chars().collect();
    let mut matches = Vec::new();
    if needle.is_empty() {
        return matches;
    }
    for start in 0..haystack.len() {
        let mut here = start;
        let mut there = 0usize;
        let matched = loop {
            if there == needle.len() {
                break true;
            }
            if here >= haystack.len() {
                break false;
            }
            if needle[there] == ' ' {
                if !haystack[here].is_whitespace() {
                    break false;
                }
                while here < haystack.len() && haystack[here].is_whitespace() {
                    here += 1;
                }
                there += 1;
                continue;
            }
            if haystack[here] != needle[there] {
                break false;
            }
            here += 1;
            there += 1;
        };
        if matched {
            matches.push((start, here));
        }
    }
    matches
}

/// Every number of [`SMALLEST_SWEPT`] or more in `masked`, by offset.
fn numbers_in(masked: &str) -> Vec<Found> {
    let characters: Vec<char> = masked.chars().collect();
    let lower: Vec<char> = masked.to_lowercase().chars().collect();
    let mut found = Vec::new();
    let mut position = 0usize;
    while position < characters.len() {
        if characters[position].is_ascii_digit() {
            let start = position;
            while position < characters.len()
                && (characters[position].is_ascii_digit()
                    || (characters[position] == ','
                        && position + 1 < characters.len()
                        && characters[position + 1].is_ascii_digit()))
            {
                position += 1;
            }
            let token: String = characters[start..position].iter().collect();
            // `ECMA-376`, `SHA-256`, `abi3-py39`, `v0.1`, `2.1` — a digit run glued to a word, a
            // hyphen or a dot is part of an identifier or a version, not a tally.
            let glued_before = start > 0
                && (characters[start - 1].is_alphanumeric()
                    || matches!(characters[start - 1], '-' | '.' | '_' | '§' | '#'));
            let glued_after = position < characters.len()
                && (characters[position].is_alphabetic()
                    || matches!(characters[position], '.' | '_' | '-'));
            let value: usize = token.replace(',', "").parse().unwrap_or(0);
            if value >= SMALLEST_SWEPT && !glued_before && !glued_after {
                found.push(Found {
                    offset: start,
                    token,
                });
            }
            continue;
        }
        if !characters[position].is_alphabetic() {
            position += 1;
            continue;
        }
        // A word. Take the whole alphabetic run first, so `sixteen` cannot yield `six` and a word
        // that merely contains a number word cannot yield anything.
        let start = position;
        while position < characters.len() && characters[position].is_alphabetic() {
            position += 1;
        }
        let run: String = lower[start..position].iter().collect();
        let Some(word) = NUMBER_WORDS
            .iter()
            .map(|(word, _)| *word)
            .find(|word| run == *word)
        else {
            continue;
        };
        // A hyphenated compound: `twenty-eight`, `twenty-sixth`.
        let mut end = position;
        let mut token = word.to_owned();
        if end < characters.len() && characters[end] == '-' {
            let tail_start = end + 1;
            let mut tail_end = tail_start;
            while tail_end < characters.len() && characters[tail_end].is_alphabetic() {
                tail_end += 1;
            }
            let tail: String = lower[tail_start..tail_end].iter().collect();
            let is_number_tail = NUMBER_WORDS
                .iter()
                .map(|(word, _)| *word)
                .chain(SMALL_WORDS)
                .any(|candidate| tail == candidate || tail == format!("{candidate}th"));
            if is_number_tail {
                token = characters[start..tail_end].iter().collect();
                end = tail_end;
            }
        }
        found.push(Found {
            offset: start,
            token,
        });
        position = end.max(position);
    }
    found
}

/// The 1-based line `offset` falls on.
fn line_of(masked: &str, offset: usize) -> usize {
    masked
        .chars()
        .take(offset)
        .filter(|character| *character == '\n')
        .count()
        + 1
}

/// The line `offset` falls on, trimmed, for a failure message.
fn context_of(masked: &str, offset: usize) -> String {
    let characters: Vec<char> = masked.chars().collect();
    let start = characters[..offset]
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |index| index + 1);
    let end = characters[offset..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(characters.len(), |index| offset + index);
    characters[start..end]
        .iter()
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Every span of `masked_text` a claim or an exemption accounts for.
///
/// A number is covered when it lies inside one of these, which is a **positional** test rather than
/// a textual one: an exemption for *"Phase 7+"* covers the `7` in that phrase and nothing else on
/// the line.
fn accounted_spans(document: &str, raw: &str, claims: &[Claim]) -> Vec<(usize, usize)> {
    // Matched against the **unmasked** text, because a claim's sentence routinely contains an
    // inline code span — "one `Error` carries eleven stable codes" — that masking has blanked. The
    // mask replaces rather than removes, so an offset means the same thing in both.
    let mut spans = Vec::new();
    for (page, text, _) in NOT_A_COUNT {
        if *page == document {
            spans.extend(find_flexible(raw, &one_line(text)));
        }
    }
    for claim in claims.iter().filter(|claim| claim.document == document) {
        for rendered in renderings(claim) {
            spans.extend(find_flexible(raw, &rendered));
        }
    }
    spans
}

/// A number an entry-point document states is derived by a claim, or it is deliberately not a
/// count and says why.
#[test]
fn every_number_an_entry_point_states_is_derived_or_deliberately_not_a_count() {
    let claims = claims();
    let mut swept = 0usize;
    let mut ungated = Vec::new();
    for document in ENTRY_POINTS {
        let raw = read(document);
        let masked_text = masked(&raw, document == "docs/api/README.md");
        let spans = accounted_spans(document, &raw, &claims);
        for found in numbers_in(&masked_text) {
            swept += 1;
            let end = found.offset + found.token.chars().count();
            if spans
                .iter()
                .any(|(start, stop)| *start <= found.offset && end <= *stop)
            {
                continue;
            }
            ungated.push(format!(
                "  {document}:{}  `{}`  in: {}",
                line_of(&masked_text, found.offset),
                found.token,
                context_of(&masked_text, found.offset)
            ));
        }
    }
    println!(
        "numbers of {SMALLEST_SWEPT} or more swept across {} entry-point document(s): {swept}, \
         against {} claim(s) and {} exemption(s)",
        ENTRY_POINTS.len(),
        claims.len(),
        NOT_A_COUNT.len()
    );
    // Anti-vacuity, stated as *the sweep is still reading*, never as *the corpus is this size*.
    assert!(
        swept > 10,
        "the sweep found only {swept} numbers across {} documents, so it has stopped reading them \
         and the assertion below would pass vacuously",
        ENTRY_POINTS.len()
    );
    assert!(
        ungated.is_empty(),
        "{} number(s) on an entry-point document are neither derived by a claim nor recorded as \
         not a count. Add a `Claim` with the derivation behind it, delete the number, or add it to \
         `NOT_A_COUNT` with the reason it is not a tally:\n{}",
        ungated.len(),
        ungated.join("\n")
    );
}

/// The escape hatch cannot rot: an exemption for a sentence nobody writes any more fails.
#[test]
fn every_not_a_count_entry_is_still_needed() {
    let mut stale = Vec::new();
    for (document, text, reason) in NOT_A_COUNT {
        let haystack = one_line(&read(document));
        if !haystack.contains(&one_line(text)) {
            stale.push(format!("  {document}: {text:?} — kept because {reason}"));
        }
    }
    println!("exemptions checked: {}", NOT_A_COUNT.len());
    assert!(
        stale.is_empty(),
        "{} `NOT_A_COUNT` entr(y|ies) name text that is no longer written. Delete them — an \
         exemption nobody needs is a hole nobody notices:\n{}",
        stale.len(),
        stale.join("\n")
    );
}

// ===============================================================================================
// The landing itself
// ===============================================================================================

/// A reader who lands on `README.md` reaches every guide set from there, without a second document
/// in between.
#[test]
fn the_front_page_reaches_every_guide_set_in_one_hop() {
    let front_page = read("README.md");
    let sets = guide_set_indexes();
    println!("guide sets the front page must reach: {}", sets.len());
    let unreachable: Vec<&String> = sets
        .iter()
        .filter(|index| !front_page.contains(index.as_str()))
        .collect();
    assert!(
        unreachable.is_empty(),
        "{} guide set(s) exist that `README.md` does not link, so a reader arriving at the front \
         page cannot find them in one hop:\n  {}",
        unreachable.len(),
        unreachable
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    // The other direction: a link to a guide index that is not one of the tracked guide sets.
    let known: BTreeSet<&str> = sets.iter().map(String::as_str).collect();
    let mut dangling = Vec::new();
    for line in front_page.lines() {
        for fragment in line.split('(') {
            let Some(end) = fragment.find(')') else {
                continue;
            };
            let target = &fragment[..end];
            if target.ends_with("/docs/guide/README.md") && !known.contains(target) {
                dangling.push(target.to_owned());
            }
        }
    }
    assert!(
        dangling.is_empty(),
        "`README.md` links {} guide index page(s) that do not exist: {}",
        dangling.len(),
        dangling.join(", ")
    );
    // And the index reaches them too, so the second hop is not a dead end either.
    let index = read("docs/api/README.md");
    let missing: Vec<&String> = sets
        .iter()
        .filter(|path| !index.contains(path.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "{} guide set(s) are not linked from `docs/api/README.md`: {:?}",
        missing.len(),
        missing
    );
}

/// The front page's example list and the examples Cargo would build are the same set.
///
/// This is the check that would have caught the front page listing twenty-five commands under the
/// heading *"Twenty-six runnable programs"* while Cargo built twenty-eight.
#[test]
fn the_front_page_names_every_runnable_example() {
    let front_page = read("README.md");
    let listed: BTreeSet<(String, String)> = front_page
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("cargo run -p ")?;
            let (package, rest) = rest.split_once(' ')?;
            let rest = rest.trim_start().strip_prefix("--example ")?;
            let name = rest.split_whitespace().next()?;
            Some((package.to_owned(), name.to_owned()))
        })
        .collect();
    let built: BTreeSet<(String, String)> = example_targets()
        .into_iter()
        .map(|(directory, name)| {
            let package = directory
                .rsplit('/')
                .next()
                .expect("a crate directory has a last segment")
                .to_owned();
            (package, name)
        })
        .collect();
    println!(
        "examples: {} listed on the front page, {} Cargo would build",
        listed.len(),
        built.len()
    );
    assert!(
        listed.len() > 10,
        "the front page's example parser found {} commands, so it has stopped matching",
        listed.len()
    );
    let unlisted: Vec<String> = built
        .difference(&listed)
        .map(|(package, name)| format!("cargo run -p {package} --example {name}"))
        .collect();
    let phantom: Vec<String> = listed
        .difference(&built)
        .map(|(package, name)| format!("cargo run -p {package} --example {name}"))
        .collect();
    assert!(
        unlisted.is_empty() && phantom.is_empty(),
        "`README.md`'s example list and the examples Cargo builds disagree.\n  Built but not \
         listed ({}):\n    {}\n  Listed but not built ({}):\n    {}",
        unlisted.len(),
        unlisted.join("\n    "),
        phantom.len(),
        phantom.join("\n    ")
    );
}

/// Where a crate with no guide of its own sends a reader instead, and why it has none.
///
/// Ten crates host a guide set and three more host a single page of somebody else's; the seven
/// below host neither, and each still has to answer *"where do I read about this?"*. The entry is
/// the text its crate root must contain, so deleting the pointer fails here rather than leaving a
/// dead end nobody notices.
const CRATES_WITHOUT_A_GUIDE: &[(&str, &str, &str)] = &[
    (
        "crates/mjx-ooxml-core",
        "crates/mjx-opc/docs/guide/README.md",
        "covered by the packaging tier's guide set, which spans all four crates of the tier",
    ),
    (
        "crates/mjx-xml",
        "crates/mjx-opc/docs/guide/README.md",
        "covered by the packaging tier's guide set, which spans all four crates of the tier",
    ),
    (
        "crates/mjx-derive",
        "docs/api/README.md",
        "a proc-macro crate with no runtime surface: it emits impls, and the discipline those \
         impls reproduce is the packaging tier's",
    ),
    (
        "crates/mjx-schema-gate",
        "docs/api/README.md",
        "test-only and outside the ranked graph",
    ),
    (
        "crates/mjx-fixtures",
        "docs/api/README.md",
        "test-only and outside the ranked graph",
    ),
    (
        "crates/mjx-allocation-counter",
        "docs/api/README.md",
        "test-only and outside the ranked graph",
    ),
    (
        "xtask",
        "docs/api/README.md",
        "a host-only developer binary nothing may depend on",
    ),
];

/// Every workspace member the manifest declares.
///
/// Held against `Cargo.toml`'s own list for `doc_gate.rs`'s reason: a walk that loses one crate
/// makes every claim about that crate silently unchecked while every count stays plausible.
fn declared_members() -> Vec<String> {
    let manifest = read("Cargo.toml");
    let members = manifest
        .split_once("members = [")
        .expect("the workspace manifest declares its members")
        .1
        .split_once(']')
        .expect("the members list is closed")
        .0;
    let members: Vec<String> = members
        .split('"')
        .filter(|fragment| fragment.contains('/') || fragment == &"xtask")
        .map(str::to_owned)
        .collect();
    assert!(
        members.len() > 15,
        "the member walk found {} crates, so it has stopped matching",
        members.len()
    );
    members
}

/// Every crate root sends a reader to a guide — its own, or the one that covers it.
#[test]
fn every_crate_root_points_a_reader_at_a_guide() {
    let mut silent = Vec::new();
    let mut with_a_guide = 0usize;
    let mut deferring = 0usize;
    for member in declared_members() {
        let root = ["src/lib.rs", "src/main.rs"]
            .into_iter()
            .map(|file| format!("{member}/{file}"))
            .find(|path| repository_root().join(path).exists())
            .unwrap_or_else(|| panic!("{member} has neither a lib.rs nor a main.rs"));
        let source = read(&root);
        let header: String = source
            .lines()
            .take_while(|line| line.starts_with("//!") || line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let hosts_a_guide = source.contains("pub mod guide");
        if hosts_a_guide {
            with_a_guide += 1;
            if !header.contains("[`guide`]") && !header.contains("](guide)") {
                silent.push(format!(
                    "  {root} hosts `pub mod guide` but its crate-level docs never link it"
                ));
            }
            continue;
        }
        deferring += 1;
        let Some((_, destination, reason)) = CRATES_WITHOUT_A_GUIDE
            .iter()
            .find(|(crate_directory, _, _)| *crate_directory == member)
        else {
            silent.push(format!(
                "  {member} hosts no guide and is not in `CRATES_WITHOUT_A_GUIDE`, so nothing says \
                 where a reader of it should go"
            ));
            continue;
        };
        if !header.contains(destination) {
            silent.push(format!(
                "  {root} hosts no guide ({reason}) and its crate-level docs do not name \
                 `{destination}`"
            ));
        }
    }
    println!(
        "crate roots checked: {} hosting a guide, {} deferring to one",
        with_a_guide, deferring
    );
    assert!(
        with_a_guide > 5 && deferring > 0,
        "the crate-root walk classified {with_a_guide} hosting and {deferring} deferring, so it \
         has stopped matching"
    );
    assert!(
        silent.is_empty(),
        "{} crate root(s) leave a reader with nowhere to go:\n{}",
        silent.len(),
        silent.join("\n")
    );
    // The register cannot rot either: an entry for a crate that has since grown a guide, or that
    // no longer exists, is a claim nothing checks.
    let members = declared_members();
    let stale: Vec<&str> = CRATES_WITHOUT_A_GUIDE
        .iter()
        .map(|(crate_directory, _, _)| *crate_directory)
        .filter(|crate_directory| {
            !members.iter().any(|member| member == crate_directory)
                || repository_root()
                    .join(crate_directory)
                    .join("src/guide.rs")
                    .exists()
        })
        .collect();
    assert!(
        stale.is_empty(),
        "{} `CRATES_WITHOUT_A_GUIDE` entr(y|ies) name a crate that has a guide now, or that is no \
         longer a member: {}",
        stale.len(),
        stale.join(", ")
    );
}
