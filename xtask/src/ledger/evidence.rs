//! **Reading the evidence off the disk** — the half of the ledger that looks, and never decides.
//!
//! Nothing here knows what a capability is or what any of the five states mean. It answers four
//! questions about a tree of files and stops:
//!
//! 1. **Which suites exist**, and for each one: how many `#[test]` functions it declares, how many
//!    assertions it makes, and which crate it lives in.
//! 2. **Which suites declare a limitation** — a `MJX-LEDGER-LIMITATION:` line in a module doc
//!    comment, written beside the assertion that proves it.
//! 3. **What provenance the suites declare** — the `SpecCode` / `DocumentedBehaviour` /
//!    `EngineDerived` split MJXOFF-172 onward records per expectation, read out of the ledgers that
//!    hold it.
//! 4. **The three independent facts** the report needs and must not restate from memory: the
//!    command-surface and schema censuses, and the oracle's approval records.
//!
//! # Why this is a text scan and not a parse
//!
//! It reads Rust source with a line scanner. That is the weakest part of this pipeline and it is
//! written to fail loudly rather than quietly: comments are stripped before anything is counted, a
//! provenance ledger whose rows do not pair up is **not read** rather than read wrongly, and a file
//! that declares a `Provenance` enum this scanner cannot read is **named in the generated
//! document** instead of being skipped. A miscount that shows up as a missing row is a defect a
//! reader can see; one that shows up as a confident `implemented` is the defect this whole child
//! exists to prevent.
//!
//! Every direction the scan can be wrong in is the conservative one. Fewer assertions counted means
//! a *lower* state, never a higher one; provenance not read means a row reports no declared
//! evidence rather than reporting borrowed evidence.
//!
//! The alternative — having the suites emit a machine-readable artefact of their own — would put
//! the ledger downstream of a `cargo test` run, and the ticket is explicit that the generator does
//! not run the renderer and judges nothing itself.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Context, Result};

/// The marker a suite writes in its module documentation to declare a limitation it asserts.
///
/// It goes **in the suite**, not in a source module and not in a table here, because the project's
/// first ledger rule is that a state is produced by a test. A limitation nothing asserts is a claim;
/// a limitation a suite asserts is a fact, and the marker is where the fact says so.
pub(crate) const LIMITATION_MARKER: &str = "MJX-LEDGER-LIMITATION:";

/// Crates whose suites exercise the **rendering** path.
///
/// It decides one thing: whether a document capability has anything at all that draws it, which is
/// the difference between `implemented` and `preserved-not-rendered`. A crate absent from this list
/// is model tier, which can only ever *lower* a state — so an unclassified newcomer is reported
/// conservatively rather than optimistically.
///
/// Every name here is checked against the real workspace by [`scan`], because a stale entry would
/// silently demote everything it was meant to promote.
pub(crate) const RENDERING_TIER: &[&str] = &[
    "mjx-text",
    "mjx-layout",
    "mjx-layout-chart",
    "mjx-layout-docx",
    "mjx-layout-pptx",
    "mjx-layout-xlsx",
    "mjx-scene",
    "mjx-scene-pptx",
    "mjx-scene-xlsx",
    "mjx-geometry",
    "mjx-paint",
    "mjx-view",
    "mjx-canvas-harness",
    "mjx-render-oracle",
    "mjx-reference-pack",
];

/// Where one expectation came from, in the vocabulary MJXOFF-172 established and every layout child
/// since has used.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Provenance {
    /// ECMA-376, or a schema default.
    SpecCode,
    /// An external, checkable definition that is not this repository's.
    DocumentedBehaviour,
    /// Read off this engine. **A change detector, not evidence about Office.**
    EngineDerived,
}

impl Provenance {
    /// The name this tier is written under in the suites and in the generated document.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::SpecCode => "SpecCode",
            Self::DocumentedBehaviour => "DocumentedBehaviour",
            Self::EngineDerived => "EngineDerived",
        }
    }
}

