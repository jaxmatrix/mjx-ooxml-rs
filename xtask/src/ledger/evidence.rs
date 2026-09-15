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
//! It reads Rust source through [`super::scan`]: comments removed, string literals numbered and
//! whitespace collapsed, over the suite and every helper module it pulls in. That is the weakest
//! part of this pipeline and it is written to fail loudly rather than quietly: a helper that cannot
//! be found fails the scan, a provenance ledger whose rows do not pair up is **not read** rather
//! than read wrongly, and a file
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

use super::scan;

/// The marker a suite writes in its module documentation to declare a limitation it asserts.
///
/// It goes **in the suite**, not in a source module and not in a table here, because the project's
/// first ledger rule is that a state is produced by a test. A limitation nothing asserts is a claim;
/// a limitation a suite asserts is a fact, and the marker is where the fact says so.
pub(crate) const LIMITATION_MARKER: &str = "MJX-LEDGER-LIMITATION:";

/// Crates whose suites work at or past the display list: the companions, the painter and the oracle.
///
/// **A crate never makes evidence drawn.** Only a cited test function that reads a display list or
/// pixels does, wherever it lives; anything else from these crates — a resolved colour, a paint table,
/// a whole suite — stops short of drawn and caps a drawn row at `partial`.
///
/// Every `mjx-scene-*` companion must be here, and [`scan`] refuses a workspace where one is not.
/// `mjx-reference-pack` is not: only its journeys draw a document, and they are named one by one in
/// [`RENDERING_JOURNEYS`]. `mjx-canvas-harness` is not either: it draws synthetic fragment trees and
/// no document ever reaches it, so nothing it asserts is about a capability in this ledger.
///
/// A crate in neither this list nor [`LAYOUT_TIER`] is model tier, which can only ever *lower* a
/// state — so an unclassified newcomer is reported conservatively rather than optimistically.
pub(crate) const RENDERING_TIER: &[&str] = &[
    "mjx-scene",
    "mjx-scene-pptx",
    "mjx-scene-xlsx",
    "mjx-paint",
    "mjx-render-oracle",
];

/// The `mjx-reference-pack` suites that carry a committed document all the way to pixels; a function of theirs that draws nothing stops short of drawn.
pub(crate) const RENDERING_JOURNEYS: &[&str] = &[
    "crates/mjx-reference-pack/tests/a_real_deck_reaches_pixels.rs",
    "crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs",
];

/// Crates whose suites stop at a **`FragmentTree`**, a measured run or an outline.
///
/// Evidence here proves a capability is laid out, and nothing about whether it is drawn: a drawn row
/// whose only evidence is this tier caps at `partial`, and the row says so.
/// Every `mjx-layout-*` box model must be here, and [`scan`] refuses a workspace where one is not.
pub(crate) const LAYOUT_TIER: &[&str] = &[
    "mjx-text",
    "mjx-layout",
    "mjx-layout-chart",
    "mjx-layout-docx",
    "mjx-layout-pptx",
    "mjx-layout-xlsx",
    "mjx-geometry",
    "mjx-view",
];

/// Which tier one citation's evidence is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tier {
    /// A cited test function reads an encoded display list or pixels.
    Rendering,
    /// It stops short of drawn: a fragment tree, an outline, a resolved paint.
    Layout,
    /// It reads or writes markup.
    Model,
}

/// A stand-in a suite, or a helper module it pulls in, supplies for something the product should supply itself.
///
/// A suite that draws with one of these proves the painter works **around** the stand-in, and
/// nothing about the feature the stand-in replaces. [`super::rows::STAND_INS`] says which rows each
/// one is not evidence for.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Double {
    /// `NoImages`, a test-local `no_images` function, or a test-local `impl … ImageSource for`: no
    /// picture the document holds is ever decoded.
    Images,
    /// A `.with_theme(…)` call in the test: the caller, not the product, supplied the theme.
    Theme,
    /// `PlaceholderGeometry`, a test-local `impl … GeometryProvider for`, or a closure handed to
    /// `register_all`: the outline came from the test, not from library code.
    Geometry,
}

