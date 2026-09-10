//! The documentation gate: **a claim a document makes about this repository can now fail**
//! (MJXOFF-199).
//!
//! Until this file existed, nothing in the workspace read a single prose document. The consequence
//! is on the record in MJXOFF-88 §9 B5/B6: five documents directed a reader at a *presentation.rs*
//! that has been the directory `crates/mjx-pptx/src/presentation/` since Phase A; a **live test**'s
//! own doc comment cited a file MJXOFF-99 had deleted and described that deletion in the future
//! tense. Every one of those was found by a person reading carefully. This file is what finds the
//! next one.
//!
//! # The trap this file is written against, in its own terms
//!
//! > *A documentation gate over documents that name nothing checkable passes trivially and stays
//! > passing forever.*
//!
//! That is §7's shape — *"a gate phrased 'X is covered and green' is green precisely when X is
//! skipped"* — in its documentation form, and it is the exact failure this file exists to prevent.
//! A path-checker that only fires on paths written one particular way checks almost nothing and
//! reports green.
//!
//! Four things are done about it, and none of them is optional:
//!
//! 1. **Every check reports its counts.** A count is what distinguishes "ran" from "skipped
//!    quietly", and it is printed on success, not only on failure. Every arm that *skips* is
//!    counted too, and printed beside them.
//! 2. **Every check carries an anti-vacuity floor** stated as *the parser is still matching*,
//!    never as *the corpus is exactly this size* — MJXOFF-118's precedent
//!    (`assert!(types.len() > 100, "… the `pub use` parser has stopped matching")`) and
//!    `validation_index.rs`'s refinement of it. A floor pinned to the exact corpus size fires
//!    before the assertion it guards and hides the mutation that was supposed to prove that
//!    assertion.
//! 3. **The corpus is derived, never listed.** `xtask::repository_files::WorkingTree` is what "the
//!    documents this repository holds" means. It excludes `target/`, other worktrees under
//!    `.claude/` and every vendored tree without a hand-maintained skip list — the same reason
//!    `mjx-fixtures` exists and the same reason `CLAUDE.md` forbids a `const FIXTURES` list. A new
//!    page is inside the corpus the moment it is **written**, not the moment it is committed: until
//!    MJXOFF-290 the corpus was the Git index, so the one run of this gate that could have caught a
//!    brand-new page's claim was the run before that page existed. See that module for why.
//! 4. **The crate set is derived twice and compared in both directions** — see
//!    [`declared_members`]. This one was added after the first version of this file shipped with
//!    the hole it closes, and the hole is worth stating because it is the trap above at a
//!    granularity a total cannot see. Three of these checks are keyed by crate; each key set is
//!    built by a walk; and **a walk that loses one crate makes every claim about that crate
//!    silently unchecked while every count stays plausible.** Dropping `mjx-sml` — the largest
//!    crate here — took 1,793 item names and every `mjx_sml::…` reference out of the symbol
//!    comparison and left all four tests green; dropping `mjx-pptx` from the resolver's crate-name
//!    table took five path mentions out of 1,120 and did the same. Floors sized to catch the
//!    extractor dying altogether cannot catch it losing one crate, and *losing one crate* — a
//!    rename, a manifest edit, a parse tweak — is the failure this gate will actually meet. So the
//!    walks are held against `Cargo.toml`'s own `members` list rather than against a number.
//!
//! # What is checked
//!
//! * [`every_path_a_document_names_exists`] — every repository path named in a code span or a
//!   file-shaped markdown link, in **every `.md`, `.rs`, `.py` and `.mjs` file this tree holds**,
//!   resolves
//!   to something on disk. Markdown is read whole apart from the fences rustdoc compiles; the
//!   three source languages are read through their comments. See [`Kind`].
//! * [`every_retired_path_entry_is_still_needed`] — the escape hatch below cannot rot silently.
//! * [`every_crate_qualified_symbol_a_document_names_resolves`] — a `mjx_foo::Bar::baz` written in
//!   a code span still names something in `mjx-foo`.
//! * [`the_index_and_the_repository_agree_in_both_directions`] — `docs/api/README.md`.
//!
//! # Why the index is prose that is *checked against* a derivation, not a generated file
//!
//! MJXOFF-199 asks for an index that is "derived, not hand-listed", for the reason `CLAUDE.md`
//! gives about `const FIXTURES`: a new page must not be able to sit silently outside it. But an
//! index *generated* from the same walk a test then compares it against is
//! `validation_index.rs`'s own warning — *"an index test that passes because it compares two lists
//! generated from the same source proves nothing"* — and it would carry no description, which is
//! the only reason a reader opens an index at all.
//!
//! So the derivation is the **enforcement**, not the artefact. `docs/api/README.md` is written by
//! a person; its row set is required to equal the `.md` files of the working tree exactly, in both
//! directions. Writing a page without indexing it fails here, and indexing a page that does not
//! exist fails here. That is the property the ticket asks for, and the descriptions survive.
//!
//! # What is deliberately *not* checked, and why
//!
//! Stated here rather than left as a silent hole, because an unstated exclusion is how a gate
//! becomes vacuous without anyone deciding that it should.
//!
//! * **`CHANGELOG.md` is excluded from both claim checks.** It is a dated historical record, and
//!   each entry was true at its release — `mjx-omml`'s modules really were laid out the way 0.0.x's
//!   entry says, just as `mjx-chart` really did have an embedded-workbook writer of its own.
//!   Editing a released entry so that a gate goes green falsifies the record, and MJXOFF-88 §9
//!   twice rules "released text, leave it". What this skips is **counted and printed by both
//!   tests** — 597 path mentions and 640 symbol references at 0.0.131 — so the exclusion is
//!   measured rather than invisible.
//! * **A bare filename with no directory** — `mod.rs`, `lib.rs` — unless it resolves at the
//!   repository root. Dozens of files answer to each of those names and prose uses them as nouns,
//!   so treating one as an address would be noise. `Resolver::resolve_span` states the cost.
//! * **Paths under `target/`** — build output. Whether `target/corpus/workbook_large.xlsx` exists
//!   is a statement about what has been run, not about what the repository contains.
//! * **Paths under `References/`** — git-ignored by a standing rule of this repository and absent
//!   from the CI job that runs this test.
//! * **Markdown link targets that are rustdoc item paths.** The guides are pulled into rustdoc
//!   with `#![doc = include_str!(…)]`, so `[`Document::save`](Document::save)` is an intra-doc
//!   link, and CI's `rustdoc` job already denies `rustdoc::broken_intra_doc_links`. Re-checking
//!   them here would duplicate a stronger gate and misread every one of them as a missing file. A
//!   link target is treated as a **file** reference exactly when it has a path separator or a file
//!   extension and no `::`.
//! * **Tense.** `crates/mjx-sml/tests/shared_strings_fidelity.rs` said MJXOFF-99 *"then deletes"*
//!   the writer — future tense about something that had happened — and no parser can tell that
//!   from a sentence that is still true. It was fixed by hand under this ticket. What *is*
//!   mechanised is the narrower claim underneath it, and it is the one that generalises: a
//!   document may name a file that no longer exists **only in a block that also names the ticket
//!   that removed it** (see [`RETIRED_PATHS`]). A live claim about a deleted file then fails,
//!   while an honest historical sentence passes, and the difference is exactly the difference
//!   between the two sites §9 B5 lists.
//! * **Method resolution.** `mjx_sml::Border::left_edge` is generated by a declarative macro whose
//!   invocation lists bare identifiers, so no lexical pass can tell a generated accessor from a
//!   local binding. The subset that *is* resolved, and its boundary, is stated on
//!   [`every_crate_qualified_symbol_a_document_names_resolves`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use xtask::repository_files::WorkingTree;

// ===============================================================================================
// The corpus
// ===============================================================================================

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Every file this working tree holds, repository-relative and sorted.
///
/// Deriving the corpus rather than listing it is the whole point; see this file's header. Reading
/// the **working tree** rather than the index is MJXOFF-290, and the reason is in
/// `xtask/src/repository_files.rs`: a document's claim is wrong the moment it is written, and a
/// gate that cannot see the file until it is committed is a gate the author cannot run.
fn working_tree() -> WorkingTree {
    WorkingTree::read(&repository_root())
}

/// Which kind of file a document's prose lives in.
///
/// The repository writes prose in four languages, and until MJXOFF-256/MJXOFF-263 this gate read
/// two of them. `bindings/mjx-python/tests/guide_examples/*.py` and
/// `bindings/mjx-wasm/tests/node/guide_examples/*.mjs` name their guide page, their Rust sibling
/// and the harness that runs them **by path, in a docstring**, and every one of those paths was a
/// claim nothing checked. A comment is a document whatever file it sits in.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Kind {
    /// A markdown page: every line outside a fence rustdoc compiles is prose.
    Markdown,
    /// Rust source: `//` line comments and `/* … */` blocks.
    Rust,
    /// Python source: `#` line comments and `""" … """` docstrings.
    Python,
    /// JavaScript source: `//` line comments and `/* … */` blocks, JSDoc included.
    JavaScript,
}

