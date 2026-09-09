//! **Rendering the ledger** — the Markdown a person reads, and the header they meet before the
//! table.
//!
//! The header is not decoration. A ledger is read as authority, so the three rules of
//! `OFFICE_FEATURE_INVENTORY.md` §7 and the sentence that says what a green row is *not* have to
//! arrive before the first number, not after the last one.
//!
//! Everything printed here is computed. There is no sentence in this file that states a count, a
//! ratio or a fact about the tree: the counts come from the assessment, the censuses from the two
//! committed derivations, and the approval record from the oracle's own `APPROVAL` files. A number
//! typed into prose is a number that will be wrong by the next commit.

use std::fmt::Write as _;

use super::assess::{Assessed, State};
use super::evidence::{Evidence, Provenance, Split};
use super::rows::{Kind, Section};

/// Renders the whole document.
pub(crate) fn render(rows: &[Assessed], evidence: &Evidence) -> String {
    let mut out = String::with_capacity(64 * 1024);
    header(&mut out);
    standing_facts(&mut out, rows, evidence);
    counts(&mut out, rows);
    denominators(&mut out, rows, evidence);
    provenance(&mut out, rows, evidence);
    limitations(&mut out, rows);
    table(&mut out, rows);
    out
}

/// The part a reader meets before any number.
fn header(out: &mut String) {
    out.push_str(
        "# The parity ledger\n\
         \n\
         > **Generated. Do not edit.** Produced by `cargo run -p xtask -- ledger` from the suites \
         in this workspace; `cargo run -p xtask -- ledger --check` refuses if this file is not what \
         they produce, and that check runs in `xtask/tests/ledger.rs`. The rows come from\n\
         > [`OFFICE_FEATURE_INVENTORY.md`](OFFICE_FEATURE_INVENTORY.md); the **states do not** — \
         every one is derived from what the named suites actually contain.\n\
         \n\
         ## ⚠ This is a ledger of what was *checked*, not of what is *true*\n\
         \n\
         A row says which suites cover a capability and what those suites assert. It does not say \
         that the result matches Microsoft Office, because **nothing in this workspace has ever \
         been compared against Microsoft Office**. A reader who takes a green ledger for a parity \
         claim has read it as the opposite of what it says.\n\
         \n\
         The distinction has a name in this project and it is printed on every row: an expectation \
         is `SpecCode`, `DocumentedBehaviour` or `EngineDerived`, and **`EngineDerived` means a \
         change detector, not evidence about Office**. A row whose expectations are entirely \
         `EngineDerived` is a row where this engine agrees with itself.\n\
         \n\
         ## The three rules\n\
         \n\
         From `OFFICE_FEATURE_INVENTORY.md` §7, and enforced rather than quoted:\n\
         \n\
         1. **A state is produced by a test, never by assertion.** There is no state field in \
         `xtask/src/ledger/rows.rs` — a row declares evidence, and the generator derives the rest. \
         **Anything nothing tests is `not-started`, regardless of what anyone believes about it.**\n\
         2. **`preserved-not-rendered` is a legitimate, permanent state** for markup the core \
         round-trips faithfully but the renderer does not draw. It is how \"no holes\" stays \
         truthful without pretending every element is drawn on day one.\n\
         3. **The exclusions of §2 are ledger rows too**, in a fifth state — `out-of-scope` — with \
         the reason attached, so a later reader can reopen the decision instead of rediscovering \
         the gap.\n\
         \n\
         ## The five states\n\
         \n\
         | State | Means |\n\
         |---|---|\n",
    );
    for state in State::ALL {
        let _ = writeln!(out, "| `{}` | {} |", state.name(), state.meaning());
    }
    out.push('\n');
}