impl Double {
    /// Every double, in the order the generated document lists them.
    pub(crate) const ALL: [Self; 3] = [Self::Images, Self::Theme, Self::Geometry];

    /// The spelling the scan looks for, as the generated document names it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Images => {
                "a test-supplied image source (`NoImages` or a test-local `ImageSource`)"
            }
            Self::Theme => "a test-supplied theme (`with_theme`)",
            Self::Geometry => "a test-local geometry provider",
        }
    }

    /// What the double stands in for, in the words a limitation quotes.
    pub(crate) fn stands_in_for(self) -> &'static str {
        match self {
            Self::Images => "decoded pictures",
            Self::Theme => "the document's own theme reaching the resolver",
            Self::Geometry => "library code carrying each shape's declared outline",
        }
    }

    /// Whether normalised code — a suite and its helpers — uses this double.
    fn used_by(self, code: &str) -> bool {
        match self {
            Self::Images => {
                scan::contains_identifier(code, "NoImages")
                    || code.contains("fn no_images(")
                    || scan::implements(code, "ImageSource")
            }
            Self::Theme => code.contains(".with_theme("),
            Self::Geometry => {
                scan::contains_identifier(code, "PlaceholderGeometry")
                    || scan::implements(code, "GeometryProvider")
                    || scan::registers_a_closure(code)
            }
        }
    }
}

/// A citation as the generated document names it: crate and file stem, then the function it names.
pub(crate) fn short_citation(citation: &str) -> String {
    let (path, function) = scan::split_citation(citation);
    match function {
        Some(function) => format!("{}::{function}", short_suite(path)),
        None => short_suite(path),
    }
}

/// A suite's workspace path as its crate and file stem, the way the generated document names a suite.
pub(crate) fn short_suite(path: &str) -> String {
    let trimmed = path.strip_prefix("crates/").unwrap_or(path);
    let trimmed = trimmed.strip_suffix(".rs").unwrap_or(trimmed);
    match trimmed.split_once("/tests/") {
        Some((crate_name, stem)) => format!("{crate_name}: {stem}"),
        None => trimmed.to_owned(),
    }
}

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
    /// The test doubles its code, or a helper module it pulls in, uses.
    pub(crate) doubles: BTreeSet<Double>,
    /// The normalised code of the suite and every helper module it pulls in.
    pub(crate) code: String,
    /// The `#[test]` functions the suite declares.
    pub(crate) test_functions: BTreeSet<String>,
    /// The test functions that read a display list or pixels, directly or through a local helper.
    pub(crate) drawing_tests: BTreeSet<String>,
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
    /// The `features.json` row ids naming each ledger row, by ledger row id.
    pub(crate) checklist: BTreeMap<String, Vec<String>>,
}

/// One citation, resolved: the suite, and the test function it names if it names one.
pub(crate) struct Cited<'a> {
    /// The suite's workspace path.
    pub(crate) path: &'a str,
    /// The suite.
    pub(crate) suite: &'a Suite,
    /// The test function it names, if it names one.
    pub(crate) function: Option<&'a str>,
    /// Tests the citation covers.
    pub(crate) tests: usize,
    /// Assertions the citation makes, through the local functions a cited function calls.
    pub(crate) assertions: usize,
}

