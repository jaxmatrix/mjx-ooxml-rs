//! **Deriving a state from evidence** — the half of the ledger that decides, and never looks.
//!
//! [`assess`] is a pure function of a [`Capability`] and an [`Evidence`] index. It touches no
//! filesystem, so every rule below is testable against a synthetic index — which is how the three
//! guarantees the ticket asks to be *demonstrated* are demonstrated here rather than described:
//!
//! * an uncovered capability is `not-started`, and no other default exists;
//! * a capability whose suite exists but asserts nothing is `not-started` as well;
//! * removing a suite moves the rows that named it.
//!
//! # The five states, and how each one is reached
//!
//! | State | Reached when |
//! |---|---|
//! | `out-of-scope` | the row is a §2 exclusion and carries its reason |
//! | `not-started` | **the default** — no evidence, or no evidence that asserts anything |
//! | `preserved-not-rendered` | a document capability with evidence, none of it in the layout or rendering tier |
//! | `partial` | a suite declares a limitation it asserts; or a document capability whose drawing evidence is only layout tier, or only rendering suites whose test doubles stand in for it |
//! | `implemented` | rendering-tier evidence with no double standing in for the row, and no declared limitation |
//!
//! # Why `implemented` cannot be the default, and is not reachable by accident
//!
//! The ticket's trap (a): *a generator that defaults an unknown row to `implemented` produces a
//! document that is confidently false.* The shape of the match below is the answer — `implemented`
//! is the **last** arm and it is reached only after a row has been shown to have evidence, to have
//! evidence that asserts something, and to have evidence in a crate that draws. Every earlier
//! condition is a way of *not* getting there.
//!
//! # What a state does not mean
//!
//! `implemented` means **the suites this row names cover it and assert something**. It does not
//! mean the output matches Microsoft Office, because nothing in this workspace has ever been
//! compared against Microsoft Office. The provenance split carried alongside every row is what
//! keeps that visible: a row whose expectations are entirely `EngineDerived` is covered by change
//! detectors, and a change detector is not evidence.

use std::collections::BTreeSet;

use anyhow::Result;

use super::evidence::{tier_of, Double, Evidence, Split, Tier};
use super::rows::{Capability, Kind, STAND_INS};

/// What a ledger row says about one capability.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum State {
    /// Covered by suites that assert something, in a crate that draws.
    Implemented,
    /// Covered, and a suite declares a limitation it asserts.
    Partial,
    /// The markup round-trips; nothing draws it.
    PreservedNotRendered,
    /// Nothing tests it. **The default.**
    NotStarted,
    /// Excluded by decision, with the reason attached.
    OutOfScope,
}

impl State {
    /// Every state, in the order the summary prints them.
    pub(crate) const ALL: [Self; 5] = [
        Self::Implemented,
        Self::Partial,
        Self::PreservedNotRendered,
        Self::NotStarted,
        Self::OutOfScope,
    ];

    /// The name the generated document writes.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Partial => "partial",
            Self::PreservedNotRendered => "preserved-not-rendered",
            Self::NotStarted => "not-started",
            Self::OutOfScope => "out-of-scope",
        }
    }

    /// What the state means, printed once in the generated document's key.
    pub(crate) fn meaning(self) -> &'static str {
        match self {
            Self::Implemented => {
                "suites that emit a display list or pixels cover it, with no test double standing \
                 in for it, and they assert something. **Not** a claim that the output matches \
                 Office"
            }
            Self::Partial => {
                "covered, and either a suite declares a limitation it asserts, or nothing proves it \
                 is drawn — the evidence stops at a fragment tree, or every suite that draws it \
                 does so through a test double. The reason is quoted"
            }
            Self::PreservedNotRendered => {
                "the markup round-trips faithfully and nothing lays it out or draws it. A \
                 legitimate, permanent state"
            }
            Self::NotStarted => {
                "nothing tests it, whatever anyone believes about it. This is the default, and it \
                 is what an unrecognised row becomes"
            }
            Self::OutOfScope => "excluded by decision, with the reason in the row",
        }
    }
}

/// One assessed row: what was asked, what was found, and what follows.
#[derive(Clone, Debug)]
pub(crate) struct Assessed {
    /// The capability, as declared.
    pub(crate) capability: &'static Capability,
    /// The state derived from the evidence.
    pub(crate) state: State,
    /// The provenance of every expectation the evidence declares.
    pub(crate) split: Split,
    /// Tests and assertions across the evidence.
    pub(crate) tests: usize,
    /// Assertions across the evidence.
    pub(crate) assertions: usize,
    /// The limitations the evidence declares, quoted from the suites that assert them.
    pub(crate) limitations: Vec<(String, String)>,
    /// Why a document capability with evidence is not drawn, derived from the tiers and doubles.
    pub(crate) caps: Vec<String>,
}