impl Kind {
    /// Every kind, in the order a count is printed.
    const ALL: [Kind; 4] = [Kind::Markdown, Kind::Rust, Kind::Python, Kind::JavaScript];

    /// What a failure calls it.
    fn label(self) -> &'static str {
        match self {
            Kind::Markdown => "markdown",
            Kind::Rust => "Rust",
            Kind::Python => "Python",
            Kind::JavaScript => "JavaScript",
        }
    }

    /// The kind a tracked file's extension makes it, or `None` when the file carries no prose this
    /// gate can separate from its code.
    ///
    /// `.pyi` is deliberately absent, and it is the one exclusion here worth stating. Since
    /// MJXOFF-234 the committed stub's docstrings are *generated*:
    /// `bindings/mjx-python/tools/stub_docs.py` copies each member's `__doc__` out of the compiled
    /// module, and that `__doc__` is the `///` comment on the `#[pyclass]` in
    /// `bindings/mjx-python/src/`, which this gate already reads as [`Kind::Rust`]. Reading the
    /// stub too would check the same sentences a second time and report a defect one file away
    /// from where a person would fix it.
    fn of(file: &str) -> Option<Kind> {
        if file.ends_with(".md") {
            Some(Kind::Markdown)
        } else if file.ends_with(".rs") {
            Some(Kind::Rust)
        } else if file.ends_with(".py") {
            Some(Kind::Python)
        } else if file.ends_with(".mjs") || file.ends_with(".js") {
            Some(Kind::JavaScript)
        } else {
            None
        }
    }

    /// The smallest corpus this kind may shrink to before the walk is presumed broken.
    ///
    /// A floor, never a total: it says the corpus walk is still reaching this language, and it is
    /// far enough below every real count that it cannot fire in place of the comparison it guards.
    fn floor(self) -> usize {
        match self {
            Kind::Markdown | Kind::Rust => 40,
            // Two binding test trees and a stub. The whole population is a few dozen; ten says the
            // extension match is still finding them.
            Kind::Python | Kind::JavaScript => 10,
        }
    }
}

/// One document of the corpus, already reduced to its prose lines.
struct Document {
    /// Repository-relative path.
    path: String,
    kind: Kind,
    /// The crate directory that owns this file (`crates/mjx-sml`, `bindings/mjx-wasm`, `xtask`),
    /// or `None` for a repository-level document. A path a document names is resolved against this
    /// first, because a crate's own docs write a roundtrip suite as a bare *tests/roundtrip.rs*
    /// where the repository path is `crates/mjx-docx/tests/roundtrip.rs`.
    crate_dir: Option<String>,
    /// Every prose line, with its 1-based line number. A fenced code block is dropped **exactly
    /// when rustdoc compiles it** — see [`fence_is_a_rust_doctest`]. Every other fence is prose:
    /// a `python` or `js` block is run by its binding's harness, which exercises its calls and
    /// says nothing whatever about the paths its comments name.
    lines: Vec<(usize, String)>,
}

// ===============================================================================================
// The workspace's crate set — the second, independent derivation
// ===============================================================================================

/// One workspace member, as the **root manifest declares it**.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Member {
    /// `mjx-sml`.
    name: String,
    /// `mjx_sml` — the library name a document writes in a path.
    library: String,
    /// `crates/mjx-sml`.
    directory: String,
}

/// Every workspace member, read from `Cargo.toml`'s own `members = [ … ]`.
///
/// # Why this exists at all, and why it is a *second* derivation
///
/// Three of the four checks in this file are keyed by crate: the symbol map, the resolver's
/// crate-name table, and the index's owner column. Each is built by a walk — over the working tree,
/// over `*/Cargo.toml`, over `.rs` files — and **a walk that loses one crate makes every claim
/// about that crate silently unchecked**. That is §7's shape at a granularity the totals cannot
/// see: dropping `mjx-sml`, the largest crate in the workspace, takes 1,793 item names and every
/// `mjx_sml::…` reference out of the comparison while leaving 20 crates and ~1,200 references
/// behind — comfortably above any floor sized to catch the extractor dying altogether.
///
/// So the crate set is derived a second time, from a source the walks do not touch, and the two are
/// required to agree **in both directions** — the same shape as the index check, for the same
/// reason. A crate that falls out of a walk is then impossible rather than merely improbable.
///
/// # Why the manifest's `members` list rather than `cargo metadata`
///
/// `xtask/tests/layering.rs` reads `cargo metadata` and says why: Cargo's own resolution is the
/// authority on *dependency edges*, which manifests state only partially (inherited dependencies,
/// feature unification). **Membership is not that question.** `members = [ … ]` is a literal list,
/// it is what Cargo itself reads to answer "which crates are in this workspace", and reading it
/// here avoids either invoking Cargo from inside a Cargo test or duplicating `layering.rs`'s
/// hand-written JSON reader — and this workspace has spent fourteen deletions on not having two of
/// something.
///
/// A glob entry (`crates/*`) would make this list stop naming crates individually, so it is
/// rejected loudly rather than silently under-reporting.
fn declared_members() -> Vec<Member> {
    let manifest = std::fs::read_to_string(repository_root().join("Cargo.toml"))
        .expect("reading the workspace Cargo.toml");
    let mut members = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("members") && trimmed.contains('[') {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if trimmed.starts_with(']') {
            break;
        }
        let Some(directory) = trimmed
            .strip_prefix('"')
            .and_then(|rest| rest.split('"').next())
        else {
            continue; // a comment line inside the array
        };
        assert!(
            !directory.contains('*'),
            "Cargo.toml's `members` has the glob {directory:?}. This file reads that list as the \
             authoritative crate set, and a glob does not name crates individually — expand it, or \
             teach `declared_members` to expand it."
        );
        let name = directory.rsplit('/').next().unwrap_or(directory).to_owned();
        members.push(Member {
            library: name.replace('-', "_"),
            name,
            directory: directory.to_owned(),
        });
    }
    assert!(
        members.len() >= 15,
        "only {} member(s) were parsed out of Cargo.toml's `members` list; the parser has stopped \
         matching, and every crate-set comparison below would pass on almost nothing",
        members.len()
    );
    members
}

/// **Both directions between the declared crate set and whatever a walk actually found.**
///
/// `found` is keyed however the caller keys it — by `mjx-sml` or by `mjx_sml` — so `key` says which
/// field of a [`Member`] to compare against. `what` names the walk, so a failure says which of the
/// three lost the crate.
fn assert_every_workspace_crate_was_reached(
    found: &BTreeSet<String>,
    key: fn(&Member) -> &str,
    what: &str,
) {
    let members = declared_members();
    let expected: BTreeSet<&str> = members.iter().map(key).collect();

    let missing: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|crate_name| !found.contains(*crate_name))
        .collect();
    assert!(
        missing.is_empty(),
        "{what} reached {} of the {} crate(s) Cargo.toml declares, and lost {}: {}. Every claim a \
         document makes about {} of them stopped being checked, and no total would show it — that \
         is the exact failure this comparison exists to make impossible.",
        found.len(),
        expected.len(),
        missing.len(),
        missing.join(", "),
        missing.len()
    );

    let stray: Vec<&str> = found
        .iter()
        .map(String::as_str)
        .filter(|crate_name| !expected.contains(*crate_name))
        .collect();
    assert!(
        stray.is_empty(),
        "{what} holds {} entr(ies) that Cargo.toml's `members` list does not declare: {}. Either \
         the workspace grew a crate nobody added to `members`, or this walk is matching something \
         that is not a crate.",
        stray.len(),
        stray.join(", ")
    );
}

/// The crate directories, longest first so `bindings/mjx-python` wins over any prefix of it.
fn crate_directories(tracked: &[String]) -> Vec<String> {
    let mut directories: BTreeSet<String> = BTreeSet::new();
    for file in tracked {
        if !file.ends_with("Cargo.toml") {
            continue;
        }
        let Some(parent) = Path::new(file).parent().and_then(Path::to_str) else {
            continue;
        };
        if parent.is_empty() {
            continue; // the workspace manifest
        }
        directories.insert(parent.to_owned());
    }
    let mut directories: Vec<String> = directories.into_iter().collect();
    directories.sort_by_key(|d| std::cmp::Reverse(d.len()));
    directories
}

