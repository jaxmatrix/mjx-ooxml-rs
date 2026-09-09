//! `cargo run -p xtask -- ledger` — **the parity ledger** (MJXOFF-179).
//!
//! # Why this is generated and not written
//!
//! *"No holes"* is only meaningful if holes are countable, and the only way that claim survives
//! contact with a 3,404-element format and an 11,869-command surface is a matrix produced by
//! evidence rather than by memory. A ledger a person writes is accurate on the day it is written
//! and drifts the moment either the code or the belief changes; the drift is invisible, because a
//! ledger is read as authority and is never itself tested.
//!
//! So the pipeline is split down the middle, and the halves cannot see each other:
//!
//! * [`rows`] declares **what to look at** — a capability, and the suites that are its evidence. It
//!   has no state field, so there is nowhere to write `implemented`.
//! * [`evidence`] reads **what is there** — assertions, declared limitations, declared provenance,
//!   and the three independent facts the report must not restate from memory. It knows nothing
//!   about capabilities.
//! * [`assess`] derives **what follows**, as a pure function of the two. Its rules are unit-tested
//!   against synthetic indices, which is how the ticket's three demonstrations are demonstrations
//!   rather than descriptions.
//! * [`emit`] renders it.
//!
//! # The four ways this generator can fail, and the four gates
//!
//! 1. **A row could name a suite that no longer exists** and be silently emitted from nothing.
//!    `UNCOVERED_SCHEMAS` shipped with exactly that defect and the ticket names it. Here a missing
//!    suite is an `Err` at the lookup, so the build goes red naming the path.
//! 2. **A row could default to `implemented`.** The default is `not-started`, it is the *first*
//!    arm that can be reached, and `assess`'s own tests prove an uncovered row lands there.
//! 3. **A suite could exist and assert nothing** — green precisely because the work is skipped.
//!    The assertion count is derived from the source, and a suite with none promotes nothing.
//! 4. **The committed file could stop being regenerated.** `--check` regenerates in memory and
//!    refuses if the file on disk is not what the tree produces, exactly as the token pipeline
//!    does, and `xtask/tests/ledger.rs` runs it.
//!
//! There is a fifth, and it runs the other way: a suite could declare a limitation that **no row
//! cites**, so a known defect would be asserted in the tree and absent from the ledger.
//! [`check_every_declared_limitation_is_cited`] refuses that too.
//!
//! # What this command does not do
//!
//! It does not run the renderer, does not open a document, and judges nothing itself. It reads
//! source files and committed data. That is deliberate: a ledger downstream of a `cargo test` run
//! would be a ledger that cannot be produced on a machine without a GPU, and the one thing this
//! document must always be is available.

pub(crate) mod assess;
pub(crate) mod emit;
pub(crate) mod evidence;
pub(crate) mod rows;

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use assess::Assessed;
use rows::{Kind, CAPABILITIES};

/// The committed artefact, relative to the workspace root.
const LEDGER: &str = "docs/client-platform/PARITY_LEDGER.md";

/// Assesses every row against an evidence index, in declaration order.
pub(crate) fn assess_all(evidence: &evidence::Evidence) -> Result<Vec<Assessed>> {
    check_the_rows_are_well_formed()?;
    let mut assessed = Vec::with_capacity(CAPABILITIES.len());
    for capability in CAPABILITIES {
        assessed.push(assess::assess(capability, evidence)?);
    }
    check_every_declared_limitation_is_cited(&assessed, evidence)?;
    Ok(assessed)
}

/// Renders the ledger for a tree, without touching the disk.
///
/// The tree is read exactly once: the scan is the expensive half, and running it twice would let a
/// concurrent edit produce a document whose two halves disagree.
pub(crate) fn generate(root: &std::path::Path) -> Result<String> {
    let evidence = evidence::scan(root)?;
    let assessed = assess_all(&evidence)?;
    Ok(emit::render(&assessed, &evidence))
}

