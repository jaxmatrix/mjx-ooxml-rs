//! **No test enumerates by hand a population this repository can enumerate for itself.**
//! (MJXOFF-225.)
//!
//! # The class, and the instance that named it
//!
//! MJXOFF-224 found that `crates/mjx-ooxml-types/src/child_order.rs`'s four safety suites each
//! opened with the same literal:
//!
//! ```text
//! for table in [&DML_MAIN_TYPES[..], &PML_TYPES[..], &DML_CHART_TYPES[..]] {
//! ```
//!
//! That was the whole population when it was written. Six schemas joined `CHILD_ORDER_SCHEMAS`
//! afterwards and none of them joined the list, so four suites whose stated subject was *the
//! child-order tables* were checking three of nine — 516 of 1,335 complex types — and **every count
//! they printed stayed plausible**. The safety property turned out to hold across all nine, so
//! nothing had been faulting conforming markup; the exposure was real and the outcome was clean.
//!
//! MJXOFF-224 fixed that instance by generating `ALL_TABLES`. This file owns the *class*: the
//! shape, wherever else it occurs, and the rule that stops it recurring.
//!
//! # What the sweep looks for
//!
//! A **roster** is a literal `[…]` list, in test code, of two or more elements whose first string
//! literal is a member of a population this repository derives. The scanner walks every tracked
//! `.rs` file, takes the whole of a file under a `tests/` directory and the body of every
//! `#[cfg(test)] mod` elsewhere, and matches each list against the derived populations in
//! [`BasePopulation`]. Matching is on the *first string of each element*, so a roster survives being
//! written as bare strings (`["mjx-pptx", …]`), as tuples (`[("mjx-dml", 10), …]`) or as struct
//! literals (`[AuditedCrate { name: "mjx-chart", … }, …]`) — the three spellings the workspace
//! actually uses, and the reason a grep for `for … in [` finds barely any of them.
//!
//! Every roster it finds must be on [`ROSTERS`], which says **exactly which population it is**. The
//! gate then re-derives that population and requires equality in both directions. A crate added to
//! the workspace, a schema added to the generator, a module whose visibility changes: the
//! derivation moves, the literal does not, and the site fails by name.
//!
//! # Whether a *partial* sweep should be expressible at all
//!
//! MJXOFF-225 asks. The answer this file takes is **yes, but only by naming the subset as a
//! population of its own** — [`Population::CratesAtRanks`] and
//! [`Population::CratesRankedAtOrAbove`] are the two the register needs. Each is derived, so "the
//! rank-2.2 crates" is a *fact about the repository* that moves when the repository moves, rather
//! than a count that was true the day somebody typed it. Where the subset can be derived *in
//! place* it is, and no register row is written at all: `codegen_drift.rs`'s curated re-export
//! sweep now reads `SIMPLE_TYPE_MODULES` and filters on `visibility == "pub(crate)"`, which is
//! strictly better than registering the two module names it used to spell out.
//!
//! There is deliberately **no** variant for a subset chosen by hand and justified in prose. Every
//! roster in this workspace turned out to be the whole of some derivable population once the
//! population was named precisely enough, including the two that looked most arbitrary:
//! `upper_markup_ledger.rs`'s calibration pair is exactly the shared markup below rank 2.2, and
//! `package_writer.rs`'s forbidden-edge list is exactly the crates ranked above `mjx-sml`. The
//! weaker form MJXOFF-225 offers — *state the subset in the name and assert its size against the
//! whole* — is what you fall back to when the subset cannot be derived, and nothing here needed it.
//! If a future site does, that is the moment to add the variant, and the reviewer should ask first
//! whether the subset really has no derivation. The sweep prints how many rosters it compared, so
//! the number lives where it stays true rather than in this paragraph.
//!
//! # The rank table is now load-bearing, so it is now checked
//!
//! Several rosters are *some* subset of the workspace crates rather than all of them, so the gate
//! needs each crate's rank. `CLAUDE.md`'s table is the only place a rank is written down
//! outside `xtask/tests/layering.rs`'s `TIERS`, and until this file it was **mirrored by convention
//! and checked by nothing** — a crate could join the workspace, be ranked in `TIERS`, and never
//! reach the prose table. So [`the_rank_table_in_claude_md_names_every_workspace_member`] holds it
//! against `Cargo.toml`'s `members` in both directions, and `xtask/tests/layering.rs` gained
//! `the_rank_table_in_claude_md_is_the_table_in_this_file`, which compares the prose table against
//! `TIERS` crate by crate, rank and label. That is `doc_gate.rs`'s model — derive twice by
//! independent routes and compare — applied to the table this file's populations are read out of.
//! The comparison lives there rather than here because `TIERS`, `Tier::rank`, `Tier::label` and a
//! JSON reader for `cargo metadata` are all already in that file, and a second copy of any of them
//! would be its own defect.
//!
//! # Anti-vacuity
//!
//! A scanner that has stopped matching reports no rosters and passes everything, which is
//! MJXOFF-221's finding in a different costume. Three things stand against that:
//!
//! * [`the_roster_scanner_matches_the_three_spellings_a_roster_is_written_in`] runs the scanner over
//!   a sample held in this file, and requires it to find all three spellings and to reject two
//!   near-misses. It depends on no corpus at all.
//! * [`every_registered_roster_is_still_in_the_source`] fails on a register row nothing matches, so
//!   the register cannot outlive the code it describes.
//! * Every population derivation carries a floor phrased as *the derivation has stopped matching*,
//!   never as an exact total, and every test prints its counts.
//!
//! # The mutation register
//!
//! Every test below was made to fail by a reachable mutation; the verbatim output is in the pull
//! request for MJXOFF-225.
//!
//! | Mutation | Fails |
//! |---|---|
//! | drop `mjx-vml` from `shared_markup_reachability.rs`'s `SHARED_CRATES` | [`every_roster_in_the_workspace_is_the_whole_of_a_derived_population`] |
//! | move `mjx-omml` to rank 2.1 in `CLAUDE.md`'s table | the same test, on the first site that stops matching |
//! | delete `mjx-mce`'s row from `CLAUDE.md`'s table | [`the_rank_table_in_claude_md_names_every_workspace_member`] |
//! | rename a tier in `CLAUDE.md`'s rank table | `layering.rs`'s `the_rank_table_in_claude_md_is_the_table_in_this_file` |
//! | make the element parser require a bare string literal | [`the_roster_scanner_matches_the_three_spellings_a_roster_is_written_in`] |
//! | point a `ROSTERS` row at a file that has no such roster | both [`every_registered_roster_is_still_in_the_source`] (the row matches nothing) and [`every_roster_in_the_workspace_is_the_whole_of_a_derived_population`] (the site it left is now unregistered) |
//! | give `mjx-sml` an `mjx-omml` dependency | `package_writer.rs`'s own assertion — and **not** the roster it had before MJXOFF-225, which named five of the nine and passed |
//!
//! # What this sweep cannot catch
//!
//! Stated rather than left implicit, because an unstated hole is how a gate becomes vacuous.
//!
//! * **A population nobody named.** [`BasePopulation`] is hand-written, and it is this file's own
//!   instance of the defect it exists to close. A roster over some enumerable thing not on it —
//!   the fixture corpus, the guide set, the preset shape names — is invisible here. The honest
//!   mitigation is that adding a population is one variant and the scanner then sweeps the whole
//!   workspace for it; MJXOFF-252 owns the residue.
//! * **A roster that is not a literal list.** A `match` with one arm per table, a chain of
//!   `assert!`s, a `format!` naming three crates: none is a `[…]`, and none is found.
//! * **A roster whose elements are not strings.** `facade_curation.rs`'s
//!   `const SURFACES: &[&Surface] = &[&DECK, &DOCUMENT, &WORKBOOK]` is the whole of the facade
//!   handle population and is invisible to a scanner that keys on string literals.
//! * **A rank that is wrong in a way no edge exposes.** Moving a leaf crate one rank changes no
//!   edge's direction, so only the population comparison notices — and it notices by reddening a
//!   roster, which is a change somebody reviews rather than a hole nobody sees.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