impl Cited<'_> {
    /// Whether it names a test function that reads a display list or pixels.
    pub(crate) fn draws(&self) -> bool {
        self.function
            .is_some_and(|function| self.suite.drawing_tests.contains(function))
    }

    /// Its tier, decided by what it reads rather than by the crate it lives in.
    pub(crate) fn tier(&self) -> Tier {
        let crate_name = self.suite.crate_name.as_str();
        if self.draws() {
            Tier::Rendering
        } else if LAYOUT_TIER.contains(&crate_name)
            || RENDERING_TIER.contains(&crate_name)
            || RENDERING_JOURNEYS.contains(&self.path)
        {
            Tier::Layout
        } else {
            Tier::Model
        }
    }
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

    /// Resolves a citation — a suite path, or a suite path and `::` and a test function it declares.
    pub(crate) fn cite<'a>(&'a self, citation: &'a str) -> Result<Cited<'a>> {
        let (path, function) = scan::split_citation(citation);
        let suite = self.suite(path)?;
        let Some(function) = function else {
            return Ok(Cited {
                path,
                suite,
                function: None,
                tests: suite.tests,
                assertions: suite.assertions,
            });
        };
        if !suite.test_functions.contains(function) {
            bail!(
                "the parity ledger cites `{citation}`, and `{path}` declares no `#[test] fn \
                 {function}`. A row may not outlive its evidence: fix the citation in \
                 `xtask/src/ledger/rows.rs`."
            );
        }
        let assertions = scan::reach(&suite.code, function)
            .iter()
            .map(|body| scan::count_assertions(body))
            .sum();
        Ok(Cited {
            path,
            suite,
            function: Some(function),
            tests: 1,
            assertions,
        })
    }

    /// The `features.json` rows that name the ledger row `id`.
    pub(crate) fn checklist_rows_naming(&self, id: &str) -> &[String] {
        self.checklist.get(id).map_or(&[], Vec::as_slice)
    }

    /// An index of synthetic suites, and of checklist rows as `(ledger id, checklist id)`.
    #[cfg(test)]
    pub(crate) fn of(entries: &[(&str, Suite)], named: &[(&str, &str)]) -> Self {
        let mut checklist: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (ledger, row) in named {
            checklist
                .entry((*ledger).to_owned())
                .or_default()
                .push((*row).to_owned());
        }
        Self {
            suites: entries
                .iter()
                .map(|(path, suite)| ((*path).to_owned(), suite.clone()))
                .collect(),
            provenance_not_read: Vec::new(),
            approvals: Vec::new(),
            commands: CommandCensus {
                per_application: vec![("Word".to_owned(), 1)],
            },
            declared_elements: 1,
            crates: BTreeSet::new(),
            checklist,
        }
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

    check_the_tiers(&crates)?;

    let sources = read_suite_sources(root, &crates)?;
    let mut suites: BTreeMap<String, Suite> = sources
        .iter()
        .map(|(path, (crate_name, unit))| (path.clone(), read_suite_unit(crate_name, unit)))
        .collect();
    let provenance_not_read = attach_provenance(&sources, &mut suites)?;
    check_the_journeys(&suites)?;

    Ok(Evidence {
        suites,
        provenance_not_read,
        approvals: scan_approvals(root)?,
        commands: read_command_census(root)?,
        declared_elements: read_schema_census(root)?,
        crates,
        checklist: read_checklist(root)?,
    })
}

/// The two tier lists name real crates, share none, and between them classify every companion and
/// every box model.
fn check_the_tiers(crates: &BTreeSet<String>) -> Result<()> {
    for (list, names) in [
        ("RENDERING_TIER", RENDERING_TIER),
        ("LAYOUT_TIER", LAYOUT_TIER),
    ] {
        for name in names {
            if !crates.contains(*name) {
                bail!(
                    "`{list}` names `{name}`, which is not a crate in this workspace. The tiers \
                     decide whether a capability is `implemented`, `partial` or \
                     `preserved-not-rendered`, so a stale name here misstates every row it touches."
                );
            }
        }
    }
    if let Some(shared) = RENDERING_TIER
        .iter()
        .find(|name| LAYOUT_TIER.contains(name))
    {
        bail!("`{shared}` is in both `RENDERING_TIER` and `LAYOUT_TIER`; a suite has one tier");
    }
    for name in crates {
        let rendering = RENDERING_TIER.contains(&name.as_str());
        let layout = LAYOUT_TIER.contains(&name.as_str());
        if name.starts_with("mjx-scene-") && !rendering {
            bail!("`{name}` is a scene companion and is not in `RENDERING_TIER`");
        }
        if name.starts_with("mjx-layout-") && !layout {
            bail!("`{name}` is a box model and is not in `LAYOUT_TIER`");
        }
    }
    Ok(())
}