/// How many expectations of each tier a suite — or a set of them — rests on.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Split {
    /// `SpecCode` rows.
    pub(crate) spec: usize,
    /// `DocumentedBehaviour` rows — **the ones that are actually evidence**.
    pub(crate) documented: usize,
    /// `EngineDerived` rows — change detectors, and not evidence about Office.
    pub(crate) engine: usize,
}

impl Split {
    /// Every declared expectation, of any tier.
    pub(crate) fn total(self) -> usize {
        self.spec + self.documented + self.engine
    }

    /// Adds another suite's declarations to this one.
    pub(crate) fn add(&mut self, other: Self) {
        self.spec += other.spec;
        self.documented += other.documented;
        self.engine += other.engine;
    }

    /// Counts one row.
    fn record(&mut self, provenance: Provenance) {
        match provenance {
            Provenance::SpecCode => self.spec += 1,
            Provenance::DocumentedBehaviour => self.documented += 1,
            Provenance::EngineDerived => self.engine += 1,
        }
    }
}

/// One integration suite, as read off the disk.
#[derive(Clone, Debug)]
pub(crate) struct Suite {
    /// The crate directory it lives in, e.g. `mjx-layout-docx`.
    pub(crate) crate_name: String,
    /// `#[test]` functions it declares.
    pub(crate) tests: usize,
    /// Assertion-macro invocations it makes, comments excluded.
    pub(crate) assertions: usize,
    /// Limitations it declares, in file order, with the marker stripped.
    pub(crate) limitations: Vec<String>,
    /// The provenance of the expectations declared *for* this suite, wherever they are declared.
    pub(crate) split: Split,
}

impl Suite {
    /// Whether this suite exercises the rendering path.
    pub(crate) fn renders(&self) -> bool {
        RENDERING_TIER.contains(&self.crate_name.as_str())
    }

    /// Whether it checks anything at all.
    ///
    /// A suite that asserts nothing is not evidence, however many `#[test]` functions it declares.
    /// **This is trap (b) of the ticket** — the suite that is green precisely because the work is
    /// skipped — caught at the only place a generator that does not run the tests can see it.
    pub(crate) fn checks_something(&self) -> bool {
        self.assertions > 0
    }
}

/// One approval record from the fidelity oracle's committed baselines.
#[derive(Clone, Debug)]
pub(crate) struct Approval {
    /// The specimen it approves.
    pub(crate) specimen: String,
    /// `generator` or `human`.
    pub(crate) approver: String,
}

impl Approval {
    /// Whether a person looked at the image.
    pub(crate) fn is_human(&self) -> bool {
        self.approver == "human"
    }
}

/// The in-scope command count of each application, summed from the committed derivation.
#[derive(Clone, Debug)]
pub(crate) struct CommandCensus {
    /// Application, in-scope controls, sorted by application.
    pub(crate) per_application: Vec<(String, u64)>,
}

impl CommandCensus {
    /// Every in-scope control of every application.
    pub(crate) fn total(&self) -> u64 {
        self.per_application.iter().map(|(_, count)| count).sum()
    }
}

/// Everything the assessor is allowed to look at.
#[derive(Clone, Debug)]
pub(crate) struct Evidence {
    /// Every integration suite in the workspace, by path relative to the root.
    pub(crate) suites: BTreeMap<String, Suite>,
    /// Files that declare a `Provenance` enum in a form this scanner **could not** read. Named
    /// rather than skipped: a provenance ledger silently dropped is exactly the rot this child was
    /// told not to repeat.
    pub(crate) provenance_not_read: Vec<String>,
    /// The oracle's committed approval records.
    pub(crate) approvals: Vec<Approval>,
    /// The published command surface, from `docs/client-platform/data/command-surface.tsv`.
    pub(crate) commands: CommandCensus,
    /// Declared elements, from `docs/client-platform/data/schema-census.txt`.
    pub(crate) declared_elements: u64,
    /// Every crate directory in the workspace.
    pub(crate) crates: BTreeSet<String>,
}

