//! **The preservation gate (MJXOFF-210): every fixture × every mutating API — assert what changed,
//! and that nothing else did.**
//!
//! # The rule this enforces
//!
//! *An edit changes what the caller asked to change, and nothing else.* Sharpened from the standing
//! design rule of Phase G: **supply a default only in the absence of the user's own, never in place
//! of it.** A file opened from disk keeps what it came with.
//!
//! # Why it is a sweep and not four hand-written tests
//!
//! Four tests in this workspace already state the property for one method on one fixture, and the
//! best of them — `mjx-xlsx/tests/charts.rs`'s `authoring_a_chart_adds_exactly_its_own_parts_…` — is
//! the model this file generalizes. But every one of them is per-feature and hand-written, so **a
//! method added later is outside all of them by default.** That is not a hypothetical: it is how
//! both destructive defects of Phase G got in.
//!
//! * **MJXOFF-208** — a chart data edit rebuilt the producer's embedded workbook from scratch,
//!   discarding its extra sheets, its formats and its document properties. The part legitimately
//!   changed; what was destroyed was *inside* it, which is why [`diff`] walks into an embedded
//!   package instead of comparing it as opaque bytes.
//! * **MJXOFF-209** — a relationship target that was percent-encoded resolved to nothing, so an
//!   edit about a header swept an unrelated image out of the package as an orphan. The part
//!   *vanished*, which is why a removal is a first-class difference here and not something only a
//!   both-directions name comparison could notice.
//! * **MJXOFF-200** — a theme was authored where the package needed one. Written the naive way it
//!   would have overwritten the theme of every real document this library opens, turning an
//!   invisible-chart bug into a corrupt-the-customer's-file bug. A theme part changing under a
//!   method that never declared it is what fails here.
//!
//! Each of those three was found by a person. Each is now caught by a machine — proved by
//! re-introducing all three defects and watching this suite go red (see the PR for MJXOFF-210).
//!
//! # How it differs from `mjx_schema_gate::references`, which G4 landed
//!
//! They are adjacent and neither supersedes the other. `references` asks a question about **a
//! package we authored**: does every reference its own content makes — a relationship id, a scheme
//! colour, a style id — resolve to something the package contains? It is about *completeness*, and
//! it is what catches an absent part that should have existed. This file asks a question about **a
//! file the user already had**: given a package we did not write, what did one call do to it? It is
//! about *restraint*, and it is what catches a present part that should have been left alone.
//!
//! The two fail on opposite mistakes. Authoring a theme into a package that has none makes
//! `references` green and is invisible here — nothing was taken away. Overwriting a theme the
//! package already carried makes `references` green too, because the reference still resolves, and
//! reddens this file at once. Neither gate can see the other's defect, which is why both exist.
//!
//! # What a declaration says
//!
//! Every method is registered with a [`Touches`](rules::Touches): the classes of part it may add,
//! change or remove, and how many of each. A difference matching no clause fails; a clause counted
//! [`Exactly`](rules::Count::Exactly) that matched nothing fails too. A reader declares
//! [`NOTHING`](rules::NOTHING), which is the strongest assertion in the file — the package must come
//! back part for part identical, so a reader that dirtied the part it read is caught.
//!
//! # The two vacuity traps, and what is under each
//!
//! **A suite this size can be green because it exercises almost nothing.** So:
//!
//! * The method list is **derived from the facade's source** ([`enumeration`]) and compared against
//!   the registry **in both directions**. A method added to the facade and not registered here
//!   fails; a registered method the facade no longer has fails too.
//! * The corpus is **read from `mjx-fixtures`** and compared against the fixtures the sweep visited
//!   **in both directions**. A fixture added later joins by being there, and a fixture that stopped
//!   being visited cannot hide.
//! * The set of package extensions is compared against `mjx_fixtures::PACKAGE_EXTENSIONS` **in both
//!   directions**, so a fourth format cannot be added to the corpus and silently reach no surface.
//! * A method that never once did what it declares — never applied, or, for a reader, never even
//!   succeeded — is listed in [`NEVER_EXERCISED`], and that register is compared against what the
//!   sweep saw **in both directions**. An entry that starts working fails just as loudly as a method
//!   that stops.
//!
//! A count over the whole sweep is reported (`cargo test … -- --nocapture`) and floored, but the
//! count is not what the suite rests on: a floor over a total says the driver is alive, not that it
//! is complete. The four both-directions comparisons above are what say that.
//!
//! # What it costs
//!
//! One open, one call and one `save()` per pair, over a corpus whose largest fixture is 35 KB.
//! Fixtures are swept on one thread each. The whole sweep runs once per test binary and is shared
//! through a [`OnceLock`], so the assertions below that read it share that one sweep between
//! them rather than each paying for their own.