use xtask::codegen::SIMPLE_TYPE_MODULES;
use xtask::validation::ArtefactFormat;

// ===============================================================================================
// Reading the repository
// ===============================================================================================

/// The workspace root — `xtask/..`.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Reads a repository-relative file, failing loudly: an unreadable file must never become an empty
/// population.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every tracked `.rs` file, repository-relative.
///
/// Derived from `git ls-files` rather than from a walk, so an untracked scratch file cannot join
/// the corpus and a `target/` tree cannot flood it. A `git` failure is fatal, never a skip.
fn tracked_rust_files() -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .arg("ls-files")
        .arg("*.rs")
        .output()
        .expect("running `git ls-files` — the corpus this file sweeps is derived from it");
    assert!(
        output.status.success(),
        "`git ls-files` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git ls-files output is utf-8")
        .lines()
        .map(str::to_owned)
        .collect()
}

// ===============================================================================================
// The rank table, read out of `CLAUDE.md`
// ===============================================================================================

/// A rank as `CLAUDE.md` writes it — `2.1` is `Rank(2, 1)`. Ordered, and compared strictly.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Rank(u8, u8);

impl Rank {
    /// `"2.1"` -> `Rank(2, 1)`.
    fn parse(text: &str) -> Option<Self> {
        let (major, minor) = text.split_once('.')?;
        Some(Self(major.trim().parse().ok()?, minor.trim().parse().ok()?))
    }

    fn label(self) -> String {
        format!("{}.{}", self.0, self.1)
    }
}

