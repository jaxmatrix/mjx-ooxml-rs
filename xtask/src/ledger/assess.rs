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
//! | `preserved-not-rendered` | a drawn capability with evidence, none of it past the markup |
//! | `partial` | a suite declares a limitation it asserts; or a drawn capability no cited function draws, or that every drawing function draws only through a test double |
//! | `implemented` | a cited test function that draws, with no double standing in for the row, and no declared limitation |
//!
//! A **drawn capability** is every rendered row, and any row a `features.json` row names, whatever
//! its kind: a checklist row is a user-visible feature, so re-kinding its ledger row cannot excuse it
//! from the question.
//!
//! # Why `implemented` cannot be the default, and is not reachable by accident
//!
//! The ticket's trap (a): *a generator that defaults an unknown row to `implemented` produces a
//! document that is confidently false.* The shape of the match below is the answer — `implemented`
//! is the **last** arm and it is reached only after a row has been shown to have evidence, to have
//! evidence that asserts something, and to cite a test function that reads what was drawn. Every
//! earlier condition is a way of *not* getting there.
//!
//! # What a state does not mean
//!
//! `implemented` means **the suites this row names cover it and assert something**. It does not
//! mean the output matches Microsoft Office, because nothing in this workspace has ever been
//! compared against Microsoft Office. The provenance split carried alongside every row is what
//! keeps that visible: a row whose expectations are entirely `EngineDerived` is covered by change
//! detectors, and a change detector is not evidence.

use std::collections::BTreeSet;

use anyhow::{bail, Result};

use super::evidence::{short_citation, Double, Evidence, Split, Tier};
use super::rows::{Capability, Kind, ALLOWANCES};

/// What a ledger row says about one capability.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum State {
    /// Covered by a cited test function that draws.
    Implemented,
    /// Covered, and a suite declares a limitation it asserts, or nothing cited proves it drawn.
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
                "a cited test function reads the display list or pixels it drew, with no test \
                 double standing in for the row, and the evidence asserts something. **Not** a \
                 claim that the output matches Office"
            }
            Self::Partial => {
                "covered, and either a suite declares a limitation it asserts, or nothing proves it \
                 is drawn — no cited function reads a display list or pixels, or every one that \
                 does draws through a test double. The reason is quoted"
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
    /// Tests across the evidence.
    pub(crate) tests: usize,
    /// Assertions across the evidence.
    pub(crate) assertions: usize,
    /// The limitations the evidence declares, quoted from the suites that assert them.
    pub(crate) limitations: Vec<(String, String)>,
    /// Why a drawn capability with evidence is not drawn, derived from the tiers and doubles.
    pub(crate) caps: Vec<String>,
}

/// Whether a row is asked whether it is drawn: every rendered row, and any row the checklist names.
pub(crate) fn asks_whether_drawn(capability: &Capability, evidence: &Evidence) -> bool {
    match capability.kind {
        Kind::Rendered => true,
        Kind::Behaviour => !evidence.checklist_rows_naming(capability.id).is_empty(),
        Kind::Excluded => false,
    }
}

/// Whether an allowance names this row's use of `double` through the suite at `path`.
fn allowed(id: &str, path: &str, double: Double) -> bool {
    ALLOWANCES.iter().any(|allowance| {
        allowance.row == id && allowance.suite == path && allowance.double == double
    })
}