impl Evidence {
    /// The suite at a workspace-relative path, or an error naming what the ledger asked for.
    ///
    /// **This is the liveness check.** A ledger row that names a suite which no longer exists fails
    /// the build here, at the point of the lookup, rather than being emitted as a state derived
    /// from nothing — which is the defect `UNCOVERED_SCHEMAS` shipped with and the one the ticket
    /// names by name.
    pub(crate) fn suite(&self, path: &str) -> Result<&Suite> {
        self.suites.get(path).with_context(|| {
            format!(
                "the parity ledger names `{path}` as evidence, and there is no such suite in the \
                 workspace. Either the suite was renamed or deleted and the row now points at \
                 nothing, or the path is misspelt. A row may not outlive its evidence: fix the \
                 path in `xtask/src/ledger/rows.rs`, or delete the evidence and let the capability \
                 fall back to `not-started`, which is what it has become."
            )
        })
    }

    /// Every limitation any suite in the workspace declares, as `(suite path, text)`.
    pub(crate) fn declared_limitations(&self) -> Vec<(&str, &str)> {
        let mut all = Vec::new();
        for (path, suite) in &self.suites {
            for limitation in &suite.limitations {
                all.push((path.as_str(), limitation.as_str()));
            }
        }
        all
    }
}

/// Reads everything the ledger rests on out of a workspace tree.
pub(crate) fn scan(root: &Path) -> Result<Evidence> {
    let crates = crate_directories(root)?;

    for name in RENDERING_TIER {
        if !crates.contains(*name) {
            bail!(
                "`RENDERING_TIER` names `{name}`, which is not a crate in this workspace. That \
                 classification decides whether a capability is `implemented` or \
                 `preserved-not-rendered`, so a stale name here would silently demote everything \
                 it was meant to promote."
            );
        }
    }

    let sources = read_suite_sources(root, &crates)?;
    let mut suites: BTreeMap<String, Suite> = sources
        .iter()
        .map(|(path, (crate_name, source))| (path.clone(), read_suite(crate_name, source)))
        .collect();
    let provenance_not_read = attach_provenance(&sources, &mut suites)?;

    Ok(Evidence {
        suites,
        provenance_not_read,
        approvals: scan_approvals(root)?,
        commands: read_command_census(root)?,
        declared_elements: read_schema_census(root)?,
        crates,
    })
}

/// Every directory directly under `crates/` that carries a manifest.
fn crate_directories(root: &Path) -> Result<BTreeSet<String>> {
    let directory = root.join("crates");
    let mut names = BTreeSet::new();
    let entries = std::fs::read_dir(&directory)
        .with_context(|| format!("reading {}", directory.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        if !entry.path().join("Cargo.toml").is_file() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            names.insert(name.to_owned());
        }
    }
    if names.is_empty() {
        bail!(
            "no crates found under {} — the ledger would then derive `not-started` for every row, \
             which would be a confident statement about an empty read rather than about the code",
            directory.display()
        );
    }
    Ok(names)
}

/// The source of every integration suite: a `.rs` file **directly** inside some crate's `tests/`.
///
/// Depth one is deliberate. `tests/support/mod.rs` and `tests/common/mod.rs` are helpers compiled
/// *into* a suite rather than suites, and counting their assertions against a capability would
/// credit a fixture builder with checking something.
///
/// Returns `path -> (crate name, source)`, sorted, so the whole tree is read exactly once.
type SuiteSources = BTreeMap<String, (String, String)>;