/// `CLAUDE.md`'s architecture table: every workspace member, and its rank where it has one.
///
/// A crate outside the shipped graph — `mjx-fixtures`, `mjx-schema-gate`,
/// `mjx-allocation-counter`, `xtask` — sits on the row whose rank cell is not a number, and gets
/// `None`. The crates cell names bindings by directory (`bindings/mjx-python`), so the last path
/// segment is the crate.
fn documented_ranks() -> BTreeMap<String, Option<Rank>> {
    let mut ranks = BTreeMap::new();
    for line in read("CLAUDE.md").lines() {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix('|')
            .and_then(|rest| rest.strip_suffix('|'))
        else {
            continue;
        };
        let cells: Vec<&str> = inner.split('|').map(str::trim).collect();
        if cells.len() != 2 {
            continue;
        }
        let rank = Rank::parse(cells[0].split('—').next().unwrap_or_default().trim());
        let mut named = 0usize;
        for span in code_spans(cells[1]) {
            let name = span.rsplit('/').next().unwrap_or(span);
            if !name.starts_with("mjx-") && name != "xtask" {
                continue;
            }
            ranks.insert(name.to_owned(), rank);
            named += 1;
        }
        assert!(
            rank.is_none() || named > 0,
            "`CLAUDE.md`'s rank table has a row for rank {} that names no crate — the table parser \
             has stopped matching its crates cell",
            cells[0]
        );
    }
    assert!(
        ranks.len() >= 15,
        "only {} crate(s) were read out of `CLAUDE.md`'s rank table; the parser has stopped \
         matching, and every population below would be derived from almost nothing",
        ranks.len()
    );
    ranks
}

/// The text between each pair of backticks on one line.
fn code_spans(text: &str) -> Vec<&str> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        spans.push(&rest[..close]);
        rest = &rest[close + 1..];
    }
    spans
}

/// `Cargo.toml`'s `members`, by crate name — the workspace as Cargo declares it.
///
/// The same derivation `doc_gate.rs` uses, and for the same stated reason: a walk that loses one
/// crate makes every claim about that crate silently unchecked while every count stays plausible.
fn declared_members() -> BTreeSet<String> {
    let manifest = read("Cargo.toml");
    let mut members = BTreeSet::new();
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
            "Cargo.toml's `members` has the glob {directory:?}; this file reads that list as the \
             authoritative crate set and a glob names no crate individually"
        );
        members.insert(directory.rsplit('/').next().unwrap_or(directory).to_owned());
    }
    assert!(
        members.len() >= 15,
        "only {} member(s) were parsed out of Cargo.toml's `members` list; the parser has stopped \
         matching",
        members.len()
    );
    members
}

// ===============================================================================================
// The populations
// ===============================================================================================

/// What a roster's elements are drawn from — the six kinds of thing this workspace can enumerate
/// for itself, and the only kinds the scanner recognises a roster by.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum BasePopulation {
    /// Every workspace member, from `Cargo.toml`'s `members`.
    WorkspaceCrates,
    /// The generated child-order tables, from the committed `child_order.rs`.
    ChildOrderTables,
    /// The generated simple-type modules, from `SIMPLE_TYPE_MODULES`.
    SimpleTypeModules,
    /// `Deck`, `Document`, `Workbook` — from the facade's own module layout.
    FacadeHandleTypes,
    /// Every page of `docs/validation/`.
    ValidationCataloguePages,
    /// The three `--format` tokens, from `ArtefactFormat::all()`.
    ValidationArtefactFormats,
}

impl BasePopulation {
    /// The six, in the order a failure message lists them.
    const ALL: [Self; 6] = [
        Self::WorkspaceCrates,
        Self::ChildOrderTables,
        Self::SimpleTypeModules,
        Self::FacadeHandleTypes,
        Self::ValidationCataloguePages,
        Self::ValidationArtefactFormats,
    ];

