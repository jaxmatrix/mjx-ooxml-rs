//! Every `MJX_REQUIRE_…` escape is either bound by a workflow or justified where it is defined
//! (MJXOFF-197).
//!
//! ## Why this is a test and not a sweep somebody did once
//!
//! This workspace's suites skip when the thing they need is absent — a licensed schema tree, a
//! LibreOffice install, a graphics device, poppler — and print a named notice when they do. Each
//! such skip has an escape hatch, an environment variable whose presence turns the absence into a
//! hard failure, and continuous integration sets it so that "green" can never quietly mean "nothing
//! ran".
//!
//! That arrangement has one failure mode, and it is the one MJXOFF-197 was filed about:
//! **an escape nobody sets is invisible.** The suite passes, the log says `ok`, and the coverage it
//! reports does not exist. `MJX_REQUIRE_PRESET_GEOMETRY` sat in exactly that state through the
//! entire history of the repository — a sweep over the whole normative preset-shape corpus that had
//! never once executed on CI, green every time.
//!
//! A one-time census closes that instance and nothing else. While MJXOFF-197 was being written, a
//! child added `MJX_REQUIRE_OFFICE_EXPORTS`; between the brief for this work and its first hour, a
//! peer branch added another. **The roster expires faster than anyone reads it**, so the deliverable
//! is this gate rather than a list, and nothing here hard-codes what the roster is expected to
//! contain beyond the handful of memberships that are themselves regression guards.
//!
//! ## The three states
//!
//! 1. **Set** — genuinely bound in a workflow's `env:` mapping, at job or at step level.
//! 2. **Unset, justified** — not bound anywhere, and the comment block above one of its definition
//!    sites carries the [`MARKER`] and says why. `MJX_REQUIRE_OFFICE_CORPUS` and
//!    `MJX_REQUIRE_OFFICE_EXPORTS` are both in this state: their corpora are empty by design and no
//!    agent may fill them, so setting either would make a build red about something no build can
//!    fix.
//! 3. **Unset, unjustified** — the failure. Also a failure: an escape that is *both* bound and
//!    marked deliberately-unset, because then one of the two is lying.
//!
//! ## Why the workflow is parsed rather than grepped
//!
//! **A string census cannot tell a binding from a comment explaining why there is no binding** — and
//! that comment is exactly what a careful author writes when leaving one unset deliberately. Our own
//! tree has the case: `MJX_REQUIRE_OFFICE_CORPUS`'s only appearance in `.github/workflows/ci.yml` is
//! a comment saying it is deliberately not set. A grep reports it as set. It is not set, and a peer
//! session's instrument made precisely this mistake.
//!
//! So [`env_bindings`] reads the file's indentation structure — comments removed quote-aware, a
//! stack of mapping keys, a binding recognised only under an `env` ancestor — and
//! [`a_workflow_comment_is_not_a_binding`] holds it to that. A scanner that silently stops
//! discriminating passes forever, which is this ticket's own defect one level up; every test below
//! that runs the scanner over a synthetic tree exists for that reason.
//!
//! An inline `VAR=1 cargo test …` inside a `run:` script is deliberately **not** counted as a
//! binding. It really would set the variable, so this is a conservative reading — but it is
//! conservative in the safe direction (a spurious *failure*, whose message names the fix: move it
//! into an `env:` map) rather than in the direction that lets an escape be reported as covered when
//! it is not.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The marker a deliberately-unset escape carries in the comment block above a definition site.
///
/// The same shape `MJX-PAINT-SURFACE-UNSAFE` uses for the one hand-written `unsafe` block in
/// `mjx-paint`: a machine cannot read prose, so what it checks is that a human wrote some, under a
/// token that cannot be typed by accident.
const MARKER: &str = "MJX-ESCAPE-UNSET";

/// How much prose the marker's comment block must carry, in characters.
///
/// A smoke alarm, not a judge. It cannot tell a reason from a sentence, and it is not trying to —
/// it only makes a bare rubber stamp fail, so that the marker costs as much to add as to mean.
const MINIMUM_PROSE: usize = 80;

/// File kinds whose text is read for escape names.
///
/// Prose is deliberately absent: `CHANGELOG.md` records escapes that no longer exist, and a history
/// file is not a definition site. Code and configuration name what is live.
const SCANNED: [&str; 9] = ["rs", "sh", "py", "mjs", "js", "ts", "toml", "yml", "yaml"];

