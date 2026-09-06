//! The committed design-token artefacts are derived, not hand-written (MJXOFF-156).
//!
//! # Why this runs the binary
//!
//! `xtask` is a binary crate, so an integration test cannot reach its generator directly. It can
//! run it: `CARGO_BIN_EXE_xtask` is the compiled binary this test was built alongside, and
//! `tokens --check` regenerates all three artefacts in memory and refuses if what is on disk is
//! not exactly what `docs/client-platform/data/tokens.json` produces. Running the real command is
//! also the stronger gate — a helper this test called instead could drift from the one a developer
//! types.
//!
//! # ⚠ No test here writes into the repository, and that is load-bearing
//!
//! **This is a hazard every generator test in this workspace can reproduce, so the reasoning is
//! written down here rather than left to be rediscovered.**
//!
//! [`the_write_path_is_deterministic_and_agrees_with_what_is_committed`] has to exercise the
//! *writing* half of the generator — `--check` alone only proves the in-memory emitters agree with
//! the committed bytes, and would go on passing if `write_plain` truncated every file it touched.
//! The obvious way to exercise it is to regenerate in place and check afterwards. **That is a data
//! race, and it was a real intermittent failure**, introduced by MJXOFF-156 and diagnosed while
//! reviewing MJXOFF-157:
//!
//! * regenerating in place truncates `ui/tokens/tokens.css` and rewrites it;
//! * [`all_three_artefacts_are_committed_and_carry_their_generated_header`] reads that same file,
//!   and Rust's harness runs the tests in one binary **concurrently on a thread pool**;
//! * so a reader can observe the file mid-write, at zero length. The symptom is a bewildering
//!   `line 1 differs / committed: <end of file>` — a *length* mismatch reported as *drift*.
//!
//! **An in-process lock does not fix this.** `crates/mjx-tokens/tests/artefacts_agree.rs` reads
//! `ui/tokens/tokens.css` and `tokens.ts` too, and it is a **different test binary** —
//! `cargo test --workspace` runs test binaries as concurrent *processes*, which no `static Mutex`
//! can order. Serialising within this file would have narrowed the window and left the hole open,
//! which is worse than leaving it obvious.
//!
//! So the writer writes into a temporary directory, through the generator's `--out-dir`, and the
//! comparison happens there. That removes the shared mutable state rather than scheduling around
//! it, and it removes a second failure mode with it: a writer that panicked mid-write used to leave
//! the working tree holding a truncated generated file, so the *next* run failed for a different
//! and more confusing reason. A temporary directory is simply discarded.
//!
//! **The rule for whoever adds the next generator test: a test may read the committed artefacts, or
//! it may write generated ones somewhere disposable. It may not write the ones another test reads.**
//!
//! # What this proves and what it does not
//!
//! It proves **derivation**: nobody hand-edited an artefact, and nobody edited the source without
//! regenerating. It does *not* prove the three agree with each other — an emitter that rendered a
//! colour one way for CSS and another for TypeScript would regenerate perfectly. That is
//! `crates/mjx-tokens/tests/artefacts_agree.rs`.
//!
//! Proved by mutation: change one hexadecimal digit in `ui/tokens/tokens.css` and
//! [`the_committed_artefacts_are_exactly_what_the_source_generates`] goes red naming the file, the
//! line and both spellings of the value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The three artefacts, relative to the root they are written under.
const ARTEFACTS: [&str; 3] = [
    "ui/tokens/tokens.css",
    "ui/tokens/tokens.ts",
    "crates/mjx-tokens/src/generated.rs",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

fn run_tokens(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("tokens")
        .args(arguments)
        .current_dir(workspace_root())
        .output()
        .expect("running the xtask binary")
}

/// A directory under the system temporary directory, removed when the test ends.
///
/// Named from the process id and a per-test counter, so two tests in this binary — and two runs of
/// it — never collide.
struct DisposableDirectory(PathBuf);

impl DisposableDirectory {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "mjx-tokens-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        // A leftover from a killed run would make this test compare against stale bytes.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("creating a temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for DisposableDirectory {
    fn drop(&mut self) {
        // Best effort: a temporary directory that outlives a panicking test is untidy, not wrong,
        // and a panic inside `drop` during unwinding would abort the process and hide the real
        // failure.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn read_artefacts(root: &Path) -> Vec<(&'static str, String)> {
    ARTEFACTS
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            let contents = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            (*relative, contents)
        })
        .collect()
}

#[test]
fn the_committed_artefacts_are_exactly_what_the_source_generates() {
    let output = run_tokens(&["--check"]);
    assert!(
        output.status.success(),
        "`cargo run -p xtask -- tokens --check` failed. The committed artefacts have drifted from \
         the source; run `cargo run -p xtask -- tokens` and commit the result.\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let reported = String::from_utf8_lossy(&output.stdout);
    assert!(
        reported.contains("3 artefacts match"),
        "`--check` must say what it checked, so a run that checked nothing is visible: {reported}"
    );
}

/// The three artefacts are committed, so a checkout has to contain them. A `--check` that passed
/// because it compared two empty strings would be exactly the failure this ticket names.
#[test]
fn all_three_artefacts_are_committed_and_carry_their_generated_header() {
    let root = workspace_root();
    for (relative, contents) in read_artefacts(&root) {
        assert!(
            contents.len() > 2_000,
            "{relative} is {} bytes, which cannot be this token set",
            contents.len()
        );
        assert!(
            contents.contains("@generated by `cargo run -p xtask -- tokens`"),
            "{relative} does not say it is generated"
        );
        assert!(
            contents.contains("--color-green"),
            "{relative} does not mention a token this platform defines"
        );
    }
}

/// The write path, exercised twice into two disposable directories.
///
/// Three things are asserted, and the first two are what `--check` on its own cannot reach:
///
/// 1. **The bytes the generator writes are the bytes that are committed.** `--check` compares the
///    committed files against a *fresh in-memory* render, so a `write_plain` that mangled what it
///    was handed would leave `--check` green and the repository wrong.
/// 2. **Two runs write identical bytes** — the determinism the committed-output doctrine rests on,
///    observed across two separate processes rather than inferred.
/// 3. `--check` against a freshly written tree agrees, which is the round trip a developer makes.
///
/// **Nothing here touches the repository.** See this module's documentation for why that matters
/// more than it looks.
#[test]
fn the_write_path_is_deterministic_and_agrees_with_what_is_committed() {
    let first_directory = DisposableDirectory::new("first");
    let second_directory = DisposableDirectory::new("second");

    for directory in [&first_directory, &second_directory] {
        let written = run_tokens(&[
            "--out-dir",
            directory
                .path()
                .to_str()
                .expect("a temporary path is UTF-8 on every platform this builds for"),
        ]);
        assert!(
            written.status.success(),
            "regeneration into {} failed: {}",
            directory.path().display(),
            String::from_utf8_lossy(&written.stderr)
        );
    }

    let committed = read_artefacts(&workspace_root());
    let first = read_artefacts(first_directory.path());
    let second = read_artefacts(second_directory.path());

    for index in 0..ARTEFACTS.len() {
        let (relative, generated) = &first[index];
        assert_eq!(
            generated, &second[index].1,
            "two runs of the generator wrote different bytes for {relative}; the committed-output \
             doctrine rests on the generator being deterministic"
        );
        assert_eq!(
            generated, &committed[index].1,
            "what the generator *writes* to {relative} is not what is committed. `--check` \
             compares the committed file against a fresh in-memory render, so it would not have \
             caught this: the fault is on the write path, not in the emitters."
        );
    }

    // And the round trip a developer actually makes: check a tree the generator just wrote.
    let checked = run_tokens(&[
        "--check",
        "--out-dir",
        first_directory
            .path()
            .to_str()
            .expect("a temporary path is UTF-8"),
    ]);
    assert!(
        checked.status.success(),
        "`--check` disagreed with a tree this generator had just written: {}{}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr),
    );
}

/// `--out-dir` really does redirect the write, which is the whole reason the tests above are safe.
///
/// Without this, `--out-dir` could silently be ignored and every assertion above would go on
/// passing while the writer quietly scribbled over the repository again — the failure this suite
/// exists to have fixed, wearing a disguise.
#[test]
fn out_dir_writes_only_under_the_directory_it_is_given() {
    let directory = DisposableDirectory::new("redirect");
    for relative in ARTEFACTS {
        assert!(
            !directory.path().join(relative).exists(),
            "the temporary directory starts empty"
        );
    }

    let written = run_tokens(&[
        "--out-dir",
        directory
            .path()
            .to_str()
            .expect("a temporary path is UTF-8"),
    ]);
    assert!(written.status.success());

    for relative in ARTEFACTS {
        let path = directory.path().join(relative);
        assert!(
            path.is_file(),
            "{} was not written under the directory `--out-dir` named",
            path.display()
        );
    }
}

#[test]
fn an_unknown_argument_is_refused_rather_than_ignored() {
    let output = run_tokens(&["--force"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown arguments"),
        "a mistyped flag must not be read as `write the files`"
    );
}

/// `--out-dir` with nothing after it must not be read as `write into the workspace`.
#[test]
fn out_dir_without_a_directory_is_refused() {
    let output = run_tokens(&["--out-dir"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("needs a directory"),
        "a truncated `--out-dir` must say what is missing rather than defaulting to the workspace"
    );
}