/// The facts a reader must meet before the summary, each read off the tree rather than remembered.
fn standing_facts(out: &mut String, rows: &[Assessed], evidence: &Evidence) {
    out.push_str("## Standing facts, read off this tree\n\n");

    let approvals = evidence.approvals.len();
    let human = evidence
        .approvals
        .iter()
        .filter(|approval| approval.is_human())
        .count();
    let _ = writeln!(
        out,
        "**Nobody has run Microsoft Office.** The fidelity oracle holds **{approvals}** committed \
         baselines and **{human}** of them carries a human approval; the rest are stamped \
         `approver = generator`, which is a real approval record — the digest binding is live — and \
         **is not a human review**. Parity is judged against real Office on Windows, and \
         LibreOffice is a preliminary change detector whose export is unreliable for shades and \
         gradients. So the parity count is {human} by construction, not by measurement."
    );
    if approvals > 0 {
        out.push_str("\n| Baseline | Approver |\n|---|---|\n");
        for approval in &evidence.approvals {
            let _ = writeln!(out, "| `{}` | `{}` |", approval.specimen, approval.approver);
        }
    }

    let word_scene = evidence.crates.contains("mjx-scene-docx");
    let _ = writeln!(
        out,
        "\n**Word cannot reach pixels at all.** `crates/mjx-scene-docx` {}, so a Word \
         `FragmentTree` has nothing that turns it into a display list. PowerPoint and Excel both \
         do, and their rows say so. This is derived from the crate directory rather than stated: \
         the day the crate exists, this sentence changes.",
        if word_scene {
            "**exists**, and this paragraph is stale — regenerate the ledger"
        } else {
            "does not exist"
        }
    );

    if evidence.provenance_not_read.is_empty() {
        out.push_str("\n**Every provenance ledger in the workspace was read.**\n");
    } else {
        out.push_str(
            "\n**Not every provenance ledger could be read.** These files declare a `Provenance` \
             enum in a form the generator's scanner does not understand — most often because the \
             tiers are passed through a `use` alias rather than written out — so their expectations \
             are **absent from the provenance columns below**. They are named here rather than \
             skipped silently, which is the difference between a known gap and a quiet one:\n\n",
        );
        for path in &evidence.provenance_not_read {
            let _ = writeln!(out, "* `{path}`");
        }
    }

    let uncovered = rows
        .iter()
        .filter(|row| row.state == State::NotStarted)
        .count();
    let _ = writeln!(
        out,
        "\n**{uncovered} of the rows below are `not-started`**, which is the default and is what a capability \
         becomes when the suites do not reach it. That number going *down* because a row was \
         deleted rather than covered would be the one way this document could lie about progress, \
         so the rows are a fixed partition of the inventory and are removed only when the inventory \
         removes them.\n"
    );
}

/// The state summary.
fn counts(out: &mut String, rows: &[Assessed]) {
    out.push_str("## The counts\n\n| State | Rows |\n|---|---:|\n");
    for state in State::ALL {
        let count = rows.iter().filter(|row| row.state == state).count();
        let _ = writeln!(out, "| `{}` | {count} |", state.name());
    }
    let _ = writeln!(out, "| **total** | **{}** |\n", rows.len());
}