/// Directories never descended into: build output, vendored packages, and the licensed schema tree.
const SKIPPED: [&str; 6] = [
    "target",
    "References",
    "node_modules",
    "dist",
    "pkg",
    "tests/fixtures",
];

// -------------------------------------------------------------------------------------------
// The scanner
// -------------------------------------------------------------------------------------------

/// `MJX_REQUIRE_`, assembled at run time.
///
/// Written in halves so that this file is not its own finding: the scanner reads raw bytes, and a
/// literal prefix here would make every name mentioned in these doc comments look like a definition
/// site living in a test. The synthetic names below are built the same way and for the same reason.
fn escape_prefix() -> String {
    format!("MJX_{}", "REQUIRE_")
}

/// One escape and everywhere the tree has something to say about it.
#[derive(Debug, Default)]
struct Escape {
    /// Every scanned file that names it, in walk order.
    named_in: Vec<PathBuf>,
    /// Workflow files whose `env:` mapping genuinely binds it.
    bound_in: Vec<PathBuf>,
    /// Definition sites whose preceding comment block carries the marker and prose.
    justified_at: Vec<PathBuf>,
}

/// What one walk of a tree found.
#[derive(Debug, Default)]
struct Census {
    escapes: BTreeMap<String, Escape>,
    files_read: usize,
}

impl Census {
    /// The escapes in each of the three states, plus the contradiction.
    fn verdicts(&self) -> BTreeMap<&str, Verdict> {
        self.escapes
            .iter()
            .map(|(name, escape)| {
                let verdict = match (escape.bound_in.is_empty(), escape.justified_at.is_empty()) {
                    (false, true) => Verdict::Set,
                    (true, false) => Verdict::JustifiedUnset,
                    (true, true) => Verdict::Unjustified,
                    (false, false) => Verdict::Contradiction,
                };
                (name.as_str(), verdict)
            })
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Verdict {
    /// Bound in a workflow's `env:` mapping.
    Set,
    /// Bound nowhere, and a definition site says why.
    JustifiedUnset,
    /// Bound nowhere and explained nowhere — the failure MJXOFF-197 closes.
    Unjustified,
    /// Bound *and* marked deliberately-unset. One of the two is out of date.
    Contradiction,
}

/// Walks `root` and classifies every escape it names.
fn census(root: &Path) -> Census {
    let prefix = escape_prefix();
    let mut found = Census::default();
    let mut stack = vec![root.to_path_buf()];

    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        children.sort();

        for path in children {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.is_dir() {
                // Everything hidden except `.github`, which is where the workflows live.
                let hidden = name.starts_with('.') && name != ".github";
                let relative = path.strip_prefix(root).unwrap_or(&path);
                let skipped = SKIPPED
                    .iter()
                    .any(|entry| name == *entry || relative.ends_with(entry));
                if !hidden && !skipped {
                    stack.push(path);
                }
                continue;
            }

            let extension = path.extension().and_then(|extension| extension.to_str());
            if !extension.is_some_and(|extension| SCANNED.contains(&extension)) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let text = String::from_utf8_lossy(&bytes);
            found.files_read += 1;

            let names = escapes_in(&text, &prefix);
            if names.is_empty() {
                continue;
            }
            let workflow = is_workflow(root, &path);
            let bound: BTreeSet<String> = if workflow {
                env_bindings(&text).into_iter().collect()
            } else {
                BTreeSet::new()
            };

            for name in names {
                let escape = found.escapes.entry(name.clone()).or_default();
                escape.named_in.push(path.clone());
                if bound.contains(&name) {
                    escape.bound_in.push(path.clone());
                }
                if extension == Some("rs") && justified_in(&text, &name) {
                    escape.justified_at.push(path.clone());
                }
            }
        }
    }

    found
}

/// Whether this file is one of the repository's workflow definitions.
fn is_workflow(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|relative| relative.starts_with(Path::new(".github").join("workflows")))
}

/// Every distinct escape name in `text`.
fn escapes_in(text: &str, prefix: &str) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let mut names = BTreeSet::new();
    let mut from = 0usize;

    while let Some(offset) = text[from..].find(prefix) {
        let start = from + offset;
        from = start + prefix.len();

        // Not the tail of a longer identifier.
        let preceded = start
            .checked_sub(1)
            .is_some_and(|before| bytes[before].is_ascii_alphanumeric() || bytes[before] == b'_');
        if preceded {
            continue;
        }

        let mut end = start + prefix.len();
        while end < bytes.len() && (bytes[end].is_ascii_uppercase() || bytes[end] == b'_') {
            end += 1;
        }
        let name = text[start..end].trim_end_matches('_');
        if name.len() > prefix.len() {
            names.insert(name.to_owned());
        }
    }

    names
}