/// Reads the corpus: every markdown page this tree holds, and the comments of every Rust, Python
/// and JavaScript source file in it. See [`Kind`].
fn corpus(tree: &WorkingTree) -> Vec<Document> {
    println!("{}", tree.census());
    let files = tree.paths();
    let crates = crate_directories(files);
    let root = repository_root();
    let mut documents = Vec::new();
    for file in files {
        let Some(kind) = Kind::of(file) else {
            continue;
        };
        let text = std::fs::read_to_string(root.join(file))
            .unwrap_or_else(|e| panic!("reading {file}: {e}"));
        let crate_dir = crates
            .iter()
            .find(|directory| file.starts_with(&format!("{directory}/")))
            .cloned();
        documents.push(Document {
            path: file.clone(),
            kind,
            crate_dir,
            lines: prose_lines(kind, &text),
        });
    }
    documents
}

/// The tokens rustdoc recognises in a fence's info string. A fence whose info string is empty, or
/// whose every comma-separated token is one of these, is a Rust doctest: rustdoc compiles it, so
/// this gate need not read it.
///
/// Anything else — `python`, `js`, `sh`, `text`, `xml`, `ts`, `toml` — rustdoc leaves alone, and
/// until MJXOFF-256/MJXOFF-263 so did this gate. That is the hole: a `python` block written into
/// any page is *run* by nothing unless it carries a `guide-example` marker, and even a marked one
/// has its comments read by no test. A backtick inside such a block is a code span like any other.
const RUSTDOC_FENCE_TOKENS: &[&str] = &[
    "rust",
    "ignore",
    "no_run",
    "should_panic",
    "compile_fail",
    "edition2015",
    "edition2018",
    "edition2021",
    "edition2024",
];

/// Whether the fence opened by this info string is a block rustdoc compiles.
fn fence_is_a_rust_doctest(info: &str) -> bool {
    let info = info.trim();
    if info.is_empty() {
        return true;
    }
    info.split(',')
        .map(str::trim)
        .all(|token| RUSTDOC_FENCE_TOKENS.contains(&token))
}

/// A markdown fence that is currently open.
struct OpenFence {
    /// How many backticks opened it. CommonMark closes a fence only with at least as many, which
    /// is what lets a ```` ```` ```` block hold a ``` ``` ``` one — a shape this repository's own
    /// documentation of the marker syntax uses.
    ticks: usize,
    /// Whether rustdoc compiles it, and so whether its contents are dropped.
    compiled: bool,
}

