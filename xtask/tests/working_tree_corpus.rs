//! **A file that has not been committed is in the corpus** (MJXOFF-290).
//!
//! Four gates in this directory sweep a corpus derived from Git rather than from a hand-maintained
//! list, and until MJXOFF-290 that corpus was the Git *index*. A file a unit of work had just
//! written was therefore in none of them, which made this repository's standing rule — *commit only
//! when `cargo build` and `cargo test --workspace` are green* — unsatisfiable for exactly the commit
//! that introduces a file: the gate that judges the file only begins running once the commit exists.
//! `xtask/src/repository_files.rs` is the answer and states the reasoning; this file is what keeps
//! it true.
//!
//! # The trap this file is written against, in its own terms
//!
//! > *A corpus that has quietly stopped including untracked files looks exactly like a clean
//! > checkout.*
//!
//! That is `xtask/tests/doc_gate.rs`'s own warning in the one shape a count cannot catch. CI checks out a clean
//! tree, so there **are** no untracked files there and every count is identical either way; a
//! developer's tree has them only sometimes. So the property cannot be observed, and has to be
//! *provoked*:
//!
//! * [`a_file_that_has_not_been_committed_is_in_the_corpus`] writes a file into the working tree,
//!   commits nothing, and requires the corpus to grow by it — then removes it again. It fails on a
//!   clean checkout and on a dirty one alike, because it makes its own evidence.
//! * [`no_gate_asks_git_what_is_committed_instead`] is the other half. A regression here is not a
//!   line changing inside `xtask/src/repository_files.rs`; it is a *fifth* gate written next year
//!   that shells out to Git for itself and gets the index back. So every `.rs` file this tree holds
//!   is swept, and the one module allowed to ask Git that question is named.

use std::path::{Path, PathBuf};

use xtask::repository_files::WorkingTree;

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// A file created in the working tree for the duration of one test, and removed however that test
/// ends.
///
/// A leaked probe is worse than a failed test: it is an untracked file in somebody's repository
/// that the very gates this file defends would then report on. So the removal is a `Drop` and not a
/// line at the end of a function that a panic would skip.
struct Probe {
    path: PathBuf,
}

impl Probe {
    /// Writes the probe, whose name is unique per process so two test binaries cannot collide.
    ///
    /// It is deliberately extension-less: `xtask/tests/doc_gate.rs` reads `.md`, `.rs`, `.py`
    /// and `.mjs`, and `xtask/tests/derived_rosters.rs` reads `.rs`, so a probe carrying one of
    /// those extensions would be a
    /// file those gates try to parse if they happen to run at the same moment. What is being proved
    /// here is that the *corpus* sees an uncommitted file; which extensions each gate then selects
    /// is that gate's own business.
    fn write(root: &Path) -> Self {
        let path = root.join(format!(".mjx-working-tree-probe-{}", std::process::id()));
        std::fs::write(
            &path,
            "Written by xtask/tests/working_tree_corpus.rs and removed by it. If this file is \
             here, that test was killed mid-run; deleting it is safe.\n",
        )
        .unwrap_or_else(|error| panic!("writing {}: {error}", path.display()));
        Self { path }
    }