/// Every key a workflow binds inside an `env:` mapping, at any nesting depth.
///
/// An indentation-structured read of the subset GitHub workflows are written in, not a grep: a key
/// counts only when an `env` mapping is one of its open ancestors, so a comment, a `run:` script
/// line and a job name cannot be mistaken for a binding.
fn env_bindings(text: &str) -> Vec<String> {
    let mut open: Vec<(usize, String)> = Vec::new();
    let mut bound = Vec::new();

    for raw in text.lines() {
        let line = strip_comment(raw).trim_end();
        if line.trim().is_empty() {
            continue;
        }

        // Where the key starts: past the indentation, and past any number of `- ` sequence markers,
        // each of which shifts a key rightwards exactly as further indentation would.
        let mut offset = line.len() - line.trim_start().len();
        while let Some(after) = line[offset..].strip_prefix('-') {
            if !after.starts_with(' ') {
                break;
            }
            offset += 1 + (after.len() - after.trim_start().len());
        }

        let entry = &line[offset..];
        let Some(colon) = entry.find(':') else {
            continue;
        };
        let key = entry[..colon].trim();
        let value = entry[colon + 1..].trim();
        let plausible = !key.is_empty()
            && key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-.".contains(character));
        if !plausible {
            continue;
        }

        while open.last().is_some_and(|(indent, _)| *indent >= offset) {
            open.pop();
        }
        if value.is_empty() {
            open.push((offset, key.to_owned()));
        } else if open.iter().any(|(_, ancestor)| ancestor == "env") {
            bound.push(key.to_owned());
        }
    }

    bound
}

/// `line` with any YAML comment removed — `#` outside a quoted scalar, at the start or after
/// whitespace.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;

    for (index, &byte) in bytes.iter().enumerate() {
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'#' && (index == 0 || bytes[index - 1].is_ascii_whitespace()) => {
                return &line[..index];
            }
            None => {}
        }
    }

    line
}

/// Whether some **definition site** of `escape` in this Rust source carries the marker, with prose,
/// in the contiguous comment block directly above it.
///
/// A definition site is a line naming the escape as a *string literal* — `"…"` — which is what
/// reading an environment variable looks like and what a `const` holding its name looks like. A
/// mention in prose is not one, and the distinction is load-bearing rather than pedantic: this
/// file's own module documentation names three real escapes inside one contiguous comment block
/// that also carries the marker, and a laxer rule would have this test justify them for itself.
fn justified_in(text: &str, escape: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let quoted = format!("\"{escape}\"");

    for (index, line) in lines.iter().enumerate() {
        if !line.contains(&quoted) {
            continue;
        }
        let mut block = vec![*line];
        let mut above = index;
        while above > 0 && lines[above - 1].trim_start().starts_with("//") {
            above -= 1;
            block.push(lines[above]);
        }
        if !block.iter().any(|line| line.contains(MARKER)) {
            continue;
        }
        let prose: usize = block
            .iter()
            .filter(|line| line.trim_start().starts_with("//"))
            .map(|line| line.trim_start().trim_start_matches('/').trim().len())
            .sum();
        if prose >= MINIMUM_PROSE {
            return true;
        }
    }

    false
}

/// This repository's root — `xtask`'s parent.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

