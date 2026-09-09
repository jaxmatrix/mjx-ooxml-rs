//! The committed parity ledger is derived from this workspace's suites, not written down
//! (MJXOFF-179).
//!
//! # Why this runs the binary
//!
//! `xtask` is a binary crate, so an integration test cannot reach its generator directly. It can
//! run it: `CARGO_BIN_EXE_xtask` is the binary this test was built alongside, and `ledger --check`
//! regenerates the whole document in memory and refuses if what is on disk is not exactly what the
//! tree produces. Running the real command is also the stronger gate — a helper this test called
//! instead could drift from the one a developer types.
//!
//! # ⚠ Nothing here writes into the repository
//!
//! The write path is exercised through `--out-dir`, into a temporary directory. The reasoning is
//! `xtask/tests/tokens.rs`'s and it is not repeated here beyond its conclusion: **a test may read
//! the committed artefacts, or it may write generated ones somewhere disposable; it may not write
//! the ones another test reads.** `cargo test --workspace` runs test binaries as concurrent
//! processes and no in-process lock can order them.
//!
//! # What this proves, and what the unit tests prove
//!
//! This file proves **derivation and determinism**: nobody hand-edited the document, nobody changed
//! a suite without regenerating it, and two runs produce the same bytes.
//!
//! It does not prove the *rules*. That an uncovered capability is `not-started`, that a suite which
//! asserts nothing promotes nothing, that a missing suite fails the build and that removing a suite
//! moves a row are properties of [`assess`](../src/ledger/assess.rs), and they are proved there
//! against synthetic evidence indices — which is the only way to prove them without deleting a real
//! suite from a real checkout.
//!
//! Proved by mutation: change one digit in a count inside `PARITY_LEDGER.md` and
//! [`the_committed_ledger_is_exactly_what_the_suites_produce`] goes red naming the line and both
//! spellings.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The committed artefact, relative to the root it is written under.
const LEDGER: &str = "docs/client-platform/PARITY_LEDGER.md";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

fn run_ledger(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("ledger")
        .args(arguments)
        .current_dir(workspace_root())
        .output()
        .expect("running the xtask binary")
}