/// Where every denominator in this document comes from, and whether it is independent of the rows.
fn denominators(out: &mut String, rows: &[Assessed], evidence: &Evidence) {
    let _ = write!(
        out,
        "## Where the denominators come from\n\
         \n\
         **The row count is not a census.** There are {} rows because that is how this ledger \
         partitions `OFFICE_FEATURE_INVENTORY.md` — a reading of §2 to §6, written by hand in \
         `xtask/src/ledger/rows.rs`. Dividing anything by it produces a fraction of a reading. It \
         is **not** independent of the numerator: the same file decides which rows exist and which \
         suites each one names, so a coarser partition would raise the implemented share without a \
         line of code changing.\n\
         \n\
         The two figures below **are** independent. Each is summed by the generator from a \
         committed derivation with its own regeneration script, neither of which knows this ledger \
         exists — but neither is a denominator the rows divide into, and no percentage in this \
         document is taken against them. They are here to say how large the subject is.\n\
         \n\
         | Independent census | Figure | Source |\n\
         |---|---:|---|\n",
        rows.len()
    );
    for (application, count) in &evidence.commands.per_application {
        let _ = writeln!(
            out,
            "| In-scope controls, {application} | {} | `data/command-surface.tsv` |",
            grouped(*count)
        );
    }
    let _ = writeln!(
        out,
        "| **In-scope controls, all three** | **{}** | `data/command-surface.tsv` |",
        grouped(evidence.commands.total())
    );
    let _ = writeln!(
        out,
        "| **Declared elements, ECMA-376** | **{}** | `data/schema-census.txt` |\n",
        grouped(evidence.declared_elements)
    );
    let _ = writeln!(
        out,
        "The workspace holds **{}** crates and **{}** integration suites, of which the rows below \
         name **{}**. A suite no row names is not a defect — most of them are unit-level gates on \
         one crate's own invariants — but the gap between those two numbers is the honest measure \
         of how much of the test estate this ledger actually reads.\n",
        evidence.crates.len(),
        evidence.suites.len(),
        distinct_evidence(rows).len()
    );
}

/// Every suite any row names, once each, in path order.
fn distinct_evidence(rows: &[Assessed]) -> Vec<&'static str> {
    let mut named: Vec<&'static str> = rows
        .iter()
        .flat_map(|row| row.capability.evidence.iter().copied())
        .collect();
    named.sort_unstable();
    named.dedup();
    named
}

/// The provenance of everything the ledger's evidence rests on.
fn provenance(out: &mut String, rows: &[Assessed], evidence: &Evidence) {
    // **Distinct suites, not distinct rows.** A suite two rows both name declares its expectations
    // once; summing each row's own split would count them twice and produce the absurdity of a
    // "cited" column larger than the "in the workspace" one it is a subset of.
    let mut cited = Split::default();
    for path in distinct_evidence(rows) {
        if let Some(suite) = evidence.suites.get(path) {
            cited.add(suite.split);
        }
    }
    let mut workspace = Split::default();
    for suite in evidence.suites.values() {
        workspace.add(suite.split);
    }

    out.push_str(
        "## The provenance of the evidence\n\
         \n\
         Every layout child since MJXOFF-172 declares, per expectation, where its expected value \
         came from. This is that declaration summed — first across the suites this ledger names, \
         then across the whole workspace.\n\
         \n\
         | Tier | Cited by a row | In the workspace | Means |\n\
         |---|---:|---:|---|\n",
    );
    let describe = |tier: Provenance| match tier {
        Provenance::SpecCode => "ECMA-376, or a schema default",
        Provenance::DocumentedBehaviour => {
            "an external, checkable definition — **the rows that are actually evidence**"
        }
        Provenance::EngineDerived => {
            "read off this engine — **a change detector, not evidence about Office**"
        }
    };
    for (tier, cited_count, workspace_count) in [
        (Provenance::SpecCode, cited.spec, workspace.spec),
        (
            Provenance::DocumentedBehaviour,
            cited.documented,
            workspace.documented,
        ),
        (Provenance::EngineDerived, cited.engine, workspace.engine),
    ] {
        let _ = writeln!(
            out,
            "| `{}` | {cited_count} | {workspace_count} | {} |",
            tier.name(),
            describe(tier)
        );
    }
    let _ = writeln!(
        out,
        "| **total** | **{}** | **{}** | |\n",
        cited.total(),
        workspace.total()
    );
    out.push_str(
        "A row with `—` in its provenance column names no suite that declares any. That is not the \
         same as a row with no evidence: it means the suites covering it never wrote down where \
         their numbers came from, which is a finding about those suites.\n\n",
    );
}