mod deck_cases;
mod diff;
mod document_cases;
mod enumeration;
mod rules;
mod workbook_cases;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use mjx_ooxml::Error;

use crate::rules::Touches;

/// Which facade surface a method belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Api {
    /// `mjx_ooxml::Deck` — `.pptx`.
    Deck,
    /// `mjx_ooxml::Document` — `.docx`.
    Document,
    /// `mjx_ooxml::Workbook` — `.xlsx`.
    Workbook,
}

impl Api {
    /// The file extension whose fixtures this surface is swept over.
    pub(crate) const fn extension(self) -> &'static str {
        match self {
            Self::Deck => "pptx",
            Self::Document => "docx",
            Self::Workbook => "xlsx",
        }
    }

    /// The type name, for a failure message.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Deck => "Deck",
            Self::Document => "Document",
            Self::Workbook => "Workbook",
        }
    }
}

impl std::fmt::Display for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// What one case did when it was offered a fixture.
#[derive(Debug)]
pub(crate) enum Step {
    /// The fixture offers no address this call could take — no chart to edit, no OLE object to
    /// read. Tallied as *no address*: it is not a pass and not a failure, and a method that every
    /// fixture skips is caught by [`NEVER_EXERCISED`].
    Skipped,
    /// The call was made. Whatever it answered, the package is then saved and compared.
    Ran(Result<(), Error>),
}

/// The convenience every case body ends with: run the call, discard its value, keep its result.
macro_rules! ran {
    ($call:expr) => {
        crate::Step::Ran($call.map(|_| ()))
    };
}
pub(crate) use ran;

/// A method's tally across the fixtures of its format.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Tally {
    /// Pairs where the call succeeded and changed something.
    pub(crate) applied: usize,
    /// Pairs where the call succeeded and changed nothing.
    pub(crate) no_op: usize,
    /// Pairs where the call returned an error.
    pub(crate) refused: usize,
    /// Pairs where no address was available.
    pub(crate) not_applicable: usize,
}

impl Tally {
    /// Whether the call ever succeeded, either way.
    const fn ever_succeeded(&self) -> bool {
        self.applied > 0 || self.no_op > 0
    }
}

/// Whether this pair is a registered defect, so its disagreements are collected rather than failing.
fn is_known_defect(fixture: &str, api: Api, method: &str) -> bool {
    KNOWN_DEFECTS
        .iter()
        .any(|(f, a, m, _)| *f == fixture && *a == api && *m == method)
}

/// Everything the sweep saw.
#[derive(Debug, Default)]
pub(crate) struct Report {
    /// Every (fixture, method) pair attempted.
    pub(crate) pairs: usize,
    /// Every fixture visited, with the number of pairs run against it.
    pub(crate) fixtures: BTreeMap<String, usize>,
    /// Every method, with its tally.
    pub(crate) cases: BTreeMap<(Api, String), Tally>,
    /// Every disagreement, fixture and method named.
    pub(crate) violations: Vec<String>,
    /// The disagreements of the pairs [`KNOWN_DEFECTS`] registers, as `(fixture, api, method)`.
    pub(crate) known: BTreeSet<(String, Api, String)>,
}

impl Report {
    /// The pairs that ended in each outcome.
    pub(crate) fn totals(&self) -> Tally {
        self.cases.values().fold(Tally::default(), |mut acc, t| {
            acc.applied += t.applied;
            acc.no_op += t.no_op;
            acc.refused += t.refused;
            acc.not_applicable += t.not_applicable;
            acc
        })
    }
}

// =================================================================================================
// The register of methods no fixture exercises
// =================================================================================================