/// The doubles that are not evidence for the row `id`.
fn doubles_standing_in_for(id: &str) -> BTreeSet<Double> {
    STAND_INS
        .iter()
        .filter(|(_, rows)| rows.contains(&id))
        .map(|(double, _)| *double)
        .collect()
}

/// Derives one row's state from the evidence, or fails if the row names a suite that is not there.
pub(crate) fn assess(capability: &'static Capability, evidence: &Evidence) -> Result<Assessed> {
    let mut split = Split::default();
    let mut tests = 0;
    let mut assertions = 0;
    let mut limitations = Vec::new();
    let mut renders = false;
    let mut checks_something = false;
    let mut laid_out_by: BTreeSet<&str> = BTreeSet::new();
    let mut drawn_through_doubles: Vec<(&str, Vec<Double>)> = Vec::new();
    let standing_in = doubles_standing_in_for(capability.id);

    for path in capability.evidence {
        // The liveness check. A missing suite is an error, never a lower state — a row that
        // silently degraded when its suite was deleted would report a fact about the ledger's
        // rot as though it were a fact about the product.
        let suite = evidence.suite(path)?;
        split.add(suite.split);
        tests += suite.tests;
        assertions += suite.assertions;
        checks_something |= suite.checks_something();
        for limitation in &suite.limitations {
            limitations.push(((*path).to_owned(), limitation.clone()));
        }
        // A suite that asserts nothing is no tier's evidence.
        if !suite.checks_something() {
            continue;
        }
        match tier_of(path, &suite.crate_name) {
            Tier::Rendering => {
                let doubles: Vec<Double> = standing_in
                    .iter()
                    .copied()
                    .filter(|double| evidence.uses_double(path, suite, *double))
                    .collect();
                if doubles.is_empty() {
                    renders = true;
                } else {
                    drawn_through_doubles.push((path, doubles));
                }
            }
            Tier::Layout => {
                laid_out_by.insert(suite.crate_name.as_str());
            }
            Tier::Model => {}
        }
    }

    let undrawn = capability.kind == Kind::Rendered && !renders;
    let state = if capability.kind == Kind::Excluded {
        State::OutOfScope
    } else if capability.evidence.is_empty() || !checks_something {
        // **The default, and the only default.** Nothing tests it, or what tests it asserts
        // nothing, which for this purpose is the same thing.
        State::NotStarted
    } else if undrawn && laid_out_by.is_empty() && drawn_through_doubles.is_empty() {
        State::PreservedNotRendered
    } else if undrawn || !limitations.is_empty() {
        State::Partial
    } else {
        State::Implemented
    };

    let mut caps = Vec::new();
    if state == State::Partial && undrawn {
        if !laid_out_by.is_empty() {
            let crates: Vec<String> = laid_out_by.iter().map(|name| format!("`{name}`")).collect();
            caps.push(format!(
                "no rendering-tier suite proves it is drawn: the evidence stops at the fragment \
                 tier ({})",
                crates.join(", ")
            ));
        }
        for (path, doubles) in &drawn_through_doubles {
            let named: Vec<String> = doubles
                .iter()
                .map(|double| {
                    format!(
                        "{}, standing in for {}",
                        double.name(),
                        double.stands_in_for()
                    )
                })
                .collect();
            caps.push(format!(
                "`{}` draws it only through {}",
                short_suite(path),
                named.join("; and ")
            ));
        }
    }

    Ok(Assessed {
        capability,
        state,
        split,
        tests,
        assertions,
        limitations,
        caps,
    })
}