#[test]
fn every_escape_is_set_in_a_workflow_or_justified_at_its_definition_site() {
    let found = census(&repository());
    let verdicts = found.verdicts();
    let mut complaint = String::new();

    for (name, verdict) in &verdicts {
        let escape = &found.escapes[*name];
        let sites = |paths: &[PathBuf]| {
            paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        match verdict {
            Verdict::Set | Verdict::JustifiedUnset => {}
            Verdict::Unjustified => {
                let _ = write!(
                    complaint,
                    "\n  {name}: named in {} but bound by no workflow `env:` mapping and explained \
                     nowhere.\n    Either set it in `.github/workflows/`, or write why it is not \
                     set in the comment block above one of its definition sites and mark that \
                     block `{MARKER}`.\n    An escape nobody sets is a suite that reports coverage \
                     it does not have — which is exactly how the preset-geometry sweep stayed green \
                     without ever running (MJXOFF-197).",
                    sites(&escape.named_in),
                );
            }
            Verdict::Contradiction => {
                let _ = write!(
                    complaint,
                    "\n  {name}: bound by {} *and* marked `{MARKER}` at {}. One of the two is out \
                     of date; the marker means \"deliberately not set anywhere\".",
                    sites(&escape.bound_in),
                    sites(&escape.justified_at),
                );
            }
        }
    }

    assert!(
        complaint.is_empty(),
        "{} escape(s) in this workspace are neither set nor justified:{complaint}",
        verdicts
            .values()
            .filter(|verdict| !matches!(verdict, Verdict::Set | Verdict::JustifiedUnset))
            .count(),
    );

    eprintln!(
        "escape census: {} variables over {} files — {} set in a workflow, {} justified-unset",
        verdicts.len(),
        found.files_read,
        verdicts
            .values()
            .filter(|verdict| **verdict == Verdict::Set)
            .count(),
        verdicts
            .values()
            .filter(|verdict| **verdict == Verdict::JustifiedUnset)
            .count(),
    );
}

// -------------------------------------------------------------------------------------------
// The instrument: what the scanner sees, proved rather than assumed
// -------------------------------------------------------------------------------------------

/// A synthetic escape name, never written literally anywhere, so the tree's own census cannot see
/// the fixtures these tests build.
fn synthetic(suffix: &str) -> String {
    format!("{}{suffix}", escape_prefix())
}

/// Writes `files` under a fresh temporary root and returns it.
fn tree(label: &str, files: &[(&str, String)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mjx-escape-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (relative, contents) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("creating the fixture");
        std::fs::write(&path, contents).expect("writing the fixture");
    }
    root
}

#[test]
fn a_workflow_comment_is_not_a_binding() {
    // The false positive this whole parser exists for, and the one a peer instrument shipped: a
    // careful author who leaves an escape unset writes a comment in the workflow saying so, and a
    // string census then reports the variable as set by that very comment.
    let name = synthetic("COMMENTED");
    let workflow = format!(
        "name: CI\njobs:\n  a-job:\n    env:\n      # {name} is deliberately NOT set here.\n      \
         OTHER: \"1\"\n    steps:\n      - run: cargo test\n"
    );
    let source = format!("fn read() {{ std::env::var_os(\"{name}\"); }}\n");
    let root = tree(
        "comment",
        &[
            (".github/workflows/ci.yml", workflow),
            ("crates/thing/src/lib.rs", source),
        ],
    );

    let found = census(&root);
    assert!(
        found.escapes[&name].bound_in.is_empty(),
        "a comment bound it"
    );
    assert_eq!(found.verdicts()[name.as_str()], Verdict::Unjustified);
}

#[test]
fn an_env_mapping_binds_at_job_level_and_at_step_level() {
    // Both shapes this repository actually uses, including the sequence-item form where the key sits
    // after a `- ` and the indentation stack has to follow it.
    let job = synthetic("ATJOB");
    let step = synthetic("ATSTEP");
    let workflow = format!(
        "name: CI\njobs:\n  a-job:\n    env:\n      {job}: \"1\"\n    steps:\n      - uses: \
         actions/checkout@v4\n      - name: the step\n        env:\n          {step}: \"1\"\n        \
         run: cargo test\n"
    );
    let root = tree("bound", &[(".github/workflows/ci.yml", workflow)]);

    let found = census(&root);
    assert_eq!(found.verdicts()[job.as_str()], Verdict::Set);
    assert_eq!(found.verdicts()[step.as_str()], Verdict::Set);
}

#[test]
fn an_assignment_inside_a_run_script_is_not_counted_as_a_binding() {
    // Deliberate and documented: it really would set the variable, but recognising it means reading
    // shell out of a YAML block scalar, and the failure it would risk is the dangerous direction —
    // reporting an escape as covered when it is not. Failing here instead says "move it into an
    // `env:` map", which is both the fix and the better style.
    let name = synthetic("INLINE");
    let workflow =
        format!("name: CI\njobs:\n  a-job:\n    steps:\n      - run: {name}=1 cargo test\n");
    let root = tree("inline", &[(".github/workflows/ci.yml", workflow)]);

    let found = census(&root);
    assert_eq!(found.verdicts()[name.as_str()], Verdict::Unjustified);
}

#[test]
fn the_gate_refuses_an_escape_that_is_neither_set_nor_explained() {
    // The proof that this gate can fail at all. Adding a variable and nothing else must be red;
    // a check that cannot go red is the defect it was written to catch.
    let name = synthetic("NAKED");
    let source = format!("fn read() {{ std::env::var_os(\"{name}\"); }}\n");
    let root = tree(
        "naked",
        &[
            (
                ".github/workflows/ci.yml",
                "name: CI\njobs: {}\n".to_owned(),
            ),
            ("crates/thing/src/lib.rs", source),
        ],
    );

    let found = census(&root);
    assert_eq!(found.verdicts()[name.as_str()], Verdict::Unjustified);
}

#[test]
fn a_marked_comment_block_with_prose_justifies_an_unset_escape() {
    let name = synthetic("EXPLAINED");
    let source = format!(
        "/// The escape for a corpus that ships empty on purpose.\n\
         ///\n\
         /// {MARKER}: deliberately not set in any workflow, because the directory it guards is \
         empty by design and no agent may fill it — setting it would make the build red about \
         something no build can fix.\n\
         pub const ESCAPE: &str = \"{name}\";\n"
    );
    let root = tree("explained", &[("crates/thing/src/lib.rs", source)]);

    let found = census(&root);
    assert_eq!(found.verdicts()[name.as_str()], Verdict::JustifiedUnset);
}

#[test]
fn a_bare_marker_with_no_prose_does_not_justify_anything() {
    // The marker is meant to cost as much to add as to mean. A rubber stamp is still unjustified.
    let name = synthetic("STAMPED");
    let source = format!("// {MARKER}\npub const ESCAPE: &str = \"{name}\";\n");
    let root = tree("stamped", &[("crates/thing/src/lib.rs", source)]);

    let found = census(&root);
    assert_eq!(found.verdicts()[name.as_str()], Verdict::Unjustified);
}

#[test]
fn an_escape_that_is_both_bound_and_marked_unset_is_a_contradiction() {
    let name = synthetic("BOTH");
    let workflow = format!("name: CI\njobs:\n  a-job:\n    env:\n      {name}: \"1\"\n");
    let source = format!(
        "/// {MARKER}: deliberately not set anywhere, because the corpus it guards is empty and \
         nothing a build can do would fill it.\n\
         pub const ESCAPE: &str = \"{name}\";\n"
    );
    let root = tree(
        "both",
        &[
            (".github/workflows/ci.yml", workflow),
            ("crates/thing/src/lib.rs", source),
        ],
    );

    let found = census(&root);
    assert_eq!(found.verdicts()[name.as_str()], Verdict::Contradiction);
}

#[test]
fn the_scanner_reaches_this_repository_and_classifies_what_is_in_it() {
    // Not a count of the roster — the roster expires: two escapes were added to this workspace
    // while MJXOFF-197 was open, one of them on a branch this tree has never seen. What is asserted
    // is that the walk reached the tree at all (a scanner that silently stopped walking would pass
    // every other test in this file), and the memberships that are regression guards in their own
    // right.
    let found = census(&repository());
    let verdicts = found.verdicts();

    assert!(
        found.files_read > 300,
        "the walk read only {} files — it is not reaching this repository",
        found.files_read,
    );
    assert!(
        verdicts.len() >= 5,
        "only {} escapes found; this workspace has more than that",
        verdicts.len(),
    );

    // The one this gate was written for. If the `schema-validity` step that sets it is ever
    // removed, the preset-shape sweep goes back to skipping silently — and this line goes red.
    let prefix = escape_prefix();
    assert_eq!(
        verdicts.get(format!("{prefix}PRESET_GEOMETRY").as_str()),
        Some(&Verdict::Set),
        "the preset-geometry sweep is not bound by any workflow",
    );
    assert_eq!(
        verdicts.get(format!("{prefix}SCHEMA").as_str()),
        Some(&Verdict::Set),
    );
    assert_eq!(
        verdicts.get(format!("{prefix}OFFICE_CORPUS").as_str()),
        Some(&Verdict::JustifiedUnset),
    );

    // And the discrimination that matters, measured on the real file rather than a fixture: the
    // repository's own workflow mentions `MJX_REQUIRE_OFFICE_CORPUS` in a comment saying it is
    // deliberately not set. A census that reported it as bound would have that verdict wrong.
    let workflow = std::fs::read_to_string(repository().join(".github/workflows/ci.yml"))
        .expect("the workflow");
    assert!(
        workflow.contains(&format!("{prefix}OFFICE_CORPUS is deliberately NOT set")),
        "the comment this discrimination is measured against has been reworded",
    );
    assert!(
        !env_bindings(&workflow).contains(&format!("{prefix}OFFICE_CORPUS")),
        "a comment was read as a binding",
    );
}