/// Regenerates the ledger, or — with `--check` — refuses if the committed one is not what the tree
/// produces.
///
/// `--out-dir` writes somewhere other than the workspace. It exists for the same reason the token
/// pipeline's does, and the reasoning is worth repeating rather than looking up: a test that
/// regenerates *in place* truncates a file another test binary is concurrently reading, and no
/// in-process lock can order two processes. A writer that writes somewhere disposable removes the
/// shared mutable state instead of scheduling around it.
pub(crate) fn run(arguments: &[String]) -> Result<()> {
    let mut check = false;
    let mut output_root: Option<PathBuf> = None;
    let mut remaining = arguments.iter();
    while let Some(argument) = remaining.next() {
        match argument.as_str() {
            "--check" => check = true,
            "--out-dir" => {
                let path = remaining.next().context(
                    "`--out-dir` needs a directory; usage: `cargo run -p xtask -- ledger [--check] \
                     [--out-dir <directory>]`",
                )?;
                output_root = Some(PathBuf::from(path));
            }
            _ => bail!(
                "unknown arguments {arguments:?}; usage: `cargo run -p xtask -- ledger [--check] \
                 [--out-dir <directory>]`"
            ),
        }
    }

    let root = crate::codegen::workspace_root();
    let rendered = generate(&root)?;
    let output_root = output_root.unwrap_or_else(|| root.clone());
    let path = output_root.join(LEDGER);

    if check {
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        if let Some(report) = crate::codegen::tokens::first_difference(&committed, &rendered) {
            bail!(
                "the committed parity ledger is not what this workspace's suites produce:\n  \
                 {LEDGER}: {report}\n\
                 Run `cargo run -p xtask -- ledger` and commit the result. This file is generated; \
                 the suites are the source, and a row's state is not editable here."
            );
        }
        println!(
            "ledger: {LEDGER} matches the suites ({} rows)",
            CAPABILITIES.len()
        );
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    crate::codegen::write_plain(&path, &rendered)?;
    println!("ledger: wrote {LEDGER} ({} rows)", CAPABILITIES.len());
    Ok(())
}

/// The table's own invariants: unique identifiers, and a reason on exactly the excluded rows.
///
/// A duplicate identifier is worth failing on rather than tolerating, because the identifier is how
/// a ticket refers to a row — two rows sharing one makes every reference ambiguous forever.
fn check_the_rows_are_well_formed() -> Result<()> {
    let mut seen = BTreeSet::new();
    for capability in CAPABILITIES {
        if !seen.insert(capability.id) {
            bail!(
                "the parity ledger declares `{}` twice; a row's identifier is what a ticket cites, \
                 so it has to be unique",
                capability.id
            );
        }
        match (capability.kind, capability.excluded_because) {
            (Kind::Excluded, None) => bail!(
                "`{}` is excluded and states no reason. An exclusion without a reason is a silent \
                 omission wearing a state, which is the thing the fifth state exists to prevent",
                capability.id
            ),
            (Kind::Excluded, Some(reason)) if reason.len() < 40 => bail!(
                "`{}`'s exclusion reason is too short to reopen the decision from: {reason:?}",
                capability.id
            ),
            (kind, Some(_)) if kind != Kind::Excluded => bail!(
                "`{}` states an exclusion reason and is not excluded",
                capability.id
            ),
            _ => {}
        }
    }
    Ok(())
}

/// Every `MJX-LEDGER-LIMITATION:` in the workspace is cited by at least one row.
///
/// **This is the check that runs the other way.** Everything else here asks whether the ledger's
/// claims are supported by the tree; this asks whether the tree's declared defects reached the
/// ledger. A suite that asserts a known-wrong behaviour and is named by no row would leave the
/// defect proved in the code and invisible in the document a reader treats as authority — which is
/// the same failure as a hand-written table, arrived at from the other side.
fn check_every_declared_limitation_is_cited(
    rows: &[Assessed],
    evidence: &evidence::Evidence,
) -> Result<()> {
    let cited: BTreeSet<&str> = rows
        .iter()
        .flat_map(|row| row.limitations.iter().map(|(path, _)| path.as_str()))
        .collect();
    for (path, text) in evidence.declared_limitations() {
        if !cited.contains(path) {
            bail!(
                "`{path}` declares a limitation that no ledger row cites:\n  {text}\n\
                 A defect asserted in the tree and absent from the ledger is worse than one that is \
                 merely unfixed. Add the suite to the evidence of the row it is about in \
                 `xtask/src/ledger/rows.rs`."
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed table survives its own rules, in a test rather than in a terminal.
    #[test]
    fn the_committed_rows_are_well_formed() {
        check_the_rows_are_well_formed().expect("the committed rows are well formed");
    }

    /// **There is no state in the row table.** The evidence is that the only place any of the five
    /// names appears in `rows.rs` is inside prose, and never as a value a row could carry.
    #[test]
    fn no_row_can_declare_its_own_state() {
        let source = include_str!("rows.rs");
        for spelling in [
            "state: State::",
            "State::Implemented",
            "State::Partial",
            "State::NotStarted",
        ] {
            assert!(
                !source.contains(spelling),
                "`rows.rs` contains `{spelling}`, so a row can now assert its own state and the \
                 ledger's first rule is broken"
            );
        }
    }

    /// Every row's evidence path is a plausible suite path, which catches a typo one round trip
    /// before the filesystem does.
    #[test]
    fn every_evidence_path_is_shaped_like_a_suite() {
        for capability in CAPABILITIES {
            for path in capability.evidence {
                assert!(
                    path.starts_with("crates/")
                        && path.contains("/tests/")
                        && path.ends_with(".rs"),
                    "`{}` names `{path}`, which is not a suite path",
                    capability.id
                );
            }
        }
    }

    /// An excluded row consults no suite, so pointing one at evidence would be misleading.
    #[test]
    fn an_excluded_row_names_no_evidence() {
        for capability in CAPABILITIES {
            if capability.kind == Kind::Excluded {
                assert!(
                    capability.evidence.is_empty(),
                    "`{}` is excluded and names evidence, which no reader would expect to be \
                     consulted — because it is not",
                    capability.id
                );
            }
        }
    }
}