    fn describe(self) -> &'static str {
        match self {
            Self::WorkspaceCrates => "the workspace crates",
            Self::ChildOrderTables => "the generated child-order tables",
            Self::SimpleTypeModules => "the generated simple-type modules",
            Self::FacadeHandleTypes => "the facade handle types",
            Self::ValidationCataloguePages => "the validation catalogue pages",
            Self::ValidationArtefactFormats => "the validation artefact formats",
        }
    }

    /// Where the members come from, named in the failure a reader has to act on.
    fn source(self) -> &'static str {
        match self {
            Self::WorkspaceCrates => "Cargo.toml's `members`",
            Self::ChildOrderTables => {
                "the `pub static … : [ChildOrder; N]` tables in \
                 crates/mjx-ooxml-types/src/generated/child_order.rs"
            }
            Self::SimpleTypeModules => "xtask::codegen::SIMPLE_TYPE_MODULES",
            Self::FacadeHandleTypes => {
                "crates/mjx-ooxml/src: each `<name>.rs` that has a `<name>/` beside it"
            }
            Self::ValidationCataloguePages => "the `.md` files under docs/validation/",
            Self::ValidationArtefactFormats => "xtask::validation::ArtefactFormat::all()",
        }
    }

    /// The smallest size that still means the derivation is matching. Phrased as a floor and never
    /// as the total, so it cannot fire in place of the comparison it guards.
    fn floor(self) -> usize {
        match self {
            Self::WorkspaceCrates => 15,
            Self::ChildOrderTables | Self::SimpleTypeModules => 3,
            Self::FacadeHandleTypes
            | Self::ValidationCataloguePages
            | Self::ValidationArtefactFormats => 3,
        }
    }

    /// The members, derived. Compared case-insensitively everywhere, because the facade handles are
    /// spelled `Deck` in one roster and `deck` in the next and both name the same thing.
    fn members(self) -> BTreeSet<String> {
        match self {
            Self::WorkspaceCrates => declared_members(),
            Self::ChildOrderTables => read("crates/mjx-ooxml-types/src/generated/child_order.rs")
                .lines()
                .filter_map(|line| {
                    let rest = line.strip_prefix("pub static ")?;
                    let (name, tail) = rest.split_once(": ")?;
                    tail.starts_with("[ChildOrder;").then(|| name.to_owned())
                })
                .collect(),
            Self::SimpleTypeModules => SIMPLE_TYPE_MODULES
                .iter()
                .map(|module| module.module.to_owned())
                .collect(),
            Self::FacadeHandleTypes => {
                let facade = repository_root().join("crates/mjx-ooxml/src");
                let mut handles = BTreeSet::new();
                for entry in std::fs::read_dir(&facade).expect("crates/mjx-ooxml/src") {
                    let path = entry.expect("a directory entry").path();
                    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                        continue;
                    };
                    if path.extension().is_some_and(|e| e == "rs") && facade.join(stem).is_dir() {
                        handles.insert(stem.to_owned());
                    }
                }
                handles
            }
            Self::ValidationCataloguePages => {
                let mut pages = BTreeSet::new();
                for entry in std::fs::read_dir(repository_root().join("docs/validation"))
                    .expect("docs/validation")
                {
                    let path = entry.expect("a directory entry").path();
                    if path.extension().is_some_and(|e| e == "md") {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            pages.insert(name.to_owned());
                        }
                    }
                }
                pages
            }
            Self::ValidationArtefactFormats => ArtefactFormat::all()
                .into_iter()
                .map(|format| format.extension().to_owned())
                .collect(),
        }
    }
}

/// The exact population a registered roster must equal.
///
/// Each variant is *derived*; see this file's header for why there is no variant for a subset
/// chosen by hand.
#[derive(Clone, Copy)]
enum Population {
    /// Every workspace member.
    WorkspaceCrates,
    /// The crates `CLAUDE.md` puts on the named ranks — `&["2.0", "2.1", "2.2"]` is shared markup.
    CratesAtRanks(&'static [&'static str]),
    /// Every ranked crate at or above `rank`. The shape a crate uses to say what it may not depend
    /// on: everything from the rank above its own upward.
    CratesRankedAtOrAbove(&'static str),
    /// `Deck`, `Document`, `Workbook`.
    FacadeHandleTypes,
}

impl Population {
    /// Which base population a roster over this one is recognised by.
    fn base(self) -> BasePopulation {
        match self {
            Self::WorkspaceCrates | Self::CratesAtRanks(_) | Self::CratesRankedAtOrAbove { .. } => {
                BasePopulation::WorkspaceCrates
            }
            Self::FacadeHandleTypes => BasePopulation::FacadeHandleTypes,
        }
    }

    fn describe(self) -> String {
        match self {
            Self::WorkspaceCrates => "every workspace member".to_owned(),
            Self::CratesAtRanks(ranks) => {
                format!("the crates `CLAUDE.md` ranks {}", ranks.join(", "))
            }
            Self::CratesRankedAtOrAbove(rank) => {
                format!("every crate `CLAUDE.md` ranks at or above {rank}")
            }
            Self::FacadeHandleTypes => "the facade handle types".to_owned(),
        }
    }

    /// Where the members come from, named in the failure a reader has to act on.
    fn source(self) -> &'static str {
        match self {
            Self::WorkspaceCrates => BasePopulation::WorkspaceCrates.source(),
            Self::CratesAtRanks(_) | Self::CratesRankedAtOrAbove(_) => {
                "the rank table in CLAUDE.md, held against Cargo.toml's `members` by \
                 `the_rank_table_in_claude_md_names_every_workspace_member` and against \
                 xtask/tests/layering.rs's TIERS by that file's own comparison"
            }
            Self::FacadeHandleTypes => BasePopulation::FacadeHandleTypes.source(),
        }
    }

    fn members(self) -> BTreeSet<String> {
        match self {
            Self::WorkspaceCrates => BasePopulation::WorkspaceCrates.members(),
            Self::CratesAtRanks(wanted) => {
                let wanted: Vec<Rank> = wanted
                    .iter()
                    .map(|text| {
                        Rank::parse(text).unwrap_or_else(|| panic!("`{text}` is not a rank"))
                    })
                    .collect();
                let members: BTreeSet<String> = documented_ranks()
                    .into_iter()
                    .filter_map(|(name, rank)| {
                        rank.filter(|rank| wanted.contains(rank)).map(|_| name)
                    })
                    .collect();
                assert!(
                    !members.is_empty(),
                    "no crate in `CLAUDE.md`'s table sits on any of {wanted:?}; a rank this file \
                     names has been renumbered or removed"
                );
                members
            }
            Self::CratesRankedAtOrAbove(rank) => {
                let floor = Rank::parse(rank).unwrap_or_else(|| panic!("`{rank}` is not a rank"));
                let members: BTreeSet<String> = documented_ranks()
                    .into_iter()
                    .filter_map(|(name, rank)| rank.filter(|rank| *rank >= floor).map(|_| name))
                    .collect();
                assert!(
                    !members.is_empty(),
                    "no crate in `CLAUDE.md`'s table sits at or above {rank}"
                );
                members
            }
            Self::FacadeHandleTypes => BasePopulation::FacadeHandleTypes.members(),
        }
    }
}