fn read_suite_sources(root: &Path, crates: &BTreeSet<String>) -> Result<SuiteSources> {
    let mut sources = SuiteSources::new();
    for name in crates {
        let directory = root.join("crates").join(name).join("tests");
        if !directory.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(&directory)
            .with_context(|| format!("reading {}", directory.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let source = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            sources.insert(
                format!("crates/{name}/tests/{stem}.rs"),
                (name.clone(), source),
            );
        }
    }
    Ok(sources)
}

/// Counts what one suite declares.
///
/// # A limitation runs to the end of its paragraph
///
/// The marker begins the text and the doc-comment lines after it continue it, up to the first blank
/// one. A one-line marker would be either unreadably long or uselessly terse, and a limitation a
/// reader cannot act on is barely better than none — so the sentence is written the way the rest of
/// the module documentation is, and joined back up here.
fn read_suite(crate_name: &str, source: &str) -> Suite {
    let mut tests = 0;
    let mut assertions = 0;
    let mut limitations: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        // The marker is read *before* comments are stripped: it is written as prose, in a doc
        // comment, beside the reasoning — which is the whole reason it is worth trusting.
        if trimmed.starts_with("//") {
            if let Some((_, text)) = trimmed.split_once(LIMITATION_MARKER) {
                if let Some(complete) = pending.take() {
                    limitations.push(complete);
                }
                pending = Some(text.trim().to_owned());
                continue;
            }
            if let Some(accumulating) = pending.as_mut() {
                let body = trimmed
                    .trim_start_matches('/')
                    .trim_start_matches('!')
                    .trim();
                if body.is_empty() {
                    limitations.push(pending.take().unwrap_or_default());
                } else {
                    accumulating.push(' ');
                    accumulating.push_str(body);
                }
            }
            continue;
        }
        if let Some(complete) = pending.take() {
            limitations.push(complete);
        }
        let code = strip_comment(trimmed);
        if code.trim() == "#[test]" {
            tests += 1;
        }
        assertions += count_assertions(code);
    }
    if let Some(complete) = pending.take() {
        limitations.push(complete);
    }

    for limitation in &mut limitations {
        while limitation.ends_with('.') {
            limitation.pop();
        }
    }

    Suite {
        crate_name: crate_name.to_owned(),
        tests,
        assertions,
        limitations,
        split: Split::default(),
    }
}

/// Everything before a `//`, so prose about assertions is not counted as one.
///
/// It does not understand a `//` inside a string literal. That can only ever *drop* code from the
/// scan, never invent it, so the failure direction is the safe one: fewer assertions counted means
/// a lower state, never a higher one.
fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(at) => &line[..at],
        None => line,
    }
}

/// Assertion-macro invocations on one line of code.
fn count_assertions(code: &str) -> usize {
    const MACROS: [&str; 4] = ["assert!(", "assert_eq!(", "assert_ne!(", "assert_matches!("];
    let mut count = 0;
    for macro_name in MACROS {
        let mut offset = 0;
        while let Some(at) = code[offset..].find(macro_name) {
            let at = offset + at;
            // `debug_assert!(` ends in `assert!(`, and an identifier character in front of the
            // match is what tells the two apart.
            let preceded = code[..at]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_alphanumeric() || character == '_');
            if !preceded {
                count += 1;
            }
            offset = at + macro_name.len();
        }
    }
    count
}

/// Attaches every declared provenance row to the suite it is about.
///
/// The canonical form is the one MJXOFF-172 wrote and MJXOFF-178 copied: a `const LEDGER` of rows,
/// each carrying `suite: "<name>"` and then `provenance: Provenance::<tier>`. A row may name a
/// single test as `test_name (suite)`, which both ledgers already do, and which resolves to the
/// suite in the parentheses.
///
/// Returns the files that declare a `Provenance` enum in some **other** form. They are not read and
/// they are not ignored: the generated document names them, so a reader can see exactly how much of
/// the workspace's declared provenance reached the ledger.
fn attach_provenance(
    sources: &SuiteSources,
    suites: &mut BTreeMap<String, Suite>,
) -> Result<Vec<String>> {
    let mut splits: BTreeMap<String, Split> = BTreeMap::new();
    let mut not_read = Vec::new();

    for (path, (crate_name, source)) in sources {
        if !source.contains("enum Provenance") {
            continue;
        }
        match read_provenance_ledger(crate_name, source) {
            Some(rows) => {
                for (suite_path, provenance) in rows {
                    splits.entry(suite_path).or_default().record(provenance);
                }
            }
            None => not_read.push(path.clone()),
        }
    }

    for (path, split) in splits {
        match suites.get_mut(&path) {
            Some(suite) => suite.split = split,
            None => bail!(
                "a provenance ledger declares expectations for `{path}`, which is not a suite in \
                 this workspace"
            ),
        }
    }

    Ok(not_read)
}