/// The corpus's size, broken out by language, for a count line.
fn corpus_by_kind(documents: &[Document]) -> String {
    Kind::ALL
        .iter()
        .map(|kind| {
            format!(
                "{} {}",
                documents.iter().filter(|d| d.kind == *kind).count(),
                kind.label()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The prose lines of one file: markdown outside a fence rustdoc compiles, or the comment lines of
/// a source file in any of the three languages this repository ships.
fn prose_lines(kind: Kind, text: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut fence: Option<OpenFence> = None;
    // Inside a `/* … */` (Rust, JavaScript) or a `""" … """` (Python). Holds the delimiter that
    // will close it, because Python writes docstrings with either quote.
    let mut open_block: Option<&'static str> = None;
    for (index, raw) in text.lines().enumerate() {
        let trimmed = raw.trim_start();
        let body: String = match kind {
            Kind::Markdown => raw.to_owned(),
            Kind::Rust | Kind::JavaScript => match line_comment(trimmed, "//", &mut open_block) {
                Some(body) => body,
                None => continue,
            },
            Kind::Python => match python_comment(trimmed, &mut open_block) {
                Some(body) => body,
                None => continue,
            },
        };
        // A fence opens and closes a block in every kind — a doc comment holds fenced examples too.
        // The line itself is never prose.
        let ticks = body.trim_start().chars().take_while(|c| *c == '`').count();
        if ticks >= 3 {
            let rest = &body.trim_start()[ticks..];
            match &fence {
                Some(open) if ticks >= open.ticks && rest.trim().is_empty() => fence = None,
                // A shorter fence, or one carrying an info string, inside a longer block: content.
                Some(open) if open.compiled => continue,
                Some(_) => lines.push((index + 1, body)),
                None => {
                    fence = Some(OpenFence {
                        ticks,
                        compiled: fence_is_a_rust_doctest(rest),
                    })
                }
            }
            continue;
        }
        if fence.as_ref().is_some_and(|open| open.compiled) {
            continue;
        }
        lines.push((index + 1, body));
    }
    lines
}

/// One comment line in a language that writes `//` and `/* … */`.
///
/// Answers `None` for a line that is code. The `open_block` flag is this function's memory of a
/// `/* … */` that spans lines; a JSDoc block's leading `*` is stripped so its text reads as prose.
fn line_comment(
    trimmed: &str,
    marker: &str,
    open_block: &mut Option<&'static str>,
) -> Option<String> {
    if open_block.is_some() {
        return Some(match trimmed.split_once("*/") {
            Some((body, _)) => {
                *open_block = None;
                strip_jsdoc_margin(body).to_owned()
            }
            None => strip_jsdoc_margin(trimmed).to_owned(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix(marker) {
        return Some(rest.trim_start_matches(['/', '!']).to_owned());
    }
    if let Some(rest) = trimmed.strip_prefix("/*") {
        return Some(match rest.split_once("*/") {
            Some((body, _)) => strip_jsdoc_margin(body).to_owned(),
            None => {
                *open_block = Some("*/");
                strip_jsdoc_margin(rest).to_owned()
            }
        });
    }
    None
}

/// The `*` a JSDoc block puts down the left margin, which is decoration rather than text.
fn strip_jsdoc_margin(body: &str) -> &str {
    body.trim_start().strip_prefix('*').unwrap_or(body)
}

/// One comment line in Python: a `#` comment, or a line of a `"""` / `'''` docstring.
///
/// The docstring matters more than the `#` comment here. Every half of a guide example under
/// `bindings/mjx-python/tests/guide_examples/` opens with one, and every one of them names its
/// guide page, its Rust sibling and its harness by path.
fn python_comment(trimmed: &str, open_block: &mut Option<&'static str>) -> Option<String> {
    if let Some(quote) = *open_block {
        return Some(match trimmed.split_once(quote) {
            Some((body, _)) => {
                *open_block = None;
                body.to_owned()
            }
            None => trimmed.to_owned(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        return Some(rest.to_owned());
    }
    for quote in ["\"\"\"", "'''"] {
        // A docstring may carry a raw or formatted prefix; `r"""` opens one exactly as `"""` does.
        let opened = trimmed
            .strip_prefix(quote)
            .or_else(|| trimmed.strip_prefix('r')?.strip_prefix(quote))
            .or_else(|| trimmed.strip_prefix('f')?.strip_prefix(quote));
        if let Some(rest) = opened {
            return Some(match rest.split_once(quote) {
                Some((body, _)) => body.to_owned(),
                None => {
                    *open_block = Some(quote);
                    rest.to_owned()
                }
            });
        }
    }
    None
}

/// A maximal run of consecutive prose lines — a markdown paragraph, or one comment block.
///
/// This is the unit [`RETIRED_PATHS`] is enforced over: naming a deleted file and naming the ticket
/// that deleted it have to be near enough that a reader meeting one meets the other.
struct Block {
    first_line: usize,
    text: String,
}

fn blocks(document: &Document) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut previous: Option<usize> = None;
    for (number, line) in &document.lines {
        let continues = match (previous, document.kind) {
            // In Rust a blank `//!` line does not end the comment block, so adjacency is the rule.
            (Some(last), _) => *number == last + 1,
            (None, _) => false,
        } && !(document.kind == Kind::Markdown && line.trim().is_empty());
        if continues {
            if let Some(block) = blocks.last_mut() {
                block.text.push('\n');
                block.text.push_str(line);
            }
        } else if !(document.kind == Kind::Markdown && line.trim().is_empty()) {
            blocks.push(Block {
                first_line: *number,
                text: line.clone(),
            });
        }
        previous = Some(*number);
    }
    blocks
}

// ===============================================================================================
// Extracting what a document claims
// ===============================================================================================

/// Every single-backtick code span on one line.
///
/// Written out rather than pulled in as a regex dependency: `xtask` carries none, and
/// `validation_index.rs` already made this call for the same reason.
fn code_spans(line: &str) -> Vec<&str> {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let start = index + 1;
        let Some(offset) = line[start..].find('`') else {
            break;
        };
        let end = start + offset;
        if end > start {
            spans.push(&line[start..end]);
        }
        index = end + 1;
    }
    spans
}

/// Every `](target)` on one line.
fn link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut index = 0usize;
    while let Some(offset) = line[index..].find("](") {
        let start = index + offset + 2;
        let Some(close) = line[start..].find(')') else {
            break;
        };
        targets.push(&line[start..start + close]);
        index = start + close + 1;
    }
    targets
}

/// File extensions this repository's documents actually cite. A code span ending in one of these is
/// a file reference even without a directory separator.
///
/// This is a hand-maintained list, which is the shape `CLAUDE.md` warns about — so its blast radius
/// is worth stating rather than leaving implicit. **It is not load-bearing for an ordinary path.** A
/// citation containing `/` is recognised regardless of its extension, so the whole main corpus (369
/// distinct paths) is unaffected by an omission here. The list gates exactly two narrow cases: a
/// *bare* filename with no directory, which `Resolver::resolve_span` already restricts to names that
/// resolve at the repository root; and the `file.rs::symbol` split, where a miss makes the span read
/// as a Rust path instead and be rejected. An omission therefore under-checks a handful of bare
/// filenames — never a whole crate, which is the failure the crate-set comparison above exists for.
const FILE_EXTENSIONS: &[&str] = &[
    ".rs", ".md", ".toml", ".py", ".pyi", ".mjs", ".js", ".ts", ".json", ".sh", ".yml", ".yaml",
    ".xsd", ".xml", ".pptx", ".docx", ".xlsx", ".txt", ".sha256", ".lock", ".wasm",
];

fn has_file_extension(candidate: &str) -> bool {
    FILE_EXTENSIONS.iter().any(|e| candidate.ends_with(e))
}

/// Expands one level of `{a,b,c}` alternation, recursively.
///
/// `crates/mjx-dml/tests/{character_model,paragraph_model}.rs` is a real citation in this
/// repository, and treating it as one impossible filename would silently check nothing.
fn expand_braces(candidate: &str) -> Vec<String> {
    let Some(open) = candidate.find('{') else {
        return vec![candidate.to_owned()];
    };
    let Some(close) = candidate[open..].find('}').map(|o| open + o) else {
        return vec![candidate.to_owned()];
    };
    let mut expanded = Vec::new();
    for alternative in candidate[open + 1..close].split(',') {
        let joined = format!(
            "{}{}{}",
            &candidate[..open],
            alternative.trim(),
            &candidate[close + 1..]
        );
        expanded.extend(expand_braces(&joined));
    }
    expanded
}

/// One path a document claims exists.
struct PathClaim {
    /// The path as written, after brace expansion, anchor and line-number stripping.
    written: String,
    /// A `file.rs::symbol` suffix, if the citation named one.
    symbol: Option<String>,
}

/// Reads a code span as a repository path, or answers `None` when it is not one.
///
/// The discriminator that keeps OOXML part names out is deliberate and is what makes this check
/// mean something: `ppt/slides/slide1.xml` and `word/document.xml` are *inside a package*, not
/// inside this repository, and neither `ppt` nor `word` is a top-level entry here. So a candidate
/// is a repository path only when its first segment is a real top-level entry of the repository, or
/// when it resolves under the crate that owns the document.
fn path_claims(span: &str) -> Vec<PathClaim> {
    let span = span.trim();
    if span.is_empty() || span.contains(char::is_whitespace) {
        return Vec::new();
    }
    // `path.rs::symbol` — a citation of an item *in* a named file. Split it before anything else,
    // because the `::` would otherwise make this look like a Rust path.
    let (head, symbol) = match span.split_once("::") {
        Some((head, tail)) if has_file_extension(head) => (head, Some(tail.to_owned())),
        Some(_) => return Vec::new(), // a Rust path, not a file
        None => (span, None),
    };
    // `file.rs:126`, `file.rs:830–838` (en dash), `page.md#section`.
    let head = head.split('#').next().unwrap_or(head);
    let head = match head.find(':') {
        Some(colon) if head[colon + 1..].starts_with(|c: char| c.is_ascii_digit()) => {
            &head[..colon]
        }
        _ => head,
    };
    let head = head.trim_end_matches(['/', ',', ';']);
    if head.is_empty() || head.contains(['*', '?', '<', '>', '"', '|', '(', ')', '[', ']']) {
        return Vec::new();
    }
    // A span that *begins* with a dot is never a repository path in this repository's prose. Two
    // shapes account for all of them and neither is ours: a bare file type used as a noun
    // (`` `.xlsx` ``, `` `.pptx` `` — 60-odd of these), and an OOXML **part** reference relative to
    // another part inside a package (`` `../tables/table1.xml` ``, `` `../slideLayouts/…` ``), plus
    // the occasional elision (`` `.../mjx-docx/benches` ``). A repository path is written from the
    // repository root or from the crate that owns the document; a *link* is different, and
    // `Resolver::resolve_link` still resolves `../` relative to the document.
    if head.starts_with('.') {
        return Vec::new();
    }
    if !head.contains('/') && !has_file_extension(head) {
        return Vec::new();
    }
    expand_braces(head)
        .into_iter()
        .map(|written| PathClaim {
            written,
            symbol: symbol.clone(),
        })
        .collect()
}

/// Reads a markdown link target as a repository path, or answers `None`.
///
/// See the header: an intra-doc item path belongs to rustdoc's gate, not to this one.
fn link_claim(target: &str) -> Option<PathClaim> {
    let target = target.trim();
    if target.is_empty()
        || target.starts_with(['#', '<'])
        || target.contains("://")
        || target.starts_with("mailto:")
        || target.contains("::")
    {
        return None;
    }
    let bare = target.split('#').next().unwrap_or(target);
    if bare.is_empty() {
        return None;
    }
    // A `.html` target is a rustdoc *output* URL, not a repository file. `mjx-opc` links to
    // `../../mjx_pptx/struct.Presentation.html` by hand precisely because it does not depend on
    // `mjx-pptx` and so cannot write an intra-doc link; the page it names exists only after
    // `cargo doc` has run.
    if bare.ends_with(".html") {
        return None;
    }
    if !bare.contains('/') && !has_file_extension(bare) {
        return None;
    }
    Some(PathClaim {
        written: bare.to_owned(),
        symbol: None,
    })
}

// ===============================================================================================
// Resolving a claim
// ===============================================================================================

/// Path prefixes whose contents are not part of the repository — see the header for each reason.
const NOT_IN_THE_REPOSITORY: &[&str] = &["target/", "References/"];

/// Documents excluded from both claim checks, with the reason. Printed as a count by each test, so
/// the hole is measured rather than invisible.
const DOCUMENTS_EXCLUDED_FROM_THE_CLAIM_CHECKS: &[(&str, &str)] = &[(
    "CHANGELOG.md",
    "a dated historical record: each entry was true at its release, and rewriting a released entry \
     so a gate goes green falsifies the record (MJXOFF-88 §9, \"released text, leave it\")",
)];

/// Files this repository deliberately removed, and the ticket that removed them.
///
/// A document may name one of these **only inside a block that also names that ticket**, so that a
/// reader who meets the path meets the reason it is gone in the same breath. That single rule is
/// what separates the two sites MJXOFF-88 §9 B5 lists. `crates/mjx-sml/src/write/package.rs`,
/// `crates/mjx-sml/src/write/constants.rs` and `crates/mjx-sml/tests/package_writer.rs` all name
/// MJXOFF-99's deletion in the same block and are correct history;
/// `crates/mjx-sml/src/strings/table.rs` claimed in the present tense that a deleted gate
/// *compares* two writers, and named no ticket at all.
///
/// This is the gate's one escape hatch, and it lives here rather than as a magic comment in the
/// documents so that adding one is a visible diff in the gate. Its own liveness is checked by
/// [`every_retired_path_entry_is_still_needed`].
const RETIRED_PATHS: &[(&str, &str)] = &[
    ("crates/mjx-chart/src/workbook.rs", "MJXOFF-99"),
    ("crates/mjx-chart/tests/workbook.rs", "MJXOFF-99"),
    ("crates/mjx-chart/tests/workbook_parity.rs", "MJXOFF-99"),
];

/// Items this repository deliberately removed, and the ticket that removed them, under exactly the
/// rule [`RETIRED_PATHS`] states: a document may name one only inside a block that also names that
/// ticket.
///
/// Every entry here was **found by this gate**, not by the register: MJXOFF-88 §9 B5 lists two live
/// sites naming deleted *files* and none naming deleted *symbols*, and the symbol check found six
/// more on its first run — `mjx-sml`'s crate root, its package writer, its address module, its
/// constants, its package-writer suite and `mjx-xlsx`'s parts module all still name `mjx-chart`'s
/// deleted workbook writer. All six turned out to be honest history, so the same-block rule passes
/// them; the entry is what makes that a decision rather than an accident.
///
/// The rule applies to this block as much as to any other: MJXOFF-99 is what removed every item
/// below, and naming `mjx_chart::workbook::column_letters` here is naming `mjx_chart::workbook`,
/// because the head segment is what is matched.
const RETIRED_SYMBOLS: &[(&str, &str)] = &[
    ("mjx_chart::EmbeddedWorkbook", "MJXOFF-99"),
    ("mjx_chart::WorkbookCell", "MJXOFF-99"),
    ("mjx_chart::DEFAULT_SHEET_NAME", "MJXOFF-99"),
    ("mjx_chart::CONTENT_TYPE_WORKBOOK_PACKAGE", "MJXOFF-99"),
    ("mjx_chart::workbook", "MJXOFF-99"),
];

/// Where a claim resolves to, if anywhere.
enum Resolution {
    /// Exists on disk at this repository-relative path.
    Found(String),
    /// Not a repository path at all — an OOXML part name, a bare filename used as a noun, a
    /// foreign path. Not counted and not failed.
    NotOurs,
    /// Looks like one of ours and is not there.
    Missing,
}

/// Everything the resolver needs, read once per test rather than per claim.
struct Resolver {
    root: PathBuf,
    /// The top-level entries of the repository. A citation whose first segment is one of these is
    /// addressed from the repository root; one whose first segment is `ppt` or `word` is an OOXML
    /// part name inside a package and is none of our business.
    top_level: BTreeSet<String>,
    /// Crate directory by its bare name, so `mjx-pptx/src/presentation/` resolves the way a reader
    /// reads it.
    crates_by_name: BTreeMap<String, String>,
}

impl Resolver {
    fn new(tracked: &[String]) -> Self {
        let root = repository_root();
        let top_level = std::fs::read_dir(&root)
            .expect("reading the repository root")
            .map(|entry| {
                entry
                    .expect("a directory entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let crates_by_name = crate_directories(tracked)
            .into_iter()
            .map(|directory| {
                let name = directory
                    .rsplit('/')
                    .next()
                    .unwrap_or(&directory)
                    .to_owned();
                (name, directory)
            })
            .collect();
        Self {
            root,
            top_level,
            crates_by_name,
        }
    }

    fn exists(&self, relative: &str) -> bool {
        self.root.join(relative).exists()
    }

    /// A markdown link target. A link is an address by construction, so anything it names and
    /// cannot reach is broken — there is no "not ours" for a relative link.
    fn resolve_link(&self, target: &str, document: &Document) -> Resolution {
        let directory = Path::new(&document.path).parent().unwrap_or(Path::new(""));
        for candidate in [
            normalise(&directory.join(target)),
            normalise(Path::new(target)),
        ] {
            if self.exists(&candidate) {
                return Resolution::Found(candidate);
            }
        }
        Resolution::Missing
    }

    /// A path named in a code span. Resolution order is the order a reader reads it in: relative to
    /// the crate that owns the document, then from the repository root, then by crate name.
    fn resolve_span(&self, claim: &str, document: &Document) -> Resolution {
        if claim.starts_with('.') {
            return self.resolve_link(claim, document);
        }
        if let Some(crate_dir) = &document.crate_dir {
            let inside = format!("{crate_dir}/{claim}");
            if self.exists(&inside) {
                return Resolution::Found(inside);
            }
        }
        let first = claim.split('/').next().unwrap_or(claim);

        // `mjx-pptx/src/presentation/` — addressed by crate name, which is how the hand-off
        // documents write it and how a reader says it out loud.
        if let Some(crate_dir) = self.crates_by_name.get(first) {
            let rest = claim[first.len()..].trim_start_matches('/');
            let joined = format!("{crate_dir}/{rest}");
            return if self.exists(&joined) {
                Resolution::Found(joined)
            } else {
                Resolution::Missing
            };
        }

        if !claim.contains('/') {
            // A bare filename is not an address. `mod.rs` and `lib.rs` name dozens of files each,
            // and prose uses them as nouns; only a name that resolves at the repository root —
            // `Cargo.toml`, `PLAN.md` — is a claim about a specific file. The cost of this
            // narrowing is real and is stated rather than hidden: a stale bare filename is
            // invisible here, which is why MJXOFF-199 rewrote the two it found
            // (`presentation.rs` in `TABLES_HANDOFF.md` and `IMAGES_HANDOFF.md`) to name the
            // directory they actually live under.
            return if self.exists(claim) {
                Resolution::Found(claim.to_owned())
            } else {
                Resolution::NotOurs
            };
        }

        if self.top_level.contains(first) {
            return if self.exists(claim) {
                Resolution::Found(claim.to_owned())
            } else {
                Resolution::Missing
            };
        }

        // Not addressed from the root, not under the owning crate, not by crate name. If the first
        // segment names a directory a crate *would* have, this is a crate-relative citation that
        // has gone stale; otherwise it is an OOXML part name or something else entirely.
        let crate_shaped = matches!(
            first,
            "src" | "tests" | "docs" | "examples" | "benches" | "python" | "npm"
        );
        if crate_shaped && document.crate_dir.is_some() {
            Resolution::Missing
        } else {
            Resolution::NotOurs
        }
    }
}

/// `a/b/../c` → `a/c`, without touching the filesystem.
fn normalise(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    let mut parts: Vec<&str> = Vec::new();
    for part in text.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

// ===============================================================================================
// Check 1 — every path a document names exists
// ===============================================================================================

/// Enough documents naming enough paths that a parser which stopped matching is visible.
///
/// Stated as *the extractor is still working*, not as *the corpus is this size*: a floor pinned to
/// the measured total fires before the assertion it guards, which is exactly how
/// `validation_index.rs` found its own floors hiding the mutation meant to prove them. Measured at
/// 0.0.131: **293 documents naming 369 distinct paths in 1,111 mentions**, out of a corpus of 70
/// markdown pages and 671 Rust files.
const MINIMUM_DOCUMENTS_NAMING_A_PATH: usize = 220;
/// See [`MINIMUM_DOCUMENTS_NAMING_A_PATH`].
const MINIMUM_DISTINCT_PATHS: usize = 280;
/// See [`MINIMUM_DOCUMENTS_NAMING_A_PATH`].
const MINIMUM_PATH_MENTIONS: usize = 800;

#[test]
fn every_path_a_document_names_exists() {
    let tree = working_tree();
    let documents = corpus(&tree);
    let root = repository_root();
    let resolver = Resolver::new(tree.paths());
    let retired: BTreeMap<&str, &str> = RETIRED_PATHS.iter().copied().collect();
    let excluded: BTreeMap<&str, &str> = DOCUMENTS_EXCLUDED_FROM_THE_CLAIM_CHECKS
        .iter()
        .copied()
        .collect();

    let mut failures: Vec<String> = Vec::new();
    let mut distinct: BTreeSet<String> = BTreeSet::new();
    let mut mentions = 0usize;
    let mut documents_naming_a_path = 0usize;
    let mut symbol_citations = 0usize;
    let mut skipped_by_exclusion = 0usize;

    for document in &documents {
        let excluded_reason = excluded.get(document.path.as_str());
        let mut named_here = false;
        for block in blocks(document) {
            for line in block.text.lines() {
                let claims = code_spans(line)
                    .into_iter()
                    .flat_map(path_claims)
                    .map(|claim| (claim, false))
                    .chain(
                        link_targets(line)
                            .into_iter()
                            .filter_map(link_claim)
                            .map(|claim| (claim, true)),
                    );
                for (claim, from_link) in claims {
                    if NOT_IN_THE_REPOSITORY
                        .iter()
                        .any(|prefix| claim.written.starts_with(prefix))
                    {
                        continue;
                    }
                    if excluded_reason.is_some() {
                        skipped_by_exclusion += 1;
                        continue;
                    }
                    // A retired path is legal exactly where the block names the ticket that
                    // retired it. Counted as a mention either way: the claim was inspected.
                    if let Some(ticket) = retired.get(claim.written.as_str()) {
                        named_here = true;
                        mentions += 1;
                        distinct.insert(claim.written.clone());
                        if !block.text.contains(ticket) {
                            failures.push(format!(
                                "{}:{} names `{}`, which {} deleted, without naming {} anywhere in \
                                 the same block. A file that is gone may be named as history; it \
                                 may not be named as a live claim.",
                                document.path, block.first_line, claim.written, ticket, ticket
                            ));
                        }
                        continue;
                    }
                    let resolution = if from_link {
                        resolver.resolve_link(&claim.written, document)
                    } else {
                        resolver.resolve_span(&claim.written, document)
                    };
                    match resolution {
                        Resolution::NotOurs => {}
                        Resolution::Found(actual) => {
                            named_here = true;
                            mentions += 1;
                            distinct.insert(actual.clone());
                            if let Some(symbol) = &claim.symbol {
                                symbol_citations += 1;
                                let body =
                                    std::fs::read_to_string(root.join(&actual)).unwrap_or_default();
                                if !body.contains(symbol.as_str()) {
                                    failures.push(format!(
                                        "{}:{} cites `{}::{}`, and `{}` appears nowhere in that \
                                         file",
                                        document.path,
                                        block.first_line,
                                        claim.written,
                                        symbol,
                                        symbol
                                    ));
                                }
                            }
                        }
                        Resolution::Missing => {
                            named_here = true;
                            mentions += 1;
                            distinct.insert(claim.written.clone());
                            failures.push(format!(
                                "{}:{} names `{}`, which does not exist",
                                document.path, block.first_line, claim.written
                            ));
                        }
                    }
                }
            }
        }
        if named_here {
            documents_naming_a_path += 1;
        }
    }

    // ---- The floors, before the verdict ---------------------------------------------------------
    // The crate set first, both directions. `Resolver::crates_by_name` is what resolves
    // `mjx-pptx/src/presentation/` — the way the hand-off documents address a file — and a crate
    // missing from it does not fail: the lookup misses, nothing else matches, and the claim is
    // classified `NotOurs` and skipped. That is the same silent-skip shape the symbol check had,
    // on a different lever, so it is closed the same way and against the same second derivation.
    assert_every_workspace_crate_was_reached(
        &resolver.crates_by_name.keys().cloned().collect(),
        |member| &member.name,
        "the resolver's crate-name table",
    );
    // And the walk that decides which crate *owns* each document, which is what lets a crate's own
    // page cite a suite of its own by a bare crate-relative path. A crate missing here fails loudly rather than quietly — its
    // documents' crate-relative citations stop resolving — but it is asserted anyway, because
    // "fails loudly" was true of the resolver table too until the mutation was actually run.
    assert_every_workspace_crate_was_reached(
        &documents
            .iter()
            .filter_map(|document| document.crate_dir.clone())
            .collect(),
        |member| &member.directory,
        "the document walk's crate-ownership map",
    );
    // The third lever of the same kind, and the last one: `top_level` is what decides whether a
    // root-addressed path is *ours* at all, so an entry missing from it sends every path under that
    // directory to `NotOurs` — skipped, uncounted, exactly as a missing crate used to be. It is
    // derived from `read_dir`, and this holds it against a second derivation: every first segment
    // of every file in the corpus must be reachable from the root.
    let addressable: BTreeSet<&str> = tree
        .paths()
        .iter()
        .filter_map(|file| file.split('/').next())
        .collect();
    let unreachable: Vec<&str> = addressable
        .iter()
        .copied()
        .filter(|segment| !resolver.top_level.contains(*segment))
        .collect();
    assert!(
        unreachable.is_empty(),
        "the resolver's top-level table is missing {} entr(ies) that tracked files are addressed \
         through: {}. Every path under them would be classified `NotOurs` and skipped.",
        unreachable.len(),
        unreachable.join(", ")
    );

    // "No document names a path that is missing" is green precisely when no path was found, which
    // is this gate's own version of the failure it exists to catch.
    assert!(
        documents_naming_a_path >= MINIMUM_DOCUMENTS_NAMING_A_PATH,
        "only {documents_naming_a_path} document(s) were found to name a repository path; the \
         extractor has stopped matching and the comparison below would pass on almost nothing"
    );
    assert!(
        distinct.len() >= MINIMUM_DISTINCT_PATHS,
        "only {} distinct path(s) were extracted; the extractor has stopped matching",
        distinct.len()
    );
    assert!(
        mentions >= MINIMUM_PATH_MENTIONS,
        "only {mentions} path mention(s) were extracted; the extractor has stopped matching"
    );
    // Every corpus has to be reached. A change that made one language stop yielding prose would
    // still clear a bare total, because markdown alone is most of the count.
    for kind in Kind::ALL {
        let held = documents.iter().filter(|d| d.kind == kind).count();
        assert!(
            held >= kind.floor(),
            "only {held} {} document(s) are in the corpus; the working-tree walk is not reaching \
             them",
            kind.label()
        );
    }

    assert!(
        failures.is_empty(),
        "{} documentation claim(s) name something that is not there:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    println!(
        "paths: {} mention(s) of {} distinct path(s) across {} document(s) ({}), all present; {} \
         `file::symbol` citation(s) resolved; {} mention(s) skipped in {} excluded document(s)",
        mentions,
        distinct.len(),
        documents_naming_a_path,
        corpus_by_kind(&documents),
        symbol_citations,
        skipped_by_exclusion,
        DOCUMENTS_EXCLUDED_FROM_THE_CLAIM_CHECKS.len()
    );
}

#[test]
fn every_retired_path_entry_is_still_needed() {
    // MJXOFF-88 §9 B10's lesson turned on this gate's own escape hatch: a permission nothing uses
    // is a permission nobody will notice going wrong. An entry here is a licence to name a file
    // that does not exist, so when the last document that needed it stops naming it, the entry
    // comes out.
    let documents = corpus(&working_tree());
    for (path, ticket) in RETIRED_PATHS {
        let citing: Vec<&str> = documents
            .iter()
            .filter(|document| {
                document.path != "CHANGELOG.md"
                    && document
                        .lines
                        .iter()
                        .any(|(_, line)| code_spans(line).iter().any(|span| span.trim() == *path))
            })
            .map(|document| document.path.as_str())
            .collect();
        assert!(
            !citing.is_empty(),
            "`RETIRED_PATHS` permits `{path}` (deleted by {ticket}) and no document names it any \
             more. Delete the entry: an unused escape hatch is one nobody is watching."
        );
        println!(
            "retired: `{path}` ({ticket}) named by {}",
            citing.join(", ")
        );
    }
    assert!(
        !RETIRED_PATHS.is_empty(),
        "`RETIRED_PATHS` is empty, so the block rule above checked nothing"
    );
}

// ===============================================================================================
// Check 2 — every crate-qualified symbol a document names resolves
// ===============================================================================================

/// What one crate declares, for the purpose of checking prose against it.
#[derive(Default)]
struct CrateSymbols {
    /// Names introduced by a `pub`-visible item declaration or a `pub use` re-export. A symbol
    /// reference's **head** — the item immediately under the crate — must be one of these.
    items: BTreeSet<String>,
    /// Every identifier token in the crate's non-comment source. A reference's **later** segments
    /// are checked against this weaker set, because accessors here are generated by declarative
    /// macros from bare identifier lists and no lexical pass can tell one from a local binding.
    tokens: BTreeSet<String>,
}

/// Reads every crate's symbols out of its own sources.
fn workspace_symbols(documents: &[Document]) -> BTreeMap<String, CrateSymbols> {
    let root = repository_root();
    let mut symbols: BTreeMap<String, CrateSymbols> = BTreeMap::new();
    for document in documents {
        if document.kind != Kind::Rust {
            continue;
        }
        let Some(crate_dir) = &document.crate_dir else {
            continue;
        };
        let library = crate_dir
            .rsplit('/')
            .next()
            .unwrap_or(crate_dir)
            .replace('-', "_");
        let text = std::fs::read_to_string(root.join(&document.path)).unwrap_or_default();
        let code: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let entry = symbols.entry(library).or_default();
        collect_items(&code, &mut entry.items);
        collect_tokens(&code, &mut entry.tokens);
    }
    symbols
}

/// Every `fn|struct|enum|trait|type|const|static|mod|union NAME` a crate declares, every name a
/// `pub use` re-export brings into it, and every derive macro a proc-macro crate exports.
///
/// Visibility is deliberately **not** required. The question this check answers is *does this crate
/// still have this name* — which is what goes stale when something is renamed or deleted — and not
/// *is it reachable at exactly this path*, which re-exports make ambiguous anyway. Requiring `pub`
/// would also reject `mjx_pptx::nav` and `mjx_sml::arena` — private modules that the crates' own
/// doc comments name constantly and entirely correctly.
fn collect_items(code: &str, into: &mut BTreeSet<String>) {
    const KEYWORDS: &[&str] = &[
        "fn", "struct", "enum", "trait", "type", "const", "static", "mod", "union",
    ];
    for line in code.lines() {
        let mut rest = line.trim_start();
        if let Some(after) = rest.strip_prefix("pub") {
            rest = after.trim_start();
            // Skip a `pub(crate)` / `pub(super)` restriction.
            if let Some(open) = rest.strip_prefix('(') {
                let Some(close) = open.find(')') else {
                    continue;
                };
                rest = open[close + 1..].trim_start();
            }
        }
        let mut words = rest.split_whitespace();
        let mut word = words.next();
        while matches!(word, Some("async" | "unsafe" | "extern" | "default")) {
            word = words.next();
        }
        let Some(keyword) = word else { continue };
        if !KEYWORDS.contains(&keyword) {
            continue;
        }
        let Some(name) = words.next() else { continue };
        let name: String = name
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            into.insert(name);
        }
    }
    // A derive macro's *name* appears only inside the attribute, never as a declaration:
    // `#[proc_macro_derive(XmlAttributes, …)] pub fn derive_xml_attributes`. Four modules name
    // `mjx_derive::XmlAttributes` in prose, and without this they would all read as broken.
    let mut index = 0usize;
    while let Some(offset) = code[index..].find("proc_macro_derive(") {
        let start = index + offset + "proc_macro_derive(".len();
        let name: String = code[start..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            into.insert(name);
        }
        index = start;
    }
    // `pub use` re-exports, which may run over many lines: read from each to its `;`. A plain,
    // non-`pub` `use` is deliberately *not* read: it brings a dependency's name into scope without
    // the crate owning it, so reading those would let a document name any imported type as though
    // the importing crate declared it.
    let mut index = 0usize;
    while let Some(offset) = code[index..].find("pub use ") {
        let start = index + offset + "pub use ".len();
        let Some(end) = code[start..].find(';').map(|o| start + o) else {
            break;
        };
        for token in code[start..end].split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if !token.is_empty() && token != "as" && token != "crate" && token != "self" {
                into.insert(token.to_owned());
            }
        }
        index = end + 1;
    }
}

/// Every identifier token in the crate's code.
fn collect_tokens(code: &str, into: &mut BTreeSet<String>) {
    for token in code.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if !token.is_empty() && !token.starts_with(|c: char| c.is_ascii_digit()) {
            into.insert(token.to_owned());
        }
    }
}

/// A code span read as a Rust path, or `None`.
fn rust_path(span: &str) -> Option<Vec<&str>> {
    let span = span.trim();
    if !span.contains("::") {
        return None;
    }
    let segments: Vec<&str> = span.split("::").collect();
    if segments.len() < 2 {
        return None;
    }
    for segment in &segments {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            || segment.starts_with(|c: char| c.is_ascii_digit())
        {
            return None;
        }
    }
    Some(segments)
}

/// Measured at 0.0.131: 1,222 crate-qualified references across 214 documents. See
/// [`MINIMUM_DOCUMENTS_NAMING_A_PATH`] for why these are stated as *the extractor works*. Measured
/// at 0.0.131: **1,356 references across 318 documents**, resolved against 21 crates holding 14,302
/// declared item names.
const MINIMUM_SYMBOL_REFERENCES: usize = 900;
/// See [`MINIMUM_SYMBOL_REFERENCES`].
const MINIMUM_DOCUMENTS_NAMING_A_SYMBOL: usize = 200;
/// See [`MINIMUM_SYMBOL_REFERENCES`]. Guards the declaration parser rather than the corpus size.
const MINIMUM_DECLARED_ITEMS: usize = 8_000;
/// How many distinct crates the documents must be seen to reach. A symbol check that only ever
/// resolved against one crate would clear every count above while checking almost nothing.
const MINIMUM_CRATES_REFERENCED: usize = 12;

/// **The checkable subset, and its boundary.**
///
/// Checked: a code span of the form `crate_name::Item[::more…]`, where `crate_name` is a workspace
/// library, plus the same shape written `crate::Item…` inside that crate's own sources. `Item`
/// must be declared or re-exported by that crate; every later segment must at least appear as an
/// identifier in it.
///
/// Not checked, each for a stated reason:
///
/// * **A bare `Item` or `Item::member` with no crate.** `Workbook`, `Cell` and `Format` are
///   declared in several crates, and prose uses those words as words. Nothing distinguishes a
///   reference from a sentence, so a check over them would either be noise or would have to be
///   silenced into vacuity. These land in the same skip arm as an external crate's path, and both
///   are counted and printed — but **a workspace crate can never land there**, because that arm is
///   selected by testing the head against `Cargo.toml`'s `members` list and not against the symbol
///   map. A crate missing from the map is a named failure, not a skip.
/// * **`super::` and `self::`.** They resolve against the *module*, which a lexical pass does not
///   know. rustdoc resolves them wherever they are written as a link.
/// * **Anything with a generic argument, a lifetime, a call or whitespace.** `Option<bool>` and
///   `Cow<'a, str>` are types in prose, not paths to items.
/// * **Whether the item is reachable at exactly the written path.** Re-exports mean an item is
///   often nameable by more than one path; this check answers *does this crate still have this
///   name*, which is what goes stale when something is renamed or deleted.
#[test]
fn every_crate_qualified_symbol_a_document_names_resolves() {
    let documents = corpus(&working_tree());
    let symbols = workspace_symbols(&documents);

    // ---- The crate set, both directions, before anything is counted ------------------------------
    // A total cannot see one crate go missing. Dropping `mjx-sml` — 1,793 item names, the largest
    // crate here — leaves 20 crates and ~1,200 references, which clears every floor below while
    // taking every `mjx_sml::…` claim in every document out of the comparison entirely. So the map's
    // key set is required to equal Cargo.toml's own `members` list exactly, in both directions.
    assert_every_workspace_crate_was_reached(
        &symbols.keys().cloned().collect(),
        |member| &member.library,
        "the symbol map",
    );

    // Floored on the total rather than per crate: `mjx-derive` is a proc-macro crate whose whole
    // public surface is three `#[proc_macro_derive]` functions, and `mjx-fixtures` and
    // `mjx-allocation-counter` are smaller still, so a per-crate floor worth having for `mjx-sml`
    // would be a false failure on those three. What is floored per crate is only that the parser
    // reached it at all — the *presence* of every crate is the assertion above, not a floor.
    let declared: usize = symbols.values().map(|table| table.items.len()).sum();
    assert!(
        declared >= MINIMUM_DECLARED_ITEMS,
        "only {declared} item name(s) were parsed out of the whole workspace; the declaration \
         parser has stopped matching and every reference below would fail or pass at random"
    );
    for (library, table) in &symbols {
        assert!(
            !table.items.is_empty() && table.tokens.len() >= 20,
            "`{library}` yielded {} item name(s) and {} token(s); the source walk is not reaching it",
            table.items.len(),
            table.tokens.len()
        );
    }

    let excluded: BTreeMap<&str, &str> = DOCUMENTS_EXCLUDED_FROM_THE_CLAIM_CHECKS
        .iter()
        .copied()
        .collect();
    // Read from Cargo.toml, **not** from the symbol map: a crate that fell out of the map must
    // still be recognised as a workspace crate at the use site, so that it fails loudly there
    // instead of falling through to the "not ours" arm.
    let declared_libraries: BTreeSet<String> = declared_members()
        .into_iter()
        .map(|member| member.library)
        .collect();
    let mut failures: Vec<String> = Vec::new();
    let mut references = 0usize;
    let mut documents_naming_a_symbol = 0usize;
    let mut skipped_by_exclusion = 0usize;
    let mut skipped_crate_at_repository_root = 0usize;
    let mut skipped_module_relative = 0usize;
    let mut skipped_standard_library = 0usize;
    let mut skipped_not_a_workspace_crate = 0usize;
    let mut crates_referenced: BTreeSet<String> = BTreeSet::new();

    for document in &documents {
        let own_library = document.crate_dir.as_ref().map(|directory| {
            directory
                .rsplit('/')
                .next()
                .unwrap_or(directory)
                .replace('-', "_")
        });
        if excluded.contains_key(document.path.as_str()) {
            skipped_by_exclusion += document
                .lines
                .iter()
                .flat_map(|(_, line)| code_spans(line))
                .filter(|span| rust_path(span).is_some())
                .count();
            continue;
        }
        let mut named_here = false;
        for block in blocks(document) {
            for span in block.text.lines().flat_map(code_spans) {
                let Some(segments) = rust_path(span) else {
                    continue;
                };
                // Classify the head segment. Every arm is **counted**, so the ones that skip are
                // measured rather than invisible — a skip nobody counts is how a check of this
                // shape quietly stops checking anything.
                let (library, rest) = if segments[0] == "crate" {
                    // `crate::…` inside a crate's own sources, or inside a page under that crate's
                    // own `docs/` directory. This arm has a real reason to skip: at the repository
                    // root there is no crate for `crate` to mean.
                    let Some(library) = own_library.clone() else {
                        skipped_crate_at_repository_root += 1;
                        continue;
                    };
                    (library, &segments[1..])
                } else if declared_libraries.contains(segments[0]) {
                    (segments[0].to_owned(), &segments[1..])
                } else if matches!(segments[0], "super" | "self" | "Self") {
                    // Module-relative, and a lexical pass does not know the module. rustdoc
                    // resolves these wherever they are written as a link.
                    skipped_module_relative += 1;
                    continue;
                } else if matches!(segments[0], "std" | "core" | "alloc") {
                    skipped_standard_library += 1;
                    continue;
                } else {
                    // An external crate, or the `Type::member` form this check's stated subset
                    // excludes. **A workspace crate can never land here**: the arm above tests
                    // against Cargo.toml's own `members` list rather than against the symbol map,
                    // so a crate that fell out of the map is caught by the `unwrap_or_else` below
                    // and by the both-directions assertion at the top of this test — not silently
                    // swallowed here, which is what it used to be.
                    skipped_not_a_workspace_crate += 1;
                    continue;
                };
                let table = symbols.get(&library).unwrap_or_else(|| {
                    panic!(
                        "{}:{} names `{span}`, and `{library}` is a crate Cargo.toml declares but \
                         the symbol map has no entry for it. The source walk lost a whole crate; \
                         every claim about it would otherwise have gone unchecked.",
                        document.path, block.first_line
                    )
                });
                if rest.is_empty() {
                    continue;
                }
                references += 1;
                named_here = true;
                crates_referenced.insert(library.clone());

                // A retired item is legal exactly where the block names the ticket that retired it
                // — the same rule `RETIRED_PATHS` states, for the same reason.
                let head = format!("{library}::{}", rest[0]);
                if let Some((_, ticket)) = RETIRED_SYMBOLS
                    .iter()
                    .find(|(retired, _)| *retired == head.as_str())
                {
                    if !block.text.contains(ticket) {
                        failures.push(format!(
                            "{}:{} names `{}`, which {} removed, without naming {} anywhere in the \
                             same block. An item that is gone may be named as history; it may not \
                             be named as a live claim.",
                            document.path, block.first_line, span, ticket, ticket
                        ));
                    }
                    continue;
                }

                if !table.items.contains(rest[0]) {
                    failures.push(format!(
                        "{}:{} names `{}`, and `{}` is neither declared nor re-exported by `{}`",
                        document.path,
                        block.first_line,
                        span,
                        rest[0],
                        library.replace('_', "-")
                    ));
                    continue;
                }
                for segment in &rest[1..] {
                    if !table.tokens.contains(*segment) {
                        failures.push(format!(
                            "{}:{} names `{}`, and `{}` appears nowhere in `{}`",
                            document.path,
                            block.first_line,
                            span,
                            segment,
                            library.replace('_', "-")
                        ));
                        break;
                    }
                }
            }
        }
        if named_here {
            documents_naming_a_symbol += 1;
        }
    }

    assert!(
        references >= MINIMUM_SYMBOL_REFERENCES,
        "only {references} crate-qualified symbol reference(s) were extracted; the extractor has \
         stopped matching and the comparison below would pass on almost nothing"
    );
    assert!(
        documents_naming_a_symbol >= MINIMUM_DOCUMENTS_NAMING_A_SYMBOL,
        "only {documents_naming_a_symbol} document(s) name a crate-qualified symbol; the extractor \
         has stopped matching"
    );
    assert!(
        crates_referenced.len() >= MINIMUM_CRATES_REFERENCED,
        "references were resolved against only {} crate(s) ({}); the extractor is reaching one \
         corner of the workspace and clearing every other floor while doing it",
        crates_referenced.len(),
        crates_referenced
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        failures.is_empty(),
        "{} documentation claim(s) name a symbol that does not resolve:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    println!(
        "symbols: {references} crate-qualified reference(s) across {documents_naming_a_symbol} \
         document(s), resolved against {} of the {} crate(s) Cargo.toml declares, holding \
         {declared} declared item(s), every one found",
        crates_referenced.len(),
        declared_libraries.len()
    );
    println!(
        "symbols: skipped {skipped_not_a_workspace_crate} head(s) that are not a workspace crate, \
         {skipped_module_relative} module-relative, {skipped_standard_library} standard-library, \
         {skipped_crate_at_repository_root} `crate::` at the repository root, \
         {skipped_by_exclusion} in {} excluded document(s)",
        DOCUMENTS_EXCLUDED_FROM_THE_CLAIM_CHECKS.len()
    );
}

// ===============================================================================================
// Check 3 — the index, in both directions
// ===============================================================================================

/// The index page. Excluded from its own row set: an index that indexes itself is a row a reader
/// never follows.
const INDEX: &str = "docs/api/README.md";

#[test]
fn the_index_and_the_repository_agree_in_both_directions() {
    let root = repository_root();
    let tree = working_tree();
    println!("{}", tree.census());
    let expected: BTreeSet<String> = tree
        .paths()
        .iter()
        .filter(|file| file.ends_with(".md") && file.as_str() != INDEX)
        .cloned()
        .collect();

    let text = std::fs::read_to_string(root.join(INDEX))
        .unwrap_or_else(|e| panic!("reading {INDEX}: {e}"));
    let directory = Path::new(INDEX).parent().unwrap_or(Path::new(""));

    // A row is `| [text](target) | `crate` | description |`. The link target is what binds the row
    // to a file; the other two columns are what makes the index worth opening.
    let mut indexed: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix('|')
            .and_then(|rest| rest.strip_suffix('|'))
        else {
            continue;
        };
        let cells: Vec<&str> = inner.split('|').map(str::trim).collect();
        if cells.len() != 3 {
            continue;
        }
        let Some(target) = link_targets(cells[0]).into_iter().next() else {
            continue;
        };
        let resolved = normalise(&directory.join(target.split('#').next().unwrap_or(target)));
        if indexed
            .insert(resolved.clone(), (cells[1].to_owned(), cells[2].to_owned()))
            .is_some()
        {
            duplicates.push(resolved);
        }
    }

    // ---- The floor, before either direction -----------------------------------------------------
    // Both directions are green on an empty parse: "every indexed page exists" finds no rows, and
    // "every page is indexed" fails only if the *other* side is non-empty. So the row parser is
    // floored first, and the floor is stated over the parse rather than over the catalogue.
    assert!(
        indexed.len() >= 40,
        "only {} row(s) were parsed out of {INDEX}; the table parser is not reaching the tables, \
         and the comparison below would pass on almost nothing",
        indexed.len()
    );
    assert!(
        expected.len() >= 40,
        "only {} markdown page(s) were found in the working tree; the walk is not reaching them",
        expected.len()
    );
    assert!(
        duplicates.is_empty(),
        "{INDEX} lists the same page twice: {}",
        duplicates.join(", ")
    );

    // ---- Direction 1: every page in the repository is in the index -------------------------------
    let missing: Vec<&String> = expected
        .iter()
        .filter(|page| !indexed.contains_key(*page))
        .collect();
    assert!(
        missing.is_empty(),
        "{} page(s) are committed and not indexed — add a row to {INDEX} for each:\n  {}",
        missing.len(),
        missing
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    // ---- Direction 2: every row of the index names a page that exists ----------------------------
    let stray: Vec<&String> = indexed
        .keys()
        .filter(|page| !expected.contains(*page))
        .collect();
    assert!(
        stray.is_empty(),
        "{INDEX} has {} row(s) naming a page this repository does not track:\n  {}",
        stray.len(),
        stray
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    // ---- And every row says something ------------------------------------------------------------
    // A row with an empty description is a row that proves the file exists and tells a reader
    // nothing, which is the documentation form of a test with no assertion.
    // The owner column's vocabulary is crate-keyed too, and it fails the other way from the two
    // above: a crate missing from this set makes a row that names it *fail* rather than pass. That
    // is safe, but it would fail for the wrong reason and name the wrong culprit, so the set is
    // held to Cargo.toml's `members` list in both directions like the others.
    let crate_directories: BTreeSet<String> = crate_directories(tree.paths())
        .into_iter()
        .map(|directory| {
            directory
                .rsplit('/')
                .next()
                .unwrap_or(&directory)
                .to_owned()
        })
        .collect();
    assert_every_workspace_crate_was_reached(
        &crate_directories,
        |member| &member.name,
        "the index check's owner vocabulary",
    );
    let mut thin: Vec<String> = Vec::new();
    for (page, (owner, description)) in &indexed {
        let owner_bare = owner.trim_matches('`').trim();
        if owner_bare != "—" && !crate_directories.contains(owner_bare) {
            thin.push(format!(
                "{page}: the owner column says {owner:?}, which is neither a workspace crate nor \
                 `—`"
            ));
        }
        if description.split_whitespace().count() < 4 {
            thin.push(format!(
                "{page}: the description is {description:?}, which tells a reader nothing"
            ));
        }
    }
    assert!(thin.is_empty(), "{}", thin.join("\n  "));

    println!(
        "index: {} row(s) <-> {} tracked markdown page(s), both directions, every row owned and \
         described",
        indexed.len(),
        expected.len()
    );
}