// ===============================================================================================
// The register
// ===============================================================================================

/// One roster in the workspace, and the population it must be the whole of.
struct Roster {
    /// The file it sits in, repository-relative.
    file: &'static str,
    /// What the site is called where a reader would look for it.
    what: &'static str,
    /// The population it must equal, in both directions.
    enumerates: Population,
    /// Why the site names a population at all rather than deriving it in place.
    note: &'static str,
}

/// Every roster the sweep finds, and what each one is.
///
/// This is a register, not an allowlist: a row does not excuse a site, it *states the claim the
/// site makes* and the gate then re-derives that claim. A row nothing matches fails, and a site no
/// row names fails, so neither the register nor the code can drift away from the other.
///
/// A site is named by its file and its population rather than by a line number, because a line
/// number rots the first time somebody adds a paragraph above it — `CLAUDE.md`'s issue-tracker note
/// makes the same point about the tickets themselves.
const ROSTERS: &[Roster] = &[
    Roster {
        file: "xtask/tests/layering.rs",
        what: "TIERS",
        enumerates: Population::WorkspaceCrates,
        note: "the rank table itself: it is the authority the other crate rosters are derived \
               from, and `every_workspace_member_has_a_declared_tier` already holds it against \
               `cargo metadata` in both directions",
    },
    Roster {
        file: "xtask/tests/upper_markup_ledger.rs",
        what: "UPPER_MARKUP",
        enumerates: Population::CratesAtRanks(&["2.2"]),
        note: "the crates that file speaks for — its claim is about rank 2.2 as a whole, so a \
               fourth crate at that rank must join it or the claim stops being true of the tier",
    },
    Roster {
        file: "xtask/tests/upper_markup_ledger.rs",
        what: "CALIBRATION",
        enumerates: Population::CratesAtRanks(&["2.0", "2.1"]),
        note:
            "the crates the impl scanner is calibrated against — exactly the shared markup below \
               rank 2.2, which is where the hand-written pairs live",
    },
    Roster {
        file: "xtask/tests/entry_points.rs",
        what: "HANDLES",
        enumerates: Population::FacadeHandleTypes,
        note: "the handle classes the Python stub is read for",
    },
    Roster {
        file: "xtask/tests/validation_calls.rs",
        what: "FACADE_TYPES",
        enumerates: Population::FacadeHandleTypes,
        note: "the facade types a documented call chain may name",
    },
    Roster {
        file: "xtask/tests/validation_calls.rs",
        what: "the facade module directories",
        enumerates: Population::FacadeHandleTypes,
        note: "the same three, lower-cased: a handle's methods are spread over `<handle>.rs` and \
               the `<handle>/` beside it",
    },
    Roster {
        file: "crates/mjx-ooxml/tests/effective_properties_shape.rs",
        what: "FORMAT_CRATES",
        enumerates: Population::CratesAtRanks(&["3.0"]),
        note: "every format crate carries docs/effective_properties.md, so the roster is the \
               format tier and a fourth format crate must answer the same requirement",
    },
    Roster {
        file: "crates/mjx-ooxml/tests/shared_markup_reachability.rs",
        what: "SHARED_CRATES",
        enumerates: Population::CratesAtRanks(&["2.0", "2.1", "2.2"]),
        note: "the shared markup a format crate may model — ranks 2.0 through 2.2, which is what \
               `CLAUDE.md` means by \"shared markup is not flat\"",
    },
    Roster {
        file: "crates/mjx-ooxml/tests/shared_markup_reachability.rs",
        what: "FORMAT_CRATES",
        enumerates: Population::CratesAtRanks(&["3.0"]),
        note: "the crates whose manifests the dependency grid is read out of",
    },
    Roster {
        file: "crates/mjx-ooxml/tests/shared_markup_reachability.rs",
        what: "the three surface signature sets",
        enumerates: Population::FacadeHandleTypes,
        note: "table 2 has one column per facade handle, so the columns are the handles",
    },
    Roster {
        file: "crates/mjx-sml/tests/package_writer.rs",
        what: "the forbidden dependency names",
        enumerates: Population::CratesRankedAtOrAbove("2.2"),
        note: "what `mjx-sml` (rank 2.1) may not depend on: everything ranked above it. Until \
               MJXOFF-225 the list named five of the nine and read as though it named all of them",
    },
];