/// Reads the canonical `suite` / `provenance` pairs out of one file, or `None` if it is not in that
/// form.
fn read_provenance_ledger(crate_name: &str, source: &str) -> Option<Vec<(String, Provenance)>> {
    let mut rows = Vec::new();
    let mut pending: Option<String> = None;
    let mut saw_suite_field = false;

    for line in source.lines() {
        let code = strip_comment(line.trim());
        let code = code.trim();
        if let Some(rest) = code.strip_prefix("suite: \"") {
            let (name, _) = rest.split_once('"')?;
            saw_suite_field = true;
            // A `suite` field that never met a `provenance` field is a shape this scanner must
            // refuse rather than silently attribute to the wrong suite.
            if pending.is_some() {
                return None;
            }
            // `test_name (suite)` points at one assertion inside a suite.
            let stem = name
                .rsplit_once(" (")
                .map_or(name, |(_, file)| file.trim_end_matches(')'));
            pending = Some(format!("crates/{crate_name}/tests/{stem}.rs"));
            continue;
        }
        if let Some(rest) = code.strip_prefix("provenance: Provenance::") {
            let provenance = match rest.trim_end_matches(',').trim() {
                "SpecCode" => Provenance::SpecCode,
                "DocumentedBehaviour" => Provenance::DocumentedBehaviour,
                "EngineDerived" => Provenance::EngineDerived,
                _ => return None,
            };
            rows.push((pending.take()?, provenance));
        }
    }

    if !saw_suite_field || pending.is_some() || rows.is_empty() {
        return None;
    }
    Some(rows)
}