/// The methods the committed corpus cannot make do what they declare, each with the reason.
///
/// **This register is compared against the sweep in both directions** by
/// [`every_method_either_works_somewhere_or_says_why_not`]. An entry that starts being exercised
/// fails exactly as loudly as a method that stops being — which is what keeps a register from
/// becoming the quiet exemption list that the gates of MJXOFF-88 §7 all decayed into.
///
/// A method is here for one of two reasons, and both are properties of the **corpus**, never of the
/// method:
///
/// * it needs a shape of content no committed fixture holds, or
/// * it needs the package to be in a state a single call cannot reach from any fixture.
///
/// Each entry names the content that would retire it. Adding such a fixture is how the register
/// shrinks.
pub(crate) const NEVER_EXERCISED: &[(Api, &str, &str)] = &[
    (
        Api::Deck,
        "paragraph_field_text",
        "No `.pptx` fixture carries a text field (`a:fld`) — a slide number, a date — and the `Deck` \
         surface authors none, so no preparation can create one either. A fixture with a slide-number \
         placeholder retires this.",
    ),
    (
        Api::Deck,
        "paragraph_field_type",
        "As `paragraph_field_text`: the corpus has no `a:fld`.",
    ),
    (
        Api::Deck,
        "refresh_chart_workbook",
        "A refresh writes only where the embedded workbook disagrees with the chart's cached data. \
         `charts.pptx`'s workbook already agrees with its cache, and a chart this suite authors is \
         written with the two in step, so every refresh here is correctly a no-op. The writing path \
         is exercised by `crates/mjx-pptx/tests/charts.rs`. A fixture whose chart cache and workbook \
         differ retires this.",
    ),
    (
        Api::Deck,
        "remove_unused_parts",
        "No `.pptx` in the corpus carries a part nothing relates to, so the sweep correctly removes \
         nothing. That is a property of the corpus worth having; a fixture with a real orphan \
         retires this.",
    ),
    (
        Api::Deck,
        "replace_linked_image_with_placeholder",
        "Needs a picture whose image is *linked* (`a:blip@r:link`) rather than embedded. No fixture \
         carries one, and no method of the facade authors one, so no preparation can make one.",
    ),
    (
        Api::Deck,
        "replace_media_with_placeholder",
        "Needs an audio or video relationship on a surface. No fixture carries one, and no method of \
         the facade authors one.",
    ),
    (
        Api::Document,
        "set_field_cached_result_text",
        "The corpus's only field is the complex form with no `w:separate` marker, so it has no \
         cached-result slot to write and the call correctly refuses. A fixture with a field carrying \
         a cached result retires this.",
    ),
    (
        Api::Workbook,
        "refresh_chart_workbook",
        "Both sheet charts in the corpus draw from a **live range** and carry no embedded workbook \
         at all, and a chart this suite authors is written with cache and workbook already in step. \
         `crates/mjx-xlsx/tests/charts.rs` exercises the patching path against a workbook that does \
         disagree.",
    ),
    (
        Api::Workbook,
        "regenerate_chart_workbook",
        "Regenerating the workbook of a chart this suite has just authored reproduces it byte for \
         byte, and the corpus's own sheet charts carry no workbook to regenerate. \
         `crates/mjx-xlsx/tests/charts.rs` exercises it against a producer-written one.",
    ),
];

/// The defects **this gate found**, each with the ticket that owns it.
///
/// A pair listed here is expected to disagree with its declaration, so its violations are collected
/// separately rather than failing the gate — and
/// [`the_defects_this_gate_found_are_exactly_the_ones_registered`] compares the two sets **in both
/// directions**. A new partial write fails at once; a fix to one of these fails too, until its entry
/// is deleted. That is what keeps a register from becoming the quiet exemption list MJXOFF-88 §7
/// names: it can only shrink, and it cannot shrink silently.
///
/// **It is empty, and that is the state to keep it in.** The one entry it opened with — MJXOFF-213,
/// a cell comment aimed at a dialogsheet refused only after `xl/comments1.xml` had been written —
/// was deleted when that fix landed, and `crates/mjx-xlsx/tests/comments.rs` pins the refusal's
/// byte-identity directly. An entry added here is a defect this project has decided to carry, so it
/// carries a ticket number saying who will stop carrying it.
pub(crate) const KNOWN_DEFECTS: &[(&str, Api, &str, &str)] = &[];

// =================================================================================================
// The sweep
// =================================================================================================

/// The one sweep every assertion below reads.
fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(sweep)
}