/// `crates/mjx-scene-pptx/tests/a_thing.rs` as `mjx-scene-pptx: a_thing`.
fn short_suite(path: &str) -> String {
    let trimmed = path.strip_prefix("crates/").unwrap_or(path);
    let trimmed = trimmed.strip_suffix(".rs").unwrap_or(trimmed);
    match trimmed.split_once("/tests/") {
        Some((crate_name, stem)) => format!("{crate_name}: {stem}"),
        None => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::evidence::{CommandCensus, Evidence, Provenance, Suite};
    use super::super::rows::{Capability, Kind, Section};
    use super::*;

    /// A suite as the scanner would have read it.
    fn suite(crate_name: &str, assertions: usize, limitations: &[&str]) -> Suite {
        Suite {
            crate_name: crate_name.to_owned(),
            tests: 1,
            assertions,
            limitations: limitations.iter().map(|text| (*text).to_owned()).collect(),
            split: Split::default(),
            doubles: BTreeSet::new(),
        }
    }

    /// An evidence index holding exactly the suites named.
    fn index(entries: &[(&str, Suite)]) -> Evidence {
        Evidence {
            suites: entries
                .iter()
                .map(|(path, suite)| ((*path).to_owned(), suite.clone()))
                .collect::<BTreeMap<_, _>>(),
            provenance_not_read: Vec::new(),
            approvals: Vec::new(),
            commands: CommandCensus {
                per_application: vec![("Word".to_owned(), 1)],
            },
            declared_elements: 1,
            crates: BTreeSet::new(),
        }
    }

    /// A capability with a chosen kind and evidence. `Capability` is `'static` in the real table;
    /// these are leaked so the tests can build them, which is free in a test binary.
    fn capability(kind: Kind, evidence: &'static [&'static str]) -> &'static Capability {
        Box::leak(Box::new(Capability {
            id: "a-capability",
            section: Section::SharedText,
            capability: "a capability",
            kind,
            excluded_because: None,
            evidence,
        }))
    }

    /// **The guard against trap (a).** A row nothing covers is `not-started`, and there is no
    /// arrangement of the rules that makes an unknown row anything else.
    #[test]
    fn a_capability_with_no_evidence_is_not_started() {
        let assessed = assess(capability(Kind::Rendered, &[]), &index(&[])).expect("no lookup");
        assert_eq!(assessed.state, State::NotStarted);

        let assessed = assess(capability(Kind::Behaviour, &[]), &index(&[])).expect("no lookup");
        assert_eq!(assessed.state, State::NotStarted);
    }

    /// **The guard against trap (b).** A suite that exists and asserts nothing is green precisely
    /// when the work is skipped, so it does not promote anything.
    #[test]
    fn a_suite_that_asserts_nothing_does_not_make_a_row_implemented() {
        let evidence = index(&[(
            "crates/mjx-layout-docx/tests/a.rs",
            suite("mjx-layout-docx", 0, &[]),
        )]);
        let assessed = assess(
            capability(Kind::Rendered, &["crates/mjx-layout-docx/tests/a.rs"]),
            &evidence,
        )
        .expect("the suite exists");
        assert_eq!(assessed.state, State::NotStarted);
    }

    /// **The liveness check.** A row that names a suite which is not there fails, and the message
    /// names the path so a reader can act on it.
    #[test]
    fn a_row_naming_a_suite_that_does_not_exist_fails_the_build() {
        let error = assess(
            capability(Kind::Rendered, &["crates/mjx-layout-docx/tests/gone.rs"]),
            &index(&[]),
        )
        .expect_err("a missing suite is an error");
        let report = format!("{error:#}");
        assert!(
            report.contains("crates/mjx-layout-docx/tests/gone.rs"),
            "{report}"
        );
    }

    /// **Deleting a real suite changes the ledger.** The same row, assessed against an index with
    /// the suite and against one without it, gives different answers — which is what "generated"
    /// means and what a hand-written table cannot do.
    #[test]
    fn removing_a_suite_moves_the_row_off_implemented() {
        let row = capability(Kind::Rendered, &["crates/mjx-scene-pptx/tests/a.rs"]);
        let present = index(&[(
            "crates/mjx-scene-pptx/tests/a.rs",
            suite("mjx-scene-pptx", 4, &[]),
        )]);
        assert_eq!(
            assess(row, &present).expect("present").state,
            State::Implemented
        );
        assert!(assess(row, &index(&[])).is_err());
    }

    // A box model's suite proves a FragmentTree, not a display list, so alone it caps a rendered row at partial.
    #[test]
    fn a_layout_tier_suite_alone_caps_a_rendered_row_at_partial() {
        let row = capability(Kind::Rendered, &["crates/mjx-layout-pptx/tests/a.rs"]);
        let evidence = index(&[(
            "crates/mjx-layout-pptx/tests/a.rs",
            suite("mjx-layout-pptx", 4, &[]),
        )]);
        let assessed = assess(row, &evidence).expect("present");
        assert_eq!(assessed.state, State::Partial);
        assert!(
            assessed
                .caps
                .iter()
                .any(|cap| cap.contains("`mjx-layout-pptx`")),
            "the cap names the box model: {:?}",
            assessed.caps
        );
    }

    // A rendering suite beside a box model's suite draws the row, so the box model does not cap it.
    #[test]
    fn a_rendering_suite_beside_a_layout_suite_is_implemented() {
        let row = capability(
            Kind::Rendered,
            &[
                "crates/mjx-layout-pptx/tests/a.rs",
                "crates/mjx-scene-pptx/tests/b.rs",
            ],
        );
        let evidence = index(&[
            (
                "crates/mjx-layout-pptx/tests/a.rs",
                suite("mjx-layout-pptx", 4, &[]),
            ),
            (
                "crates/mjx-scene-pptx/tests/b.rs",
                suite("mjx-scene-pptx", 2, &[]),
            ),
        ]);
        let assessed = assess(row, &evidence).expect("present");
        assert_eq!(assessed.state, State::Implemented);
        assert!(assessed.caps.is_empty());
    }

    // A behaviour is not capped by the tier, because nothing draws an undo stack.
    #[test]
    fn a_behaviour_with_layout_tier_evidence_is_not_capped() {
        let row = capability(Kind::Behaviour, &["crates/mjx-view/tests/a.rs"]);
        let evidence = index(&[("crates/mjx-view/tests/a.rs", suite("mjx-view", 3, &[]))]);
        assert_eq!(
            assess(row, &evidence).expect("present").state,
            State::Implemented
        );
    }

    /// A document capability whose only evidence is in the model tier is `preserved-not-rendered`,
    /// which is a state and not a failure.
    #[test]
    fn markup_with_no_renderer_is_preserved_not_rendered() {
        let evidence = index(&[("crates/mjx-vml/tests/drawing.rs", suite("mjx-vml", 9, &[]))]);
        let assessed = assess(
            capability(Kind::Rendered, &["crates/mjx-vml/tests/drawing.rs"]),
            &evidence,
        )
        .expect("the suite exists");
        assert_eq!(assessed.state, State::PreservedNotRendered);
    }

    /// A §6 behaviour has no markup, so the rendering tier is not the question for it.
    #[test]
    fn a_behaviour_with_model_tier_evidence_is_implemented() {
        let evidence = index(&[(
            "crates/mjx-session/tests/undo_granularity.rs",
            suite("mjx-session", 12, &[]),
        )]);
        let assessed = assess(
            capability(
                Kind::Behaviour,
                &["crates/mjx-session/tests/undo_granularity.rs"],
            ),
            &evidence,
        )
        .expect("the suite exists");
        assert_eq!(assessed.state, State::Implemented);
    }

    /// A declared limitation demotes a row and travels with it, so a reader meets the defect in the
    /// row rather than in a footnote.
    #[test]
    fn a_declared_limitation_makes_a_row_partial_and_is_quoted() {
        let evidence = index(&[(
            "crates/mjx-scene-pptx/tests/the_opacity_is_lost_at_the_spec_boundary.rs",
            suite("mjx-scene-pptx", 6, &["every theme shadow renders solid"]),
        )]);
        let assessed = assess(
            capability(
                Kind::Rendered,
                &["crates/mjx-scene-pptx/tests/the_opacity_is_lost_at_the_spec_boundary.rs"],
            ),
            &evidence,
        )
        .expect("the suite exists");
        assert_eq!(assessed.state, State::Partial);
        assert_eq!(assessed.limitations.len(), 1);
        assert!(assessed.limitations[0].1.contains("solid"));
    }

    /// An excluded row is a row: it carries its reason and never consults the suites.
    #[test]
    fn an_exclusion_is_out_of_scope_whatever_the_suites_say() {
        let assessed = assess(capability(Kind::Excluded, &[]), &index(&[])).expect("no lookup");
        assert_eq!(assessed.state, State::OutOfScope);
    }

    /// The provenance carried alongside the state is the sum of the evidence's declarations, which
    /// is what stops a green row being read as a claim about Office.
    #[test]
    fn the_provenance_of_the_evidence_travels_with_the_row() {
        let mut declared = suite("mjx-layout-docx", 3, &[]);
        declared.split = Split {
            spec: 2,
            documented: 1,
            engine: 7,
        };
        let evidence = index(&[("crates/mjx-layout-docx/tests/a.rs", declared)]);
        let assessed = assess(
            capability(Kind::Rendered, &["crates/mjx-layout-docx/tests/a.rs"]),
            &evidence,
        )
        .expect("the suite exists");
        assert_eq!(assessed.split.total(), 10);
        assert_eq!(assessed.split.engine, 7);
        assert_eq!(Provenance::EngineDerived.name(), "EngineDerived");
    }
}