/// Every named journey is a suite.
fn check_the_journeys(suites: &BTreeMap<String, Suite>) -> Result<()> {
    for journey in RENDERING_JOURNEYS {
        if !suites.contains_key(*journey) {
            bail!("`RENDERING_JOURNEYS` names `{journey}`, which is not a suite in this workspace");
        }
    }
    Ok(())
}

/// The `features.json` row ids naming each ledger row.
fn read_checklist(root: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let path = root.join("docs/client-platform/data/features.json");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let document = super::json::parse(&text)
        .map_err(|error| anyhow::anyhow!("{}: {error}", path.display()))?;
    let rows = document
        .get("features")
        .and_then(super::json::Value::array)
        .with_context(|| format!("{} has no `features` array", path.display()))?;
    let mut checklist: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let field = |key: &str| row.get(key).and_then(super::json::Value::string);
        if let (Some(id), Some(ledger)) = (field("id"), field("ledger")) {
            checklist
                .entry(ledger.to_owned())
                .or_default()
                .push(id.to_owned());
        }
    }
    if checklist.is_empty() {
        bail!(
            "{} names no ledger row, so no row would be asked whether it is drawn on the \
             checklist's account",
            path.display()
        );
    }
    Ok(checklist)
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
/// credit a fixture builder with checking something. They are still read, as part of the suite
/// that pulls them in, because a double a helper supplies is a double the suite draws through.
///
/// Returns `path -> (crate name, unit)`, sorted, so the whole tree is read once.
type SuiteSources = BTreeMap<String, (String, scan::Unit)>;

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
            let relative = format!("crates/{name}/tests/{stem}.rs");
            let unit = scan::read_unit(root, &relative).map_err(anyhow::Error::msg)?;
            sources.insert(relative, (name.clone(), unit));
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
#[cfg(test)]
pub(crate) fn read_suite(crate_name: &str, source: &str) -> Suite {
    read_unit(crate_name, source, &[])
}

/// One suite read from its source and its helpers' sources, as the tree scan would read it.
#[cfg(test)]
fn read_unit(crate_name: &str, source: &str, helpers: &[String]) -> Suite {
    let code = scan::normalise(source).code;
    let unit = scan::unit_code(&code, helpers);
    read_suite_unit(
        crate_name,
        &scan::Unit {
            source: source.to_owned(),
            code,
            unit,
        },
    )
}