// ===============================================================================================
// The scanner
// ===============================================================================================

/// One roster found in the source.
#[derive(Debug)]
struct Found {
    file: String,
    line: usize,
    /// The first string literal of each element, in source order.
    elements: Vec<String>,
    base: BasePopulation,
}

/// The regions of `source` that are test code: all of it under a `tests/` directory, and the body
/// of every `#[cfg(test)] mod` elsewhere.
fn test_regions(source: &str, path: &str) -> Vec<(usize, usize)> {
    if path.contains("/tests/") {
        return vec![(0, source.len())];
    }
    let bytes = source.as_bytes();
    let mut regions = Vec::new();
    let mut from = 0usize;
    while let Some(at) = source[from..].find("#[cfg(test)]") {
        let start = from + at;
        from = start + 1;
        let Some(brace) = source[start..].find('{').map(|offset| start + offset) else {
            break;
        };
        // `#[cfg(test)]` on anything but a module has no braces of its own worth walking, but
        // walking one costs nothing and a `mod` written with an attribute between is still caught.
        let mut depth = 0usize;
        let mut index = brace;
        while index < bytes.len() {
            match bytes[index] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        regions.push((start, index.min(bytes.len())));
    }
    regions
}

/// The end of the `[…]` opening at `start`, skipping string literals so a `]` inside one does not
/// close the list.
fn matching_bracket(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// The first string literal of every top-level element of a `[…]` body, or `None` if any element
/// has none.
///
/// Taking the *first* string is what lets one scanner see all three spellings a roster is written
/// in: `"mjx-pptx"`, `("mjx-dml", 10)` and `AuditedCrate { name: "mjx-chart", … }`.
fn element_heads(inner: &str) -> Option<Vec<String>> {
    let bytes = inner.as_bytes();
    let mut heads = Vec::new();
    let mut depth = 0usize;
    let mut current: Option<String> = None;
    let mut index = 0usize;
    let mut started = false;
    let mut finish = |current: &mut Option<String>, started: &mut bool| -> bool {
        if !*started {
            return true;
        }
        *started = false;
        match current.take() {
            Some(text) => {
                heads.push(text);
                true
            }
            None => false,
        }
    };
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                started = true;
                let open = index + 1;
                index = open;
                while index < bytes.len() && bytes[index] != b'"' {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
                if index >= bytes.len() {
                    return None;
                }
                if current.is_none() {
                    current = Some(inner.get(open..index)?.to_owned());
                }
            }
            b'(' | b'[' | b'{' => {
                started = true;
                depth += 1;
            }
            b')' | b']' | b'}' => {
                started = true;
                depth = depth.checked_sub(1)?;
            }
            b',' if depth == 0 => {
                if !finish(&mut current, &mut started) {
                    return None;
                }
            }
            byte if !byte.is_ascii_whitespace() => started = true,
            _ => {}
        }
        index += 1;
    }
    if !finish(&mut current, &mut started) {
        return None;
    }
    Some(heads)
}

/// Every roster in `source`, with the base population each one is drawn from.
fn rosters_in(
    path: &str,
    source: &str,
    populations: &BTreeMap<BasePopulation, BTreeSet<String>>,
) -> Vec<Found> {
    let regions = test_regions(source, path);
    if regions.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut seen_lines = BTreeSet::new();
    for (offset, _) in source.match_indices('[') {
        if !regions.iter().any(|(a, b)| *a <= offset && offset < *b) {
            continue;
        }
        let Some(end) = matching_bracket(source, offset) else {
            continue;
        };
        let Some(inner) = source.get(offset + 1..end) else {
            continue;
        };
        if !inner.contains('"') {
            continue;
        }
        let Some(heads) = element_heads(inner) else {
            continue;
        };
        let distinct: BTreeSet<String> = heads.iter().map(|head| head.to_lowercase()).collect();
        if distinct.len() < 2 {
            continue;
        }
        let Some(base) = BasePopulation::ALL.into_iter().find(|base| {
            let members = &populations[base];
            distinct.iter().all(|head| members.contains(head))
        }) else {
            continue;
        };
        let line = source[..offset].lines().count();
        if !seen_lines.insert(line) {
            continue; // a nested list inside one already reported
        }
        found.push(Found {
            file: path.to_owned(),
            line,
            elements: heads,
            base,
        });
    }
    found
}

/// The derived members of every base population, lower-cased, keyed for the scanner.
fn derived_populations() -> BTreeMap<BasePopulation, BTreeSet<String>> {
    BasePopulation::ALL
        .into_iter()
        .map(|base| {
            let members = base.members();
            assert!(
                members.len() >= base.floor(),
                "{} yielded only {} member(s) from {} — the derivation has stopped matching, and a \
                 roster over it could not be recognised at all",
                base.describe(),
                members.len(),
                base.source()
            );
            (
                base,
                members
                    .iter()
                    .map(|member| member.to_lowercase())
                    .collect::<BTreeSet<String>>(),
            )
        })
        .collect()
}

