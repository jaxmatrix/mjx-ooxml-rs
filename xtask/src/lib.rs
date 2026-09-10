//! `xtask` as a library, so its own integration tests can reach the tables they are written against.
//!
//! **This crate has no guide of its own**: it is a host-only developer binary, never published,
//! and nothing may depend on it. Every prose page in this repository is listed from
//! `docs/api/README.md`, and `CONTRIBUTING.md` is where a contributor starts.
//!
//! # Why this target exists at all
//!
//! `xtask` is a host-only developer binary and everything in it used to be private to `main.rs`. The
//! validation harness (MJXOFF-122) changed that: `xtask/tests/validation_index.rs` compares the area
//! catalogue against `docs/validation/01-index.md`, and an integration test compiles into its own
//! crate — it cannot see a binary's modules. The alternative was to have the test parse the
//! binary's `--list` output, which would make a text format the contract instead of a type.
//!
//! [`codegen`] joined it with MJXOFF-224, for the same reason and no other:
//! `xtask/tests/codegen_drift.rs` asks whether the committed `mjx-ooxml-types` source is what the
//! generator produces today, and it is written against [`codegen::artefacts`],
//! [`codegen::SIMPLE_TYPE_MODULES`] and [`codegen::UNCOVERED_SCHEMAS`] — the tables themselves,
//! not a text rendering of them. Nothing else re-derives that crate, so without a test there is no
//! moment at which a generator defect stops being invisible.
//!
//! [`guide_examples`] joined it with MJXOFF-254, and for the third instance of the same reason:
//! `xtask/tests/guide_examples.rs` asks whether the code blocks committed in the guide are copies
//! of the files the three test runners execute, and it is written against [`Language`], the marker
//! constants and the extractor themselves rather than against a text rendering of them. A gate that
//! re-implemented the extraction would be comparing a second extractor to the first.
//!
//! [`Language`]: guide_examples::Language
//!
//! [`binding_surface`] joined it with MJXOFF-261, and for the fourth: two integration tests need
//! the same answer to *is this name reachable from Python? from JavaScript?* —
//! `xtask/tests/binding_projection.rs` to measure how much of each surface its suite exercises, and
//! `xtask/tests/guide_examples.rs` to hold a guide block that declares itself Rust-only to that
//! claim. Neither test can see the other's modules, and a second signature parser would disagree
//! with the first with no way to say which was wrong.
//!
//! `fuzz` and `corpus` stay private to the binary. `fuzz` must: moving it would move the campaign's
//! `#[global_allocator]` into every `xtask` test binary along with it.
//!
//! [`fixture_corpus`] joined it with MJXOFF-252, and for the fifth: `xtask/tests/fixture_provenance.rs`
//! derives the fixtures whose `docProps/app.xml` names Microsoft, and `xtask/tests/derived_rosters.rs`
//! needs the same set to recognise that suite's ledger as the whole of a population rather than as an
//! unregistered roster. A second `Package::open` and a second `<Application>` reader would be two
//! derivations of one fact with no way to say which was wrong.
//!
//! [`docs_site`] joined it with MJXOFF-281, and for the sixth: `xtask/tests/docs_site.rs` asks
//! whether generating the user guide's site from the committed sources produces one page per guide
//! page, one tab group per example, no rustdoc scaffolding and no dead intra-doc link — and the
//! output is git-ignored, so there is nothing committed for the gate to compare against. It has to
//! run the generator itself, which means reaching [`docs_site::render`] rather than a rendering of
//! it.
//!
//! [`repository_files`] joined it with MJXOFF-290, and for the seventh — this time because four
//! tests already had the same answer written four times, and it was the *wrong* answer in all four.
//! `doc_gate`, `entry_points`, `derived_rosters` and `release_versions` each carried their own
//! four-line call asking Git for a file listing, and every one of them got the **index** back: a
//! file a unit of work had just written was in none of their corpora until the commit that added
//! it existed, which is precisely the run at which a gate is worth having. One module is what lets
//! that be fixed once and be *tested* once — `xtask/tests/working_tree_corpus.rs` provokes the
//! property by writing a file it never commits.
//!
//! Nothing depends on this crate — `xtask/tests/layering.rs` asserts it — and it is excluded from
//! the cross-build matrix, so a library target here widens nothing.

pub mod binding_surface;
pub mod codegen;
pub mod docs_site;
pub mod facade_surface;
pub mod fixture_corpus;
pub mod guide_examples;
pub mod repository_files;
pub mod validation;