/// Counts what one suite declares, reading its doubles out of the suite and its helpers together.
fn read_suite_unit(crate_name: &str, unit: &scan::Unit) -> Suite {
    let source = unit.source.as_str();
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
    }
    if let Some(complete) = pending.take() {
        limitations.push(complete);
    }

    for limitation in &mut limitations {
        while limitation.ends_with('.') {
            limitation.pop();
        }
    }

    let tests = unit.code.matches("#[test]").count();
    let assertions = scan::count_assertions(&unit.code);
    let doubles = Double::ALL
        .into_iter()
        .filter(|double| double.used_by(&unit.unit))
        .collect();
    let test_functions = scan::test_functions(&unit.code)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let drawing_tests = scan::drawing_tests(&unit.code, &unit.unit);

    Suite {
        crate_name: crate_name.to_owned(),
        tests,
        assertions,
        limitations,
        split: Split::default(),
        doubles,
        code: unit.unit.clone(),
        test_functions,
        drawing_tests,
    }
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

    for (path, (crate_name, unit)) in sources {
        let source = &unit.source;
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
        let code = scan::strip_comment(line.trim());
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
    fn a_suite_path_is_shortened_to_its_crate_and_stem() {
        assert_eq!(
            short_suite("crates/mjx-layout-docx/tests/a_thing.rs"),
            "mjx-layout-docx: a_thing"
        );
    }

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
        assert_eq!(scan::count_assertions("debug_assert!(x);"), 0);
        assert_eq!(scan::count_assertions("assert!(x); assert_ne!(a, b);"), 2);
    }

    #[test]
    fn a_limitation_is_read_out_of_the_doc_comment_that_declares_it() {
        let suite = read_suite(
            "mjx-scene-pptx",
            "//! MJX-LEDGER-LIMITATION: a resolved alpha is thrown away.\n#[test]\nfn t() { assert!(x); }\n",
        );
        assert_eq!(suite.limitations, vec!["a resolved alpha is thrown away"]);
        assert_eq!(suite.tests, 1);
        assert!(suite.assertions > 0);
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

    // A journey function that draws with `NoImages` has never decoded a picture, so it cannot promote a picture row.
    #[test]
    fn a_rendering_suite_that_draws_with_no_images_is_not_evidence_for_a_picture_row() {
        use super::super::assess::{assess, State};
        const PATH: &str = "crates/mjx-reference-pack/tests/a_real_deck_reaches_pixels.rs";
        let source = "use mjx_paint::NoImages;\n#[test]\nfn t() { let images = NoImages; assert!(draw(&images).pixel(0, 0).is_some()); }\n";
        let evidence = index_of(&[(PATH, "mjx-reference-pack", source)]);
        let assessed = assess(
            row_citing(
                "picture-insertion",
                "crates/mjx-reference-pack/tests/a_real_deck_reaches_pixels.rs::t",
            ),
            &evidence,
        )
        .expect("the suite exists");
        assert_eq!(assessed.state, State::Partial);
        assert!(
            assessed.caps.iter().any(|cap| cap.contains("NoImages")),
            "the cap names the double: {:?}",
            assessed.caps
        );
    }

    // A citation's tier is what it reads: a drawing function is rendering wherever it lives, and nothing else is.
    #[test]
    fn a_citations_tier_is_what_it_reads_and_not_its_crate() {
        let drawing = "use mjx_paint::Pixels;\n#[test]\nfn draws() { assert!(pixels.pixel(0, 0).is_some()); }\n#[test]\nfn resolves() { assert!(colour.is_some()); }\n";
        let evidence = index_of(&[
            (
                "crates/mjx-layout-pptx/tests/a.rs",
                "mjx-layout-pptx",
                drawing,
            ),
            (
                "crates/mjx-scene-pptx/tests/b.rs",
                "mjx-scene-pptx",
                drawing,
            ),
            (
                "crates/mjx-reference-pack/tests/the_instructions_are_complete.rs",
                "mjx-reference-pack",
                drawing,
            ),
            (RENDERING_JOURNEYS[0], "mjx-reference-pack", drawing),
        ]);
        let tier = |citation: &'static str| evidence.cite(citation).expect("cited").tier();
        assert!(RENDERING_TIER
            .iter()
            .all(|name| !LAYOUT_TIER.contains(name)));
        assert_eq!(tier("crates/mjx-layout-pptx/tests/a.rs"), Tier::Layout);
        assert_eq!(tier("crates/mjx-scene-pptx/tests/b.rs"), Tier::Layout);
        assert_eq!(
            tier("crates/mjx-scene-pptx/tests/b.rs::resolves"),
            Tier::Layout
        );
        assert_eq!(
            tier("crates/mjx-scene-pptx/tests/b.rs::draws"),
            Tier::Rendering
        );
        assert_eq!(
            tier("crates/mjx-reference-pack/tests/the_instructions_are_complete.rs::resolves"),
            Tier::Model
        );
        assert_eq!(
            tier("crates/mjx-reference-pack/tests/a_real_deck_reaches_pixels.rs::resolves"),
            Tier::Layout
        );
        assert_eq!(
            tier("crates/mjx-reference-pack/tests/the_instructions_are_complete.rs::draws"),
            Tier::Rendering
        );
    }

    // Drawing is reading a list or pixels, through a local helper too; producing one, or naming one in prose, is not.
    #[test]
    fn a_function_draws_when_it_or_a_local_helper_reads_what_was_drawn() {
        let code = scan::normalise(
            "use mjx_paint::Pixels;\nuse mjx_scene::{Command, DisplayList};\n\
             fn ink(pixels: &Pixels) -> usize { pixels.pixel(0, 0).map_or(0, |_| 1) }\n\
             #[test] fn constructs_a_command() { let mut list = builder(); list.push(Command::FillPath { geometry, paint }); assert!(list.len() > 0); }\n\
             #[test] fn through_a_helper() { assert_eq!(ink(&render()), 1); }\n\
             #[test] fn only_produces() { let list = build_scene(&tree); assert!(list.byte_len() > 0); }\n\
             #[test] fn in_prose() { assert!(true, \"list.commands() and .pixel(\"); }\n\
             #[test] fn through_a_method() { assert!(report.ink() > 0); }\n",
        )
        .code;
        assert_eq!(
            scan::drawing_tests(&code, &code),
            ["through_a_helper".to_owned()].into_iter().collect()
        );
        let outline = scan::normalise(
            "use mjx_text::Outline;\n#[test] fn reads_an_outline() { assert!(outline.commands().first().is_some()); }\n",
        )
        .code;
        assert!(
            scan::drawing_tests(&outline, &outline).is_empty(),
            "an outline's `commands()` is not a display list's"
        );
    }

    // Each double is read from code, never from prose, and a longer identifier is not the double.
    #[test]
    fn a_double_is_read_from_code_and_not_from_prose() {
        let suite = read_suite(
            "mjx-paint",
            "//! NoImages and PlaceholderGeometry in prose\n\
             use mjx_scene::PlaceholderGeometry;\n\
             let palette = palette.with_theme(theme);\n\
             let images = NoImagesAtAll;\n",
        );
        assert_eq!(
            suite.doubles,
            [Double::Theme, Double::Geometry].into_iter().collect()
        );
        let closure = read_suite(
            "mjx-reference-pack",
            "geometry.register_all(model.catalogue(), |request| {\nfn no_images(_id: &str) -> Option<u64> { None }\n",
        );
        assert_eq!(
            closure.doubles,
            [Double::Images, Double::Geometry].into_iter().collect()
        );
    }

    // An index of synthetic suites, each read by the scanner from its source.
    fn index_of(entries: &[(&str, &str, &str)]) -> Evidence {
        let suites: Vec<(&str, Suite)> = entries
            .iter()
            .map(|(path, crate_name, source)| (*path, read_suite(crate_name, source)))
            .collect();
        Evidence::of(&suites, &[])
    }

    // A rendered row with one citation, leaked so it is `'static` like the committed table.
    fn row_citing(
        id: &'static str,
        citation: &'static str,
    ) -> &'static super::super::rows::Capability {
        use super::super::rows::{Capability, Kind, Section};
        Box::leak(Box::new(Capability {
            id,
            section: Section::SharedDrawing,
            capability: "a capability",
            kind: Kind::Rendered,
            excluded_because: None,
            evidence: Box::leak(vec![citation].into_boxed_slice()),
        }))
    }

    // A function in a painter's suite that checks a table and draws nothing is not rendering evidence, however it is cited.
    #[test]
    fn a_rendering_crate_function_that_draws_nothing_does_not_implement_a_rendered_row() {
        use super::super::assess::{assess, State};
        const PATH: &str = "crates/mjx-paint/tests/the_tables_are_tables.rs";
        let source = "#[test]\nfn the_masks_are_distinct() { let masks = PATTERN_MASKS; assert_eq!(masks.len(), 54); }\n";
        let evidence = index_of(&[(PATH, "mjx-paint", source)]);
        let mut assessed_once = false;
        for citation in [
            PATH,
            "crates/mjx-paint/tests/the_tables_are_tables.rs::the_masks_are_distinct",
        ] {
            if let Ok(assessed) = assess(row_citing("outlines", citation), &evidence) {
                assessed_once = true;
                assert_ne!(
                    assessed.state,
                    State::Implemented,
                    "`{citation}` promoted a row though nothing in it draws"
                );
            }
        }
        assert!(assessed_once, "neither citation form was assessed");
    }

    // A cited function whose body reads the encoded display list's commands is rendering evidence.
    #[test]
    fn a_cited_function_that_reads_the_display_list_implements_a_rendered_row() {
        use super::super::assess::{assess, State};
        const PATH: &str = "crates/mjx-scene-pptx/tests/a_stroke_is_drawn.rs";
        let source = "use mjx_scene::Command;\n#[test]\nfn the_stroke_is_a_command() { let list = build(); assert!(list.commands().any(|command| matches!(command, Command::StrokePath { .. }))); }\n";
        let evidence = index_of(&[(PATH, "mjx-scene-pptx", source)]);
        let assessed = assess(
            row_citing(
                "outlines",
                "crates/mjx-scene-pptx/tests/a_stroke_is_drawn.rs::the_stroke_is_a_command",
            ),
            &evidence,
        )
        .expect("a function citation is assessed");
        assert_eq!(assessed.state, State::Implemented);
    }

    // An allowance names one row: excusing `NoImages` for the worksheet journey can never promote `excel-pictures`.
    #[test]
    fn an_allowance_for_one_row_does_not_excuse_the_double_for_another() {
        use super::super::assess::{assess, State};
        const PATH: &str = "crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs";
        let source = "use mjx_paint::NoImages;\n#[test]\nfn the_ink_lands() { let images = NoImages; let pixels = draw(&images); assert!(pixels.pixel(1, 1).is_some()); }\n";
        let evidence = index_of(&[(PATH, "mjx-reference-pack", source)]);
        let mut assessed_once = false;
        for citation in [
            PATH,
            "crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs::the_ink_lands",
        ] {
            if let Ok(assessed) = assess(row_citing("excel-pictures", citation), &evidence) {
                assessed_once = true;
                assert_ne!(
                    assessed.state,
                    State::Implemented,
                    "`{citation}` promoted `excel-pictures` through an allowance written for another row"
                );
                assert!(
                    assessed.caps.iter().any(|cap| cap.contains("NoImages")),
                    "the cap names the double: {:?}",
                    assessed.caps
                );
            }
        }
        assert!(assessed_once, "neither citation form was assessed");
        let excused = assess(
            row_citing(
                "excel-reaches-pixels",
                "crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs::the_ink_lands",
            ),
            &evidence,
        )
        .expect("assessed");
        assert_eq!(
            excused.state,
            State::Implemented,
            "the row the allowance names is excused"
        );
    }

    // A double written across several lines is still a double.
    #[test]
    fn a_double_written_across_lines_is_found() {
        let suite = read_suite(
            "mjx-paint",
            "impl\n    GeometryProvider\n    for Stand {\n}\n",
        );
        assert!(
            suite.doubles.contains(&Double::Geometry),
            "{:?}",
            suite.doubles
        );
    }

    // A path-qualified impl is the same double as a bare one.
    #[test]
    fn a_path_qualified_impl_is_a_double() {
        let suite = read_suite(
            "mjx-scene",
            "impl mjx_scene::GeometryProvider for Stand {\n}\n",
        );
        assert!(
            suite.doubles.contains(&Double::Geometry),
            "{:?}",
            suite.doubles
        );
    }

    // A `//` inside a string literal is text, so the code after it on the line is still read.
    #[test]
    fn a_comment_marker_inside_a_string_is_not_a_comment() {
        let suite = read_suite(
            "mjx-paint",
            "let url = \"http://example.org\"; assert!(url.is_empty());\n",
        );
        assert_eq!(suite.assertions, 1);
        assert_eq!(
            scan::strip_comment("let url = \"http://x\"; // prose"),
            "let url = \"http://x\"; "
        );
    }

    // Raw strings and character literals hide their contents, so neither can fake a double or an assertion.
    #[test]
    fn a_raw_string_and_a_char_literal_hide_their_contents() {
        let suite = read_suite(
            "mjx-paint",
            "let a = r#\"impl GeometryProvider for X \" assert!(y)\"#;\nlet b = '\"';\nlet c = \"NoImages\";\nassert!(a.is_empty());\n",
        );
        assert!(suite.doubles.is_empty(), "{:?}", suite.doubles);
        assert_eq!(suite.assertions, 1);
    }

    // A test-local picture source is the image double, whatever the type is called.
    #[test]
    fn a_test_local_image_source_is_a_double() {
        let suite = read_suite(
            "mjx-paint",
            "impl mjx_paint::ImageSource for OnePicture {\n}\n",
        );
        assert!(
            suite.doubles.contains(&Double::Images),
            "{:?}",
            suite.doubles
        );
    }

    // A double a suite reaches through a helper module it pulls in is that suite's double.
    #[test]
    fn a_double_in_a_helper_module_is_the_suites_double() {
        let evidence = scan(&crate::codegen::workspace_root()).expect("the workspace scans");
        let painters = &evidence.suites["crates/mjx-paint/tests/two_painters_agree.rs"];
        assert!(
            painters.doubles.contains(&Double::Images),
            "`two_painters_agree` draws through `common::OnePicture`: {:?}",
            painters.doubles
        );
        let formats =
            &evidence.suites["crates/mjx-scene-xlsx/tests/a_conditional_format_changes_a_pixel.rs"];
        assert!(
            formats.doubles.contains(&Double::Theme),
            "`a_conditional_format_changes_a_pixel` resolves through `support::resolve`'s `with_theme`: {:?}",
            formats.doubles
        );
    }

    // Helpers are found through `mod name;`, a nested `mod`, and `#[path]`, and one that is missing fails the scan.
    #[test]
    fn a_helper_is_found_through_mod_and_path_and_a_missing_one_fails() {
        let root = std::env::temp_dir().join(format!(
            "mjx-ledger-helpers-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("a clock after 1970")
                .as_nanos()
        ));
        let write = |relative: &str, text: &str| {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, text).expect("a fixture file");
        };
        write(
            "crates/a/tests/suite.rs",
            "mod common;\n#[path = \"../../b/tests/support/mod.rs\"]\nmod shared;\n",
        );
        write("crates/a/tests/common/mod.rs", "pub mod inner;\n");
        write(
            "crates/a/tests/common/inner.rs",
            "impl x::ImageSource for P {}\n",
        );
        write("crates/b/tests/support/mod.rs", "fn helper() {}\n");
        write("crates/a/tests/broken.rs", "mod absent;\n");

        let found =
            scan::module_files(&root, "crates/a/tests/suite.rs").expect("the helpers resolve");
        let paths: Vec<&str> = found.iter().map(|(path, _)| path.as_str()).collect();
        let missing = scan::module_files(&root, "crates/a/tests/broken.rs");
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(
            paths,
            [
                "crates/a/tests/common/inner.rs",
                "crates/a/tests/common/mod.rs",
                "crates/b/tests/support/mod.rs"
            ]
        );
        let helpers: Vec<String> = found.into_iter().map(|(_, source)| source).collect();
        let suite = read_unit("a", "mod common;\n", &helpers);
        assert!(suite.doubles.contains(&Double::Images));
        assert!(missing.is_err(), "a `mod` with no file is an error");
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