/// Derives one row's state from the evidence, or fails if the row cites evidence that is not there.
pub(crate) fn assess(capability: &'static Capability, evidence: &Evidence) -> Result<Assessed> {
    let drawn_question = asks_whether_drawn(capability, evidence);
    let mut split = Split::default();
    let mut tests = 0;
    let mut assertions = 0;
    let mut limitations = Vec::new();
    let mut renders = false;
    let mut checks_something = false;
    let mut short_of_drawn: BTreeSet<&str> = BTreeSet::new();
    let mut drawn_through_doubles: Vec<(&str, Vec<Double>)> = Vec::new();
    let mut suites_read: BTreeSet<&str> = BTreeSet::new();

    for citation in capability.evidence {
        // The liveness check. A missing suite or function is an error, never a lower state — a row
        // that silently degraded when its evidence was deleted would report a fact about the
        // ledger's rot as though it were a fact about the product.
        let cited = evidence.cite(citation)?;
        if suites_read.insert(cited.path) {
            split.add(cited.suite.split);
            for limitation in &cited.suite.limitations {
                limitations.push((cited.path.to_owned(), limitation.clone()));
            }
        }
        tests += cited.tests;
        assertions += cited.assertions;
        if drawn_question && cited.function.is_none() && !cited.suite.drawing_tests.is_empty() {
            let functions: Vec<String> = cited
                .suite
                .drawing_tests
                .iter()
                .map(|name| format!("`{citation}::{name}`"))
                .collect();
            bail!(
                "`{}` is asked whether it is drawn and cites the whole of `{citation}`, which holds \
                 functions that draw. A whole suite is never rendering evidence: cite the functions \
                 that draw this capability, from {}, or none of them",
                capability.id,
                functions.join(", ")
            );
        }
        // Evidence that asserts nothing is no tier's evidence.
        if cited.assertions == 0 {
            continue;
        }
        checks_something = true;
        match cited.tier() {
            Tier::Rendering => {
                let doubles: Vec<Double> = cited
                    .suite
                    .doubles
                    .iter()
                    .copied()
                    .filter(|double| !allowed(capability.id, cited.path, *double))
                    .collect();
                if doubles.is_empty() {
                    renders = true;
                } else {
                    drawn_through_doubles.push((citation, doubles));
                }
            }
            Tier::Layout => {
                short_of_drawn.insert(cited.suite.crate_name.as_str());
            }
            Tier::Model => {}
        }
    }

    let undrawn = drawn_question && !renders;
    let state = if capability.kind == Kind::Excluded {
        State::OutOfScope
    } else if capability.evidence.is_empty() || !checks_something {
        // **The default, and the only default.** Nothing tests it, or what tests it asserts
        // nothing, which for this purpose is the same thing.
        State::NotStarted
    } else if undrawn && short_of_drawn.is_empty() && drawn_through_doubles.is_empty() {
        State::PreservedNotRendered
    } else if undrawn || !limitations.is_empty() {
        State::Partial
    } else {
        State::Implemented
    };

    let mut caps = Vec::new();
    if state == State::Partial && undrawn {
        if !short_of_drawn.is_empty() {
            let crates: Vec<String> = short_of_drawn
                .iter()
                .map(|name| format!("`{name}`"))
                .collect();
            caps.push(format!(
                "no cited test function reads a display list or pixels: the evidence stops short \
                 of drawn ({})",
                crates.join(", ")
            ));
        }
        for (citation, doubles) in &drawn_through_doubles {
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
                short_citation(citation),
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

#[cfg(test)]
mod tests {
    use super::super::evidence::{read_suite, Evidence, Provenance, Suite};
    use super::super::rows::{Capability, Kind, Section};
    use super::*;

    /// A suite as the scanner would have read it, with no test function that draws.
    fn suite(crate_name: &str, assertions: usize, limitations: &[&str]) -> Suite {
        let mut suite = read_suite(crate_name, "#[test]\nfn checks() {}\n");
        suite.assertions = assertions;
        suite.limitations = limitations.iter().map(|text| (*text).to_owned()).collect();
        suite
    }

    // A suite whose one test function reads the display list's commands.
    fn drawing_suite(crate_name: &str) -> Suite {
        read_suite(
            crate_name,
            "use mjx_scene::DisplayList;\n#[test]\nfn draws() { assert!(list.commands().count() > 0); }\n",
        )
    }

    /// An evidence index holding exactly the suites named.
    fn index(entries: &[(&str, Suite)]) -> Evidence {
        Evidence::of(entries, &[])
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

    // A citation naming a test function its suite does not declare fails, naming the citation.
    #[test]
    fn a_row_naming_a_function_that_does_not_exist_fails_the_build() {
        let evidence = index(&[(
            "crates/mjx-scene-pptx/tests/a.rs",
            drawing_suite("mjx-scene-pptx"),
        )]);
        let error = assess(
            capability(Kind::Rendered, &["crates/mjx-scene-pptx/tests/a.rs::gone"]),
            &evidence,
        )
        .expect_err("a missing function is an error");
        assert!(format!("{error:#}").contains("a.rs::gone"), "{error:#}");
    }

    /// **Deleting a real suite changes the ledger.** The same row, assessed against an index with
    /// the suite and against one without it, gives different answers — which is what "generated"
    /// means and what a hand-written table cannot do.
    #[test]
    fn removing_a_suite_moves_the_row_off_implemented() {
        let row = capability(Kind::Rendered, &["crates/mjx-scene-pptx/tests/a.rs::draws"]);
        let present = index(&[(
            "crates/mjx-scene-pptx/tests/a.rs",
            drawing_suite("mjx-scene-pptx"),
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

    // A drawing function beside a box model's suite draws the row, so the box model does not cap it.
    #[test]
    fn a_rendering_suite_beside_a_layout_suite_is_implemented() {
        let row = capability(
            Kind::Rendered,
            &[
                "crates/mjx-layout-pptx/tests/a.rs",
                "crates/mjx-scene-pptx/tests/b.rs::draws",
            ],
        );
        let evidence = index(&[
            (
                "crates/mjx-layout-pptx/tests/a.rs",
                suite("mjx-layout-pptx", 4, &[]),
            ),
            (
                "crates/mjx-scene-pptx/tests/b.rs",
                drawing_suite("mjx-scene-pptx"),
            ),
        ]);
        let assessed = assess(row, &evidence).expect("present");
        assert_eq!(assessed.state, State::Implemented);
        assert!(assessed.caps.is_empty());
    }

    // A whole suite that holds a drawing function is refused on a drawn row, and the refusal names the function to cite.
    #[test]
    fn a_whole_suite_that_draws_is_refused_on_a_drawn_row() {
        let evidence = index(&[(
            "crates/mjx-scene-pptx/tests/b.rs",
            drawing_suite("mjx-scene-pptx"),
        )]);
        let error = assess(
            capability(Kind::Rendered, &["crates/mjx-scene-pptx/tests/b.rs"]),
            &evidence,
        )
        .expect_err("a whole drawing suite is refused");
        assert!(format!("{error:#}").contains("b.rs::draws"), "{error:#}");
    }

    // A behaviour no checklist row names is not capped by the tier, because nothing draws an undo stack.
    #[test]
    fn a_behaviour_with_layout_tier_evidence_is_not_capped() {
        let row = capability(Kind::Behaviour, &["crates/mjx-view/tests/a.rs"]);
        let evidence = index(&[("crates/mjx-view/tests/a.rs", suite("mjx-view", 3, &[]))]);
        assert_eq!(
            assess(row, &evidence).expect("present").state,
            State::Implemented
        );
    }

    // A behaviour a checklist row names is asked whether it is drawn, so layout evidence alone caps it.
    #[test]
    fn a_behaviour_a_checklist_row_names_is_asked_whether_it_is_drawn() {
        let row = capability(Kind::Behaviour, &["crates/mjx-layout-docx/tests/a.rs"]);
        let evidence = Evidence::of(
            &[(
                "crates/mjx-layout-docx/tests/a.rs",
                suite("mjx-layout-docx", 3, &[]),
            )],
            &[("a-capability", "docx-a-feature")],
        );
        let assessed = assess(row, &evidence).expect("present");
        assert_eq!(assessed.state, State::Partial);
        assert!(!assessed.caps.is_empty(), "the cap says why");
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