/// The oracle's committed approval records, one per baselined specimen.
fn scan_approvals(root: &Path) -> Result<Vec<Approval>> {
    let directory = root.join("crates/mjx-render-oracle/baselines");
    let mut approvals = Vec::new();
    if !directory.is_dir() {
        return Ok(approvals);
    }
    let entries = std::fs::read_dir(&directory)
        .with_context(|| format!("reading {}", directory.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        let path = entry.path().join("APPROVAL");
        if !path.is_file() {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let specimen = approval_field(&contents, "specimen")
            .with_context(|| format!("{} names no specimen", path.display()))?;
        let approver = approval_field(&contents, "approver")
            .with_context(|| format!("{} names no approver", path.display()))?;
        approvals.push(Approval { specimen, approver });
    }
    approvals.sort_by(|left, right| left.specimen.cmp(&right.specimen));
    Ok(approvals)
}

/// One `key = value` line of an `APPROVAL` file, comments excluded.
fn approval_field(contents: &str, key: &str) -> Option<String> {
    contents
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .find_map(|line| {
            line.trim()
                .strip_prefix(key)?
                .strip_prefix(" = ")
                .map(|value| value.trim().to_owned())
        })
}

/// Sums the committed command surface: in-scope controls, per application.
fn read_command_census(root: &Path) -> Result<CommandCensus> {
    let path = root.join("docs/client-platform/data/command-surface.tsv");
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut per_application: BTreeMap<String, u64> = BTreeMap::new();
    for (index, line) in contents.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 6 {
            bail!(
                "{}: line {} has {} fields, not the six its header declares",
                path.display(),
                index + 1,
                fields.len()
            );
        }
        if fields[5].trim() != "1" {
            continue;
        }
        let controls: u64 = fields[4].trim().parse().with_context(|| {
            format!(
                "{}: line {} has a non-numeric control count",
                path.display(),
                index + 1
            )
        })?;
        *per_application.entry(fields[0].to_owned()).or_default() += controls;
    }
    if per_application.is_empty() {
        bail!(
            "{} declares no in-scope controls, so the report's independent denominator would be \
             zero",
            path.display()
        );
    }
    Ok(CommandCensus {
        per_application: per_application.into_iter().collect(),
    })
}

/// The declared-element total from the committed schema census.
fn read_schema_census(root: &Path) -> Result<u64> {
    let path = root.join("docs/client-platform/data/schema-census.txt");
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    for line in contents.lines() {
        if let Some(rest) = line.trim().strip_prefix("TOTAL declared elements") {
            return rest
                .trim()
                .parse()
                .with_context(|| format!("{}: the TOTAL line is not a number", path.display()));
        }
    }
    bail!(
        "{} has no `TOTAL declared elements` line; regenerate it with \
         `bash docs/client-platform/data/schema_element_census.sh References`",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_about_assertions_is_not_an_assertion() {
        let suite = read_suite(
            "mjx-layout-docx",
            "// assert!(this is prose)\nassert_eq!(a, b);\n",
        );
        assert_eq!(suite.assertions, 1);
    }

    #[test]
    fn a_debug_assert_is_not_an_assertion() {
        assert_eq!(count_assertions("debug_assert!(x);"), 0);
        assert_eq!(count_assertions("assert!(x); assert_ne!(a, b);"), 2);
    }

    #[test]
    fn a_limitation_is_read_out_of_the_doc_comment_that_declares_it() {
        let suite = read_suite(
            "mjx-scene-pptx",
            "//! MJX-LEDGER-LIMITATION: a resolved alpha is thrown away.\n#[test]\nfn t() { assert!(x); }\n",
        );
        assert_eq!(suite.limitations, vec!["a resolved alpha is thrown away"]);
        assert_eq!(suite.tests, 1);
        assert!(suite.checks_something());
    }

    /// The sentence runs to the end of its paragraph, because a one-line marker would be either
    /// unreadably long or useless.
    #[test]
    fn a_limitation_continues_across_the_lines_of_its_paragraph() {
        let suite = read_suite(
            "mjx-layout-docx",
            "//! MJX-LEDGER-LIMITATION: a stretchy delimiter is scaled\n\
             //! rather than assembled, so its stroke grows with its height.\n\
             //!\n\
             //! A separate paragraph that is not part of it.\n\
             #[test]\nfn t() { assert!(x); }\n",
        );
        assert_eq!(
            suite.limitations,
            vec![
                "a stretchy delimiter is scaled rather than assembled, so its stroke grows with \
                 its height"
            ]
        );
    }

    /// A marker in a file that ends immediately after it is still read, rather than dropped with
    /// the loop.
    #[test]
    fn a_limitation_at_the_end_of_a_file_is_not_lost() {
        let suite = read_suite(
            "mjx-scene-xlsx",
            "//! MJX-LEDGER-LIMITATION: a dash draws solid\n",
        );
        assert_eq!(suite.limitations, vec!["a dash draws solid"]);
    }

    #[test]
    fn a_ledger_row_may_name_one_test_inside_a_suite() {
        let source = "enum Provenance {}\n\
                      suite: \"a_test (the_suite)\",\n\
                      provenance: Provenance::SpecCode,\n";
        let rows = read_provenance_ledger("mjx-layout-docx", source).expect("canonical");
        assert_eq!(
            rows,
            vec![(
                "crates/mjx-layout-docx/tests/the_suite.rs".to_owned(),
                Provenance::SpecCode
            )]
        );
    }

    #[test]
    fn a_ledger_whose_rows_do_not_pair_up_is_not_read() {
        let source = "enum Provenance {}\nsuite: \"one\",\nsuite: \"two\",\n";
        assert!(read_provenance_ledger("mjx-layout-docx", source).is_none());
    }

    #[test]
    fn an_aliased_provenance_ledger_is_not_read_rather_than_miscounted() {
        // `the_format_language_is_evaluated.rs` writes `Spec` / `Documented` / `Derived` through a
        // `use` alias and passes them as arguments. Reading it with this scanner would produce zero
        // rows and look like a suite that declares no provenance, which is a lie. It is reported as
        // unread instead, and the generated document names it.
        let source = "enum Provenance {}\nuse Provenance::{SpecCode as Spec};\n    Spec,\n";
        assert!(read_provenance_ledger("mjx-layout-xlsx", source).is_none());
    }

    #[test]
    fn an_approval_field_ignores_the_comment_header() {
        let contents = "# approver = human, in the explanatory header\napprover = generator\n";
        assert_eq!(
            approval_field(contents, "approver").as_deref(),
            Some("generator")
        );
    }
}