/// Runs every case of every surface against every fixture of that surface's format.
fn sweep() -> Report {
    let shared = Mutex::new(Report::default());
    let fixtures = mjx_fixtures::package_fixtures();

    std::thread::scope(|scope| {
        for name in &fixtures {
            scope.spawn(|| {
                let one = sweep_fixture(name);
                let mut report = shared.lock().expect("the report mutex");
                merge(&mut report, one);
            });
        }
    });

    let mut report = shared.into_inner().expect("the report mutex");
    // Every method is a key whether or not its format had a fixture, so a case that ran nowhere is
    // visible rather than absent.
    for (api, method) in registered_methods() {
        report.cases.entry((api, method)).or_default();
    }
    report.violations.sort();
    report
}

/// Folds one fixture's tallies into the shared report.
fn merge(report: &mut Report, one: Report) {
    report.pairs += one.pairs;
    for (fixture, pairs) in one.fixtures {
        *report.fixtures.entry(fixture).or_default() += pairs;
    }
    for (key, tally) in one.cases {
        let slot = report.cases.entry(key).or_default();
        slot.applied += tally.applied;
        slot.no_op += tally.no_op;
        slot.refused += tally.refused;
        slot.not_applicable += tally.not_applicable;
    }
    report.violations.extend(one.violations);
    report.known.extend(one.known);
}

/// One fixture, driven through every case of its surface.
fn sweep_fixture(name: &str) -> Report {
    let mut report = Report::default();
    let before = mjx_fixtures::fixture(name);
    report.fixtures.insert(name.to_owned(), 0);

    if name.ends_with(".pptx") {
        deck_cases::sweep(name, &before, &mut report);
    } else if name.ends_with(".docx") {
        document_cases::sweep(name, &before, &mut report);
    } else if name.ends_with(".xlsx") {
        workbook_cases::sweep(name, &before, &mut report);
    } else {
        report.violations.push(format!(
            "{name}: a package fixture whose extension reaches no facade surface — \
             mjx_fixtures::PACKAGE_EXTENSIONS and this sweep have diverged"
        ));
    }
    report
}

/// Records one pair: classify the outcome, check the declaration, tally.
///
/// `saved` is the package as `save()` produced it after the call. A save that itself fails is a
/// violation, never a quiet skip: `save()` is the user's path, and a call that leaves a package
/// unsaveable has broken the file whether or not it touched a part.
#[allow(
    clippy::too_many_arguments,
    reason = "one pair's whole record: where it ran, what it declared, what it did and what came \
              back. Bundling them into a struct would name the same eight things one line further \
              from the call that fills them in."
)]
pub(crate) fn record(
    report: &mut Report,
    fixture: &str,
    api: Api,
    method: &'static str,
    touches: Touches,
    step: Step,
    before: &[u8],
    saved: Option<Result<Vec<u8>, Error>>,
) {
    let key = (api, method.to_owned());
    let tally = report.cases.entry(key).or_default();
    report.pairs += 1;
    *report.fixtures.entry(fixture.to_owned()).or_default() += 1;

    let Step::Ran(result) = step else {
        tally.not_applicable += 1;
        return;
    };
    let known = is_known_defect(fixture, api, method);
    macro_rules! fault {
        ($($arg:tt)*) => {
            if known {
                report
                    .known
                    .insert((fixture.to_owned(), api, method.to_owned()));
            } else {
                report.violations.push(format!($($arg)*));
            }
        };
    }
    let refused = result.is_err();

    let saved = match saved.expect("a call that ran is always followed by a save") {
        Ok(bytes) => bytes,
        Err(error) => {
            fault!("{fixture}: {api}::{method} left the package unsaveable: {error}");
            if refused {
                tally.refused += 1;
            } else {
                tally.applied += 1;
            }
            return;
        }
    };

    let difference = match diff::diff(before, &saved) {
        Ok(difference) => difference,
        Err(error) => {
            report
                .violations
                .push(format!("{fixture}: {api}::{method}: {error}"));
            return;
        }
    };

    if refused {
        tally.refused += 1;
        if !difference.is_empty() {
            fault!(
                "{fixture}: {api}::{method} refused and still changed the package — an edit is \
                 all-or-nothing:\n{}",
                difference.describe()
            );
        }
        return;
    }

    if difference.is_empty() {
        tally.no_op += 1;
    } else {
        tally.applied += 1;
    }
    for violation in touches.check(&difference, !difference.is_empty()) {
        fault!("{fixture}: {api}::{method}: {violation}");
    }
}