/// Every limitation the workspace declares, and which row carries it.
fn limitations(out: &mut String, rows: &[Assessed]) {
    out.push_str(
        "## Declared limitations\n\
         \n\
         Each of these is written in the module documentation of the suite that **asserts** it, \
         behind the marker `MJX-LEDGER-LIMITATION:`. That placement is the point: a limitation \
         nothing asserts is a claim, and a limitation a suite asserts is a fact that goes red when \
         it stops being true. Every one of them demotes its row to `partial`.\n\
         \n\
         | Row | Limitation | Asserted by |\n\
         |---|---|---|\n",
    );
    let mut any = false;
    for row in rows {
        for (path, text) in &row.limitations {
            any = true;
            let _ = writeln!(
                out,
                "| `{}` | {text} | `{}` |",
                row.capability.id,
                short(path)
            );
        }
    }
    if !any {
        out.push_str("| — | none declared | — |\n");
    }
    out.push('\n');
}

/// The rows themselves, one table per section.
fn table(out: &mut String, rows: &[Assessed]) {
    out.push_str("## The rows\n");
    for section in Section::ALL {
        let section_rows: Vec<&Assessed> = rows
            .iter()
            .filter(|row| row.capability.section == section)
            .collect();
        if section_rows.is_empty() {
            continue;
        }
        let _ = write!(out, "\n### {}\n\n", section.heading());

        if section == Section::Excluded {
            out.push_str("| Row | Excluded surface | State | Reason |\n|---|---|---|---|\n");
            for row in section_rows {
                let _ = writeln!(
                    out,
                    "| `{}` | {} | `{}` | {} |",
                    row.capability.id,
                    row.capability.capability,
                    row.state.name(),
                    row.capability.excluded_because.unwrap_or("—")
                );
            }
            continue;
        }

        out.push_str(
            "| Row | Capability | State | Evidence | Tests | Assertions | `Spec`/`Doc`/`Engine` |\n\
             |---|---|---|---|---:|---:|---|\n",
        );
        for row in section_rows {
            let evidence = if row.capability.evidence.is_empty() {
                "**none**".to_owned()
            } else {
                row.capability
                    .evidence
                    .iter()
                    .map(|path| format!("`{}`", short(path)))
                    .collect::<Vec<_>>()
                    .join("<br>")
            };
            let split = if row.split.total() == 0 {
                "—".to_owned()
            } else {
                format!(
                    "{} / {} / {}",
                    row.split.spec, row.split.documented, row.split.engine
                )
            };
            // A row may be excluded and still belong to a section of its own. §5's calculation
            // engine is: `PLAN.md` puts it out of scope for v1, and filing it under §2's
            // third-party heading would attribute the decision to the wrong place.
            let capability = match row.capability.kind {
                Kind::Excluded => format!(
                    "{} — *excluded:* {}",
                    row.capability.capability,
                    row.capability.excluded_because.unwrap_or("no reason given")
                ),
                _ => row.capability.capability.to_owned(),
            };
            let _ = writeln!(
                out,
                "| `{}` | {capability} | `{}` | {evidence} | {} | {} | {split} |",
                row.capability.id,
                row.state.name(),
                row.tests,
                row.assertions,
            );
        }
    }
}

/// A figure with thousands separators, because `11869` is read one digit at a time and `11,869` is
/// read at a glance — and the inventory these figures come from writes them that way.
fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `crates/mjx-layout-docx/tests/a_thing.rs` as `mjx-layout-docx: a_thing`.
fn short(path: &str) -> String {
    let trimmed = path
        .strip_prefix("crates/")
        .unwrap_or(path)
        .strip_suffix(".rs")
        .unwrap_or(path);
    match trimmed.split_once("/tests/") {
        Some((crate_name, stem)) => format!("{crate_name}: {stem}"),
        None => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_suite_path_is_shortened_to_its_crate_and_stem() {
        assert_eq!(
            short("crates/mjx-layout-docx/tests/a_thing.rs"),
            "mjx-layout-docx: a_thing"
        );
    }

    #[test]
    fn a_figure_is_grouped_at_the_thousand() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(3404), "3,404");
        assert_eq!(grouped(11_869), "11,869");
        assert_eq!(grouped(1_000_000), "1,000,000");
    }
}