    /// The probe's path as the corpus spells it: repository-relative, `/`-separated.
    fn relative(&self, root: &Path) -> String {
        self.path
            .strip_prefix(root)
            .expect("the probe is under the repository root")
            .to_string_lossy()
            .replace('\\', "/")
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// **The proof.** A file that exists and has never been committed is in the corpus.
#[test]
fn a_file_that_has_not_been_committed_is_in_the_corpus() {
    let root = repository_root();

    let before = WorkingTree::read(&root);
    let probe = Probe::write(&root);
    let name = probe.relative(&root);
    let after = WorkingTree::read(&root);

    // The probe is untracked by construction — nothing here ever runs `git add` — so if it is in
    // the corpus at all, the corpus is reading the working tree.
    assert!(
        !before.paths().contains(&name),
        "`{name}` was already in the corpus before this test wrote it, so the comparison below \
         would prove nothing; a previous run was killed and left its probe behind"
    );
    assert!(
        after.paths().contains(&name),
        "`{name}` exists on disk, is untracked and is not ignored, and the corpus does not hold \
         it. Every gate deriving its corpus from `xtask::repository_files` is therefore blind to a \
         file until the commit that adds it — which is MJXOFF-290, and the whole reason this test \
         exists.\n\
         {}",
        after.census()
    );
    assert!(
        after.untracked() > before.untracked(),
        "the corpus grew by the probe but its untracked count did not, so the census a gate prints \
         is no longer describing the corpus it sweeps: {} then {}",
        before.census(),
        after.census()
    );
    assert_eq!(
        after.tracked(),
        before.tracked(),
        "writing an untracked file changed the tracked count, which cannot be true and means the \
         two halves of the corpus are no longer being told apart"
    );

    drop(probe);

    // And the corpus follows the tree back down again, so a gate cannot be reading a stale answer.
    let restored = WorkingTree::read(&root);
    assert!(
        !restored.paths().contains(&name),
        "`{name}` was removed from the working tree and is still in the corpus"
    );

    println!(
        "the corpus saw an uncommitted file: {} -> {} -> {}",
        before.paths().len(),
        after.paths().len(),
        restored.paths().len()
    );
}

/// The one module in this workspace that may ask Git for a file listing.
const THE_ONE_ASKING_GIT: &str = "xtask/src/repository_files.rs";

/// How a source file asks Git that question: the subcommand's name, opening quote included.
///
/// Assembled from pieces so that *this* file does not contain the sequence it sweeps for. The
/// alternative is an exemption for the scanner itself, and an exemption that exists only to let a
/// scanner past its own definition is the one nobody re-reads when the scanner changes.
fn the_literal() -> String {
    let quote = '"';
    let subcommand = ["ls", "files"].join("-");
    format!("{quote}{subcommand}")
}

/// **The other half.** No gate re-derives the corpus by asking Git what is committed.
///
/// The failure this guards against is not an edit to [`THE_ONE_ASKING_GIT`]; it is a new gate,
/// written by somebody who has never read it, reaching for the four-line `Command` that every one
/// of the four gates used to carry. That gate would get the index back, and it would be right about
/// every file it looked at and blind to every file that mattered.
#[test]
fn no_gate_asks_git_what_is_committed_instead() {
    let root = repository_root();
    let tree = WorkingTree::read(&root);
    println!("{}", tree.census());
    let literal = the_literal();

    let sources: Vec<&String> = tree.with_extension("rs").collect();
    assert!(
        sources.len() >= 200,
        "only {} `.rs` file(s) are in the corpus, so this sweep has stopped matching and would \
         pass over almost nothing",
        sources.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    let mut sanctioned = false;
    for path in sources {
        let Ok(text) = std::fs::read_to_string(root.join(path)) else {
            continue; // a `.rs` file that is not UTF-8 is not source anybody wrote
        };
        if !text.contains(&literal) {
            continue;
        }
        if path == THE_ONE_ASKING_GIT {
            sanctioned = true;
        } else {
            offenders.push(path.clone());
        }
    }

    assert!(
        sanctioned,
        "`{THE_ONE_ASKING_GIT}` does not name the Git subcommand any more, so this sweep is \
         excluding nothing and the exemption below has become a hole. Either that module moved and \
         this constant did not, or the scanner has stopped matching."
    );
    assert!(
        offenders.is_empty(),
        "a source file lists Git's files for itself instead of going through \
         `{THE_ONE_ASKING_GIT}`: {}.\n\
         Asking Git directly returns the *index*, which does not hold a file until it is committed \
         — so a gate built on it cannot judge the file the commit is about to add. That is \
         MJXOFF-290. Use `xtask::repository_files::WorkingTree`.",
        offenders.join(", ")
    );

    println!(
        "{} `.rs` file(s) swept; exactly one — `{THE_ONE_ASKING_GIT}` — asks Git for a listing",
        tree.with_extension("rs").count()
    );
}