/// Reports that a fixture could not be opened at all through its surface. Never a skip: a corpus
/// member the facade cannot open is a defect, and a sweep that stepped over it would report a
/// smaller number and no reason.
pub(crate) fn record_unopenable(report: &mut Report, fixture: &str, api: Api, error: &Error) {
    report.violations.push(format!(
        "{fixture}: {api}::open refused the fixture: {error}"
    ));
}

/// Every `(surface, method)` the registry states.
fn registered_methods() -> BTreeSet<(Api, String)> {
    let mut methods = BTreeSet::new();
    methods.extend(
        deck_cases::cases()
            .into_iter()
            .map(|case| (Api::Deck, case.method.to_owned())),
    );
    methods.extend(
        document_cases::cases()
            .into_iter()
            .map(|case| (Api::Document, case.method.to_owned())),
    );
    methods.extend(
        workbook_cases::cases()
            .into_iter()
            .map(|case| (Api::Workbook, case.method.to_owned())),
    );
    methods
}

// =================================================================================================
// The assertions
// =================================================================================================

/// **The gate.** Every pair either did what its method declares, or did nothing at all.
#[test]
fn every_call_touches_only_what_it_declares() {
    let report = report();
    assert!(
        report.violations.is_empty(),
        "{} preservation violation(s) over {} fixture × method pairs:\n{}",
        report.violations.len(),
        report.pairs,
        report.violations.join("\n")
    );
}

/// The registry and the facade's own source agree, **in both directions**.
///
/// This is the assertion that keeps the next method from landing outside the gate. It is not a
/// count: a count is satisfied by any 452 names.
#[test]
fn the_registry_and_the_facade_agree_in_both_directions() {
    let derived = enumeration::facade_mutating_methods();
    let registered = registered_methods();

    let missing: Vec<String> = derived
        .difference(&registered)
        .map(|(api, method)| format!("  {api}::{method}"))
        .collect();
    let stale: Vec<String> = registered
        .difference(&derived)
        .map(|(api, method)| format!("  {api}::{method}"))
        .collect();

    assert!(
        missing.is_empty(),
        "{} public `&mut self` method(s) of the facade are outside the preservation gate. Register \
         each in the case table of its surface — a method the gate does not exercise is a method \
         the next MJXOFF-208 lands on:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert!(
        stale.is_empty(),
        "{} registered case(s) name a method the facade no longer has:\n{}",
        stale.len(),
        stale.join("\n")
    );
    assert_eq!(derived.len(), registered.len());
}

/// The sweep visited exactly the committed corpus, **in both directions**.
#[test]
fn the_sweep_visited_every_committed_fixture_and_no_other() {
    let report = report();
    let corpus: BTreeSet<String> = mjx_fixtures::package_fixtures().into_iter().collect();
    let visited: BTreeSet<String> = report.fixtures.keys().cloned().collect();

    assert_eq!(
        corpus, visited,
        "the corpus and the fixtures this sweep visited have diverged; `mjx_fixtures` is the only \
         list, and a fixture added to it joins this suite by being there"
    );
    let unswept: Vec<&String> = report
        .fixtures
        .iter()
        .filter(|(_, pairs)| **pairs == 0)
        .map(|(name, _)| name)
        .collect();
    assert!(
        unswept.is_empty(),
        "these fixtures were visited and no pair ran against them: {unswept:?}"
    );
}

/// Every package extension the corpus recognizes reaches a facade surface, **in both directions**.
#[test]
fn every_package_extension_reaches_a_surface() {
    let swept: BTreeSet<&str> = [Api::Deck, Api::Document, Api::Workbook]
        .into_iter()
        .map(Api::extension)
        .collect();
    let known: BTreeSet<&str> = mjx_fixtures::PACKAGE_EXTENSIONS.iter().copied().collect();
    assert_eq!(
        known, swept,
        "a package extension with no surface would put a whole format's fixtures outside this gate \
         while every count stayed green"
    );
}