/// A disposable directory, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mjx-ledger-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("a clock after 1970")
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("creating a scratch directory");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// **The drift gate.** The committed document is exactly what the suites produce.
///
/// This is the check the ticket asks for and the one `UNCOVERED_SCHEMAS` never had: without it, the
/// generator would go on existing while the file it generates quietly stopped matching the tree,
/// and nothing would say so.
#[test]
fn the_committed_ledger_is_exactly_what_the_suites_produce() {
    let output = run_ledger(&["--check"]);
    assert!(
        output.status.success(),
        "`ledger --check` failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Two runs produce an identical tree, which is what makes the artefact reviewable in a diff.
#[test]
fn the_write_path_is_deterministic_and_agrees_with_what_is_committed() {
    let first = Scratch::new("first");
    let second = Scratch::new("second");

    for scratch in [&first, &second] {
        let path = scratch.0.to_str().expect("a UTF-8 temporary path");
        let output = run_ledger(&["--out-dir", path]);
        assert!(
            output.status.success(),
            "`ledger --out-dir` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let one = std::fs::read_to_string(first.0.join(LEDGER)).expect("the first run wrote a ledger");
    let two =
        std::fs::read_to_string(second.0.join(LEDGER)).expect("the second run wrote a ledger");
    assert_eq!(one, two, "two runs of the generator disagree");

    let committed =
        std::fs::read_to_string(workspace_root().join(LEDGER)).expect("the committed ledger");
    assert_eq!(
        committed, one,
        "the committed ledger is not what the write path produces"
    );
}

/// The committed document carries the things a reader must meet before the first number.
///
/// It is checked by content rather than by existence because *"the ledger is generated and
/// committed"* is satisfied by a file nobody can read correctly. The four sentences below are the
/// ones that stop a green table being mistaken for a parity claim.
#[test]
fn the_committed_ledger_says_what_it_is_not() {
    let committed =
        std::fs::read_to_string(workspace_root().join(LEDGER)).expect("the committed ledger");

    for required in [
        "This is a ledger of what was *checked*, not of what is *true*",
        "nothing in this workspace has ever been compared against Microsoft Office",
        "**Nobody has run Microsoft Office.**",
        "**Word cannot reach pixels at all.**",
        "a change detector, not evidence about Office",
        "Anything nothing tests is `not-started`",
        "The row count is not a census.",
    ] {
        assert!(
            committed.contains(required),
            "the committed ledger no longer says: {required}"
        );
    }
}

/// Every state, including the two that are easy to leave out of a generator, appears in the key.
#[test]
fn all_five_states_are_defined_in_the_document() {
    let committed =
        std::fs::read_to_string(workspace_root().join(LEDGER)).expect("the committed ledger");
    for state in [
        "`implemented`",
        "`partial`",
        "`preserved-not-rendered`",
        "`not-started`",
        "`out-of-scope`",
    ] {
        assert!(
            committed.contains(state),
            "the committed ledger does not define {state}"
        );
    }
}

/// Every §2 exclusion is a row with a reason, including the two the inventory flags for revisiting.
///
/// This is the ticket's *"§2's exclusions are ledger rows rather than silent omissions"* checked
/// against the artefact rather than against the table that produced it.
#[test]
fn every_excluded_surface_is_a_row_with_its_reason() {
    let committed =
        std::fs::read_to_string(workspace_root().join(LEDGER)).expect("the committed ledger");
    for row in [
        "excluded-add-in-host",
        "excluded-cloud-intelligence",
        "excluded-cloud-collaboration",
        "excluded-tenant-licensing",
        "excluded-external-data-and-bi",
        "excluded-automation-runtimes",
        "excluded-speech-and-translation",
        "excluded-external-publishing",
        "excluded-help-and-community",
        "excluded-mail-merge",
        "excluded-ink",
    ] {
        assert!(
            committed.contains(row),
            "the committed ledger has no `{row}` row"
        );
    }
    // The two flagged for revisiting say so, so a later reader can reopen the decision.
    assert_eq!(
        committed.matches("**flagged for revisiting.**").count(),
        2,
        "mail merge and ink are the two exclusions the inventory flags for revisiting"
    );
}

/// The rows the epic's children recorded as partial are in the document with their reasons.
///
/// Each is derived from a `MJX-LEDGER-LIMITATION:` marker in the suite that asserts it, so this
/// test fails if a marker is deleted from a suite — which is the direction that matters. A defect
/// that stops being asserted must not quietly stop being reported.
#[test]
fn the_known_wrong_on_purpose_items_are_named_in_the_ledger() {
    let committed =
        std::fs::read_to_string(workspace_root().join(LEDGER)).expect("the committed ledger");
    for fragment in [
        "theme-styled shadow renders at 100 %",
        "dashed or dotted edge draws solid",
        "chart text is *measured* and not shaped",
        "stretchy delimiter is",
        "reported unevaluated and painted as nothing",
    ] {
        assert!(
            committed.contains(fragment),
            "the committed ledger no longer reports: {fragment}"
        );
    }
}

/// The capabilities nothing covers are present as rows and say `not-started`, rather than being
/// left out of the table.
///
/// A ledger that omitted its gaps would summarise well and be worthless, and these five are the
/// largest holes Phase R leaves behind.
#[test]
fn the_largest_gaps_are_rows_rather_than_omissions() {
    let committed =
        std::fs::read_to_string(workspace_root().join(LEDGER)).expect("the committed ledger");
    for row in [
        "word-reaches-pixels",
        "pptx-animation-and-timing",
        "excel-sparklines",
        "excel-gridlines",
        "input-and-ime",
    ] {
        let line = committed
            .lines()
            .find(|line| line.contains(&format!("`{row}`")))
            .unwrap_or_else(|| panic!("the committed ledger has no `{row}` row"));
        assert!(
            line.contains("`not-started`"),
            "`{row}` is no longer `not-started`; if that is real, this test is the place to say so: \
             {line}"
        );
    }
}

/// An unknown argument is refused rather than silently ignored, so a mistyped `--check` cannot pass
/// for a check that ran.
#[test]
fn an_unknown_argument_is_refused() {
    let output = run_ledger(&["--checkk"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage"));
}
