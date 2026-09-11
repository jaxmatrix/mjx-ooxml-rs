//! The corpus a derived gate sweeps: **every file this working tree holds** (MJXOFF-290).
//!
//! # The hole this closes
//!
//! Four integration tests in `xtask/tests/` derive their corpus from Git rather than from a
//! hand-maintained list — `xtask/tests/doc_gate.rs`, `xtask/tests/entry_points.rs`,
//! `xtask/tests/derived_rosters.rs` and `xtask/tests/release_versions.rs` — and deriving it is the
//! decision `CLAUDE.md` insists on: a `const FIXTURES` list is a corpus a new file can sit outside
//! of forever. The consequence nobody had written down is that `git ls-files` answers *what is in
//! the index*, and **a file a unit of work has just written is in no index yet.**
//!
//! So a new document, test or example is invisible to the gate that judges it for exactly as long
//! as it matters. The unit that found this ran the whole local gate set green across three feature
//! modes, committed, and the *next* run of the same command failed — `doc_gate` reporting that the
//! module docs of the file that unit had just added, `xtask/tests/release_versions.rs`, named a
//! git-ignored build directory that is not on disk. A genuine finding, and the gate was right; it
//! arrived one commit late, which is the only thing wrong with it.
//!
//! That makes this repository's standing rule — *commit only when `cargo build` and
//! `cargo test --workspace` are green* — **unsatisfiable for the commit that introduces a file**,
//! because the gate that judges the file only begins running once the commit exists. It is not a
//! hypothetical: `xtask/src/validation/model.rs` reached `main` naming a page that has never
//! existed, and `doc_gate` was red on `main` from 0.0.168 until MJXOFF-287 noticed.
//!
//! # The answer, and why it is one answer for every gate
//!
//! The question these gates are really asking is *"what does this repository consist of?"*, and the
//! honest answer is the working tree rather than the index. Git already knows which files those are
//! and which are none of our business: `--others --exclude-standard` is *untracked and not ignored*,
//! so `target/`, the git-ignored `References/` tree, the Python virtualenv, `node_modules` and every
//! tool cache stay out by being ignored rather than by being listed here.
//!
//! The alternative — leave the corpus alone and tell the next agent to run `git add -N` first — is a
//! note, not a fix. A ritual nobody is reminded of at the moment it matters is exactly the shape of
//! gate this repository keeps writing tests against.
//!
//! **Every one of the four wants the working tree**, and the reasons are worth stating separately
//! because "they all agreed" is the kind of claim that stops being true when a fifth gate arrives:
//!
//! * `doc_gate.rs` — a claim in a document's prose is wrong the moment it is written, not the
//!   moment it is committed, and its index check reads *"a page that exists is indexed"*, which a
//!   page in the working tree already is.
//! * `entry_points.rs` — a count on the front page has to be updated in the same commit that
//!   changes what it counts, so the derivation has to see the change before that commit exists.
//! * `derived_rosters.rs` — a roster written into a brand-new file is a roster, and the population
//!   it claims to be the whole of is one the sweep must be able to check.
//! * `release_versions.rs` — a fifth file that states the version is a release hazard from the
//!   moment it is written; the whole point of that sweep is that nobody has to notice it.
//!
//! # Why one module rather than four copies
//!
//! Four derivations of one fact would disagree with no way to say which was wrong — the reason
//! `xtask/src/binding_surface.rs` and `xtask/src/fixture_corpus.rs` already exist. It also gives
//! the property a single place to be tested: `xtask/tests/working_tree_corpus.rs` writes a file it
//! does **not** commit and requires this module to see it, and sweeps every `.rs` file in the
//! workspace so that no gate can quietly go back to asking Git what is committed.
//!
//! # What is deliberately *not* in the corpus, and why
//!
//! * **Anything Git ignores.** The exclusion list is `.gitignore`, and it is the only one — there is
//!   no skip list in this module, on purpose. A directory that should not be swept belongs in
//!   `.gitignore`, where every other tool in this repository already reads it.
//! * **A path that is not a file on disk.** `--cached` still lists a tracked file that has been
//!   deleted in the working tree, and `--others` reports a nested repository (an agent worktree
//!   under `.claude/`) as a single directory entry. Neither is a file this repository holds, and a
//!   gate that tried to read one would panic rather than report.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// The files a derived gate sweeps, and the split that proves the derivation is still reaching
/// both halves of the working tree.
///
/// Built by [`WorkingTree::read`]. A gate reads [`WorkingTree::paths`] for the corpus and prints
/// [`WorkingTree::census`] beside its own counts, because a corpus that has quietly stopped
/// including one half looks exactly like a clean checkout.
#[derive(Debug, Clone)]
pub struct WorkingTree {
    /// Every path, repository-relative in `/` form, sorted and without duplicates.
    paths: Vec<String>,
    /// How many of them Git has in its index.
    tracked: usize,
    /// How many of them are untracked and not ignored — the half this module exists for.
    untracked: usize,
}

impl WorkingTree {
    /// Reads the working tree rooted at `root`.
    ///
    /// # Panics
    ///
    /// If `git` cannot be run, reports a failure, emits a path that is not UTF-8, or reports a tree
    /// too small to be this repository. Every one of those is a hard failure and never a skip: an
    /// empty corpus makes every sweep written against it pass while checking nothing.
    pub fn read(root: &Path) -> Self {
        let tracked = listing(root, "--cached");
        let untracked = listing(root, "--others");
        assert!(
            tracked.len() > 500,
            "git reported {} tracked file(s), which cannot be this repository — every sweep \
             written against this corpus would pass vacuously",
            tracked.len()
        );
        let paths: Vec<String> = tracked.union(&untracked).cloned().collect();
        Self {
            paths,
            tracked: tracked.len(),
            untracked: untracked.len(),
        }
    }

    /// Every file the working tree holds that Git does not ignore, sorted.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// The paths whose extension is `extension`, written without its dot — `"rs"`, `"md"`.
    pub fn with_extension<'a>(&'a self, extension: &'a str) -> impl Iterator<Item = &'a String> {
        let suffix = format!(".{extension}");
        self.paths
            .iter()
            .filter(move |path| path.ends_with(&suffix))
    }

    /// How many corpus files Git has in its index.
    pub fn tracked(&self) -> usize {
        self.tracked
    }

    /// How many corpus files are untracked and not ignored.
    ///
    /// Zero in a clean checkout, which is what CI has, and non-zero in exactly the situation this
    /// module was written for: a unit of work that has written a file and not yet committed it.
    pub fn untracked(&self) -> usize {
        self.untracked
    }

    /// The one-line count a gate prints beside its own, so the corpus's shape is visible on a green
    /// run rather than only on a red one.
    pub fn census(&self) -> String {
        format!(
            "corpus: {} file(s) in the working tree — {} tracked, {} untracked and not ignored",
            self.paths.len(),
            self.tracked,
            self.untracked
        )
    }
}

/// One `git ls-files` listing, as a set of repository-relative paths that are files on disk.
///
/// `which` is `--cached` or `--others`; `-z` because a path may hold anything but a NUL, and
/// `--exclude-standard` because `.gitignore` is the only skip list this module has.
fn listing(root: &Path, which: &str) -> BTreeSet<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg("-z")
        .arg("--exclude-standard")
        .arg(which)
        .output()
        .unwrap_or_else(|error| panic!("running git ls-files {which}: {error}"));
    assert!(
        output.status.success(),
        "git ls-files {which} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listing = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("git ls-files {which} emitted a non-UTF-8 path: {error}"));
    listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter(|path| root.join(path).is_file())
        .map(str::to_owned)
        .collect()
}