/// This file, as `git ls-files` spells it.
const THIS_FILE: &str = "xtask/tests/derived_rosters.rs";

/// Every roster in the workspace.
///
/// **This file itself is excluded, and that is not a convenience.** The `SAMPLE` in
/// [`the_roster_scanner_matches_the_three_spellings_a_roster_is_written_in`] is three rosters
/// written into this source on purpose, each deliberately *partial* — two workspace crates, never
/// the population — because a calibration that used a whole population could not tell a scanner
/// that matches from one that does not. A partial roster is exactly what the sweep below rejects,
/// and no `ROSTERS` row could truthfully name it: a row asserts *"this list is the whole of that
/// population"*, which is the one thing the sample must not be. So the fixture is kept out of the
/// corpus it calibrates, and the calibration still runs — it calls [`rosters_in`] on `SAMPLE`
/// directly, under a synthetic path.
///
/// The cost is stated rather than hidden: a real roster written in *this* file is not swept. That
/// is the same blind spot MJXOFF-252 already owns for [`BasePopulation`] and [`ROSTERS`], which are
/// hand-written lists this gate cannot check either.
fn workspace_rosters() -> Vec<Found> {
    let populations = derived_populations();
    let tracked = tracked_rust_files();
    assert!(
        tracked.iter().any(|path| path == THIS_FILE),
        "`{THIS_FILE}` is not in `git ls-files`, so the exclusion below is excluding nothing. \
         Either this file moved and the constant did not, or the corpus walk has stopped matching."
    );
    let mut found = Vec::new();
    for path in tracked {
        if path == THIS_FILE {
            continue;
        }
        let source = read(&path);
        found.extend(rosters_in(&path, &source, &populations));
    }
    found
}

// ===============================================================================================
// The gates
// ===============================================================================================

/// **The scanner still sees a roster written any of the three ways this workspace writes one.**
///
/// A scanner that has stopped matching finds nothing and passes everything, and it is
/// indistinguishable from a workspace that has no rosters left — MJXOFF-221's finding, and the
/// reason `upper_markup_ledger.rs` calibrates its own scanner against code it knows the answer for.
/// This calibration depends on no corpus: the sample is here, the answer is here.
#[test]
fn the_roster_scanner_matches_the_three_spellings_a_roster_is_written_in() {
    const SAMPLE: &str = r#"
        const BARE: [&str; 2] = ["mjx-pptx", "mjx-docx"];
        const TUPLES: &[(&str, usize)] = &[("mjx-pptx", 1), ("mjx-docx", 2)];
        const STRUCTS: &[Audited] = &[
            Audited { name: "mjx-pptx", floor: 1 },
            Audited { name: "mjx-docx", floor: 2 },
        ];
        const NOT_A_ROSTER: [&str; 2] = ["mjx-pptx", "not-a-crate"];
        const ONE_NAME_TWICE: [&str; 2] = ["mjx-pptx", "mjx-pptx"];
    "#;

    let populations = derived_populations();
    let found = rosters_in("crates/x/tests/sample.rs", SAMPLE, &populations);
    let lines: Vec<usize> = found.iter().map(|roster| roster.line).collect();
    assert_eq!(
        found.len(),
        3,
        "the scanner found {} roster(s) in the sample instead of the three it holds, at lines \
         {lines:?}. It has stopped matching one of the spellings a roster is written in, and the \
         workspace sweep below would report the difference as a clean bill.\n{found:#?}",
        found.len()
    );
    for roster in &found {
        assert_eq!(
            roster.base,
            BasePopulation::WorkspaceCrates,
            "the sample's rosters name crates, and one was recognised as {} instead",
            roster.base.describe()
        );
        assert_eq!(
            roster.elements,
            vec!["mjx-pptx".to_owned(), "mjx-docx".to_owned()],
            "the element parser read {:?} out of a two-crate roster",
            roster.elements
        );
    }
    println!(
        "roster scanner: 3 of 3 spellings matched, and both near-misses rejected (a list with a \
         non-member, and a list naming one crate twice)"
    );
}

/// **`CLAUDE.md`'s rank table names every workspace member, and nothing else.**
///
/// The table was mirrored by convention and checked by nothing until MJXOFF-225, and this file's
/// crate populations are read out of it. `doc_gate.rs` states the rule this follows: a set built by
/// a walk is held against `Cargo.toml`'s own `members` rather than against a number, because losing
/// one crate is the failure that actually happens and no total shows it.
#[test]
fn the_rank_table_in_claude_md_names_every_workspace_member() {
    let ranks = documented_ranks();
    let members = declared_members();

    let missing: Vec<&String> = members
        .iter()
        .filter(|member| !ranks.contains_key(*member))
        .collect();
    assert!(
        missing.is_empty(),
        "`CLAUDE.md`'s rank table does not name {} of the {} crate(s) Cargo.toml declares: {:?}. \
         Every population this file derives from a rank silently loses them, and the layering rule \
         has no rank to check their edges against.",
        missing.len(),
        members.len(),
        missing
    );

    let stray: Vec<&String> = ranks
        .keys()
        .filter(|name| !members.contains(*name))
        .collect();
    assert!(
        stray.is_empty(),
        "`CLAUDE.md`'s rank table names {:?}, which Cargo.toml's `members` does not declare — \
         either a crate was removed and the table kept its row, or the parser is matching \
         something that is not a crate",
        stray
    );

    let mut per_rank: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for (name, rank) in &ranks {
        per_rank
            .entry(rank.map_or_else(|| "outside the graph".to_owned(), Rank::label))
            .or_default()
            .push(name);
    }
    println!(
        "CLAUDE.md's rank table: {} crates over {} ranks, matching Cargo.toml's members exactly: \
         {per_rank:?}",
        ranks.len(),
        per_rank.len()
    );
}