/// Every method either did what it declares somewhere, or is in the register saying why not —
/// compared **in both directions**.
#[test]
fn every_method_either_works_somewhere_or_says_why_not() {
    let report = report();
    let registered: BTreeSet<(Api, String)> = NEVER_EXERCISED
        .iter()
        .map(|(api, method, _)| (*api, (*method).to_owned()))
        .collect();

    let mut observed = BTreeSet::new();
    for ((api, method), tally) in &report.cases {
        let touches = touches_of(*api, method);
        let worked = if touches.is_reader() {
            tally.ever_succeeded()
        } else {
            tally.applied > 0
        };
        if !worked {
            observed.insert((*api, method.clone()));
        }
    }

    let unregistered: Vec<String> = observed
        .difference(&registered)
        .map(|(api, method)| {
            let tally = report
                .cases
                .get(&(*api, method.clone()))
                .copied()
                .unwrap_or_default();
            format!(
                "  {api}::{method} — applied {}, no-op {}, refused {}, no address {}",
                tally.applied, tally.no_op, tally.refused, tally.not_applicable
            )
        })
        .collect();
    let retired: Vec<String> = registered
        .difference(&observed)
        .map(|(api, method)| format!("  {api}::{method}"))
        .collect();

    assert!(
        unregistered.is_empty(),
        "{} method(s) never once did what they declare. Either the case does not reach them, or \
         the corpus holds nothing they can act on — in which case add the entry to \
         NEVER_EXERCISED with the content that would retire it:\n{}",
        unregistered.len(),
        unregistered.join("\n")
    );
    assert!(
        retired.is_empty(),
        "{} NEVER_EXERCISED entr(y|ies) are now exercised; delete them, the register is a claim \
         about the corpus and this one has stopped being true:\n{}",
        retired.len(),
        retired.join("\n")
    );
}

/// The defects this gate found are exactly the ones registered, **in both directions**.
#[test]
fn the_defects_this_gate_found_are_exactly_the_ones_registered() {
    let report = report();
    let registered: BTreeSet<(String, Api, String)> = KNOWN_DEFECTS
        .iter()
        .map(|(fixture, api, method, _)| ((*fixture).to_owned(), *api, (*method).to_owned()))
        .collect();

    let fixed: Vec<String> = registered
        .difference(&report.known)
        .map(|(fixture, api, method)| format!("  {fixture}: {api}::{method}"))
        .collect();
    assert!(
        fixed.is_empty(),
        "{} registered defect(s) no longer reproduce. If the fix landed, delete the entry and \
         close its ticket; the register is a claim about today's code:\n{}",
        fixed.len(),
        fixed.join("\n")
    );
    assert_eq!(report.known, registered);
}

/// The size of the sweep, reported and floored.
///
/// The floor is deliberately the weakest assertion in the file, and it is here only to say that the
/// driver is alive. What says the sweep is *complete* is the four both-directions comparisons above.
#[test]
fn the_sweep_reports_what_it_ran() {
    let report = report();
    let totals = report.totals();
    println!(
        "MJXOFF-210 preservation gate: {} fixture × method pairs over {} fixtures and {} methods\n  \
         applied {}  no-op {}  refused {}  no address {}",
        report.pairs,
        report.fixtures.len(),
        report.cases.len(),
        totals.applied,
        totals.no_op,
        totals.refused,
        totals.not_applicable
    );
    assert_eq!(
        totals.applied + totals.no_op + totals.refused + totals.not_applicable,
        report.pairs,
        "every pair lands in exactly one outcome"
    );
    assert!(
        report.pairs >= 5_000,
        "the sweep ran {} pairs; a preservation gate that exercises a handful of them is the \
         MJXOFF-88 §7 shape",
        report.pairs
    );
    assert!(
        totals.applied >= 1_000,
        "only {} pair(s) actually changed anything; a sweep that only ever refuses proves nothing",
        totals.applied
    );
}

/// The declaration a method was registered with — for the register comparison above.
fn touches_of(api: Api, method: &str) -> Touches {
    match api {
        Api::Deck => deck_cases::cases()
            .into_iter()
            .find(|case| case.method == method)
            .map(|case| case.touches),
        Api::Document => document_cases::cases()
            .into_iter()
            .find(|case| case.method == method)
            .map(|case| case.touches),
        Api::Workbook => workbook_cases::cases()
            .into_iter()
            .find(|case| case.method == method)
            .map(|case| case.touches),
    }
    .unwrap_or(rules::NOTHING)
}