/// **Every roster in the workspace is the whole of a population this repository derives.**
///
/// This is MJXOFF-225's question. A site that names three of nine tables, five of nine crates or
/// two of three handles fails here with both directions of the difference spelled out.
#[test]
fn every_roster_in_the_workspace_is_the_whole_of_a_derived_population() {
    let found = workspace_rosters();
    assert!(
        found.len() >= 8,
        "the sweep found only {} roster(s) in the workspace. The scanner has stopped matching, and \
         with nothing to compare this test passes exactly as a working one does — which is the \
         defect it exists to close.",
        found.len()
    );

    let mut per_population: BTreeMap<&str, usize> = BTreeMap::new();
    for roster in &found {
        let candidates: Vec<&Roster> = ROSTERS
            .iter()
            .filter(|row| row.file == roster.file && row.enumerates.base() == roster.base)
            .collect();
        assert!(
            !candidates.is_empty(),
            "{}:{} holds a roster over {} that no row of `ROSTERS` names: {:?}.\n\nA literal list \
             of things this repository can enumerate is the shape MJXOFF-224 found in \
             child_order.rs — correct the day it was written and silently partial ever after. \
             Either derive the population where the list stands, or add a row saying exactly which \
             derived population it is, so that this gate can re-derive it.",
            roster.file,
            roster.line,
            roster.base.describe(),
            roster.elements
        );

        let written: BTreeSet<String> = roster
            .elements
            .iter()
            .map(|element| element.to_lowercase())
            .collect();
        let mut agreed: Option<()> = None;
        for row in &candidates {
            let expected: BTreeSet<String> = row
                .enumerates
                .members()
                .iter()
                .map(|member| member.to_lowercase())
                .collect();
            if expected == written {
                agreed = Some(());
                break;
            }
        }
        if agreed.is_none() {
            let row = candidates[0];
            let expected: BTreeSet<String> = row
                .enumerates
                .members()
                .iter()
                .map(|member| member.to_lowercase())
                .collect();
            let missing: Vec<&String> = expected.difference(&written).collect();
            let stray: Vec<&String> = written.difference(&expected).collect();
            panic!(
                "{}:{} — `{}` claims to be {}, and it is not.\n  it does not name: {:?}\n  it \
                 names, and the population does not hold: {:?}\n\n{}\n\nThe population is derived \
                 from {}; either the roster grew a member it should not have, or the repository \
                 grew one the roster never learned about — which is exactly how child_order.rs's \
                 four suites came to sweep three of nine tables.",
                roster.file,
                roster.line,
                row.what,
                row.enumerates.describe(),
                missing,
                stray,
                row.note,
                row.enumerates.source()
            );
        }
        *per_population.entry(roster.base.describe()).or_default() += 1;
    }

    println!(
        "derived rosters: {} roster(s) in {} file(s), every one equal to the population it claims, \
         over {} registered row(s): {per_population:?}",
        found.len(),
        found
            .iter()
            .map(|roster| roster.file.as_str())
            .collect::<BTreeSet<&str>>()
            .len(),
        ROSTERS.len()
    );
}

/// **Every row of [`ROSTERS`] still names a roster that is in the source.**
///
/// The register is an escape hatch, and an escape hatch that cannot rot is the only kind worth
/// having — `doc_gate.rs`'s retired-path register is the shape. A row for a site somebody deleted
/// would sit here forever asserting nothing.
#[test]
fn every_registered_roster_is_still_in_the_source() {
    let found = workspace_rosters();
    let mut unmatched = Vec::new();
    for row in ROSTERS {
        let hit = found
            .iter()
            .any(|roster| roster.file == row.file && roster.base == row.enumerates.base());
        if !hit {
            unmatched.push(format!("{} — {}", row.file, row.what));
        }
    }
    assert!(
        unmatched.is_empty(),
        "{} row(s) of `ROSTERS` name a roster the sweep no longer finds:\n  {}\nEither the site was \
         derived in place, in which case delete the row, or the scanner has stopped seeing it.",
        unmatched.len(),
        unmatched.join("\n  ")
    );
    println!(
        "roster register: all {} row(s) still name a roster in the source",
        ROSTERS.len()
    );
}
