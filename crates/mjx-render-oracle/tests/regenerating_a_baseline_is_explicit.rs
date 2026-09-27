//! **A regenerate-baselines path that can run on its own silently ratifies every regression it
//! exists to catch.**
//!
//! Two properties, and both are mechanical rather than documented:
//!
//! 1. [`Baselines::regenerate`] takes a [`RegenerationIntent`], and the only way to obtain one is
//!    [`RegenerationIntent::from_environment`]. A check path cannot construct one by accident
//!    because it cannot construct one at all — the type has a private field, no `Default` and no
//!    `new`.
//! 2. Regeneration **removes the approval**. Writing new artefacts beside an old approval is exactly
//!    how an auto-regenerating baseline ratifies what it should have caught, so the record goes
//!    first — before the artefacts, so that a failure halfway through leaves a baseline that refuses
//!    to be used rather than one approving bytes that are no longer there.
//!
//! # Why this file has one case
//!
//! It manipulates a process-global environment variable, and Cargo runs a binary's cases on several
//! threads at once. A second `#[test]` here would race the first for the variable, and the failure
//! would look like the gate misbehaving rather than like the suite doing so. An integration test is
//! its own process, so nothing outside this file sees the variable at all.

use std::path::PathBuf;

use mjx_render_oracle::baseline::{Approver, Baselines, RegenerationIntent, ARTEFACTS, REGENERATE};
use mjx_render_oracle::specimen::{Perturbation, Specimen};
use mjx_render_oracle::{png, snapshot, specimen};

const SUBJECT: &str = "solid-panels";

#[test]
fn regeneration_needs_a_stated_intent_and_removes_the_approval() {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/oracle-scratch/regeneration");
    let _ = std::fs::remove_dir_all(&directory);
    let baselines = Baselines::at(&directory);
    let subject = Specimen::named(SUBJECT).expect("the specimen is in the corpus");

    // ---------------------------------------------------------------------------------------
    // Without the variable, there is no intent to be had — and therefore no way to call
    // `regenerate` at all, because its first argument does not exist.
    // ---------------------------------------------------------------------------------------
    std::env::remove_var(REGENERATE);
    let refused = RegenerationIntent::from_environment()
        .expect_err("an intent must not be available by default");
    assert!(
        refused.contains(REGENERATE),
        "the refusal does not say what to set: {refused}"
    );
    assert!(
        refused.contains("unapproved until you do"),
        "the refusal does not say that regenerating un-approves: {refused}"
    );

    // A value that is not `1` is not a statement of intent either — `MJX_ORACLE_REGENERATE=0` reads
    // to a person as *off*, and a check for mere presence would read it as *on*.
    std::env::set_var(REGENERATE, "0");
    assert!(RegenerationIntent::from_environment().is_err());

    // ---------------------------------------------------------------------------------------
    // With it, the artefacts are written and there is no approval beside them.
    // ---------------------------------------------------------------------------------------
    std::env::set_var(REGENERATE, "1");
    let intent = RegenerationIntent::from_environment().expect("the intent is now available");
    let tree = specimen::fragments(&subject, Perturbation::None);
    let list = specimen::display_list(&subject, Perturbation::None).expect("a display list");
    let rendered = specimen::render(&subject, Perturbation::None).expect("a render");
    let artefacts = [
        (
            ARTEFACTS[0],
            snapshot::fragments(&tree).to_text().into_bytes(),
        ),
        (
            ARTEFACTS[1],
            snapshot::commands(&list).to_text().into_bytes(),
        ),
        (
            ARTEFACTS[2],
            png::encode(&png::image_from_pixels(&rendered.pixels)).expect("an image"),
        ),
    ];
    baselines
        .regenerate(intent, SUBJECT, &artefacts)
        .expect("regeneration writes");
    assert!(
        !baselines.approval(SUBJECT).is_usable(),
        "**a freshly regenerated baseline is approved.** That is the tautology this whole crate is \
         written against: the image was produced by the code that will be compared against it."
    );

    // ---------------------------------------------------------------------------------------
    // Approve it, then regenerate again: the approval must not survive.
    // ---------------------------------------------------------------------------------------
    baselines
        .approve(
            SUBJECT,
            Approver::Human {
                name: "the regeneration gate".to_owned(),
            },
            "approved so that the next regeneration has something to remove",
        )
        .expect("approving");
    assert!(baselines.approval(SUBJECT).is_usable());
    assert!(baselines.approval(SUBJECT).is_reviewed());

    let intent = RegenerationIntent::from_environment().expect("still available");
    baselines
        .regenerate(intent, SUBJECT, &artefacts)
        .expect("regeneration writes again");
    assert!(
        !baselines.approval(SUBJECT).is_usable(),
        "**the approval survived a regeneration.** The artefacts happen to be identical here, which \
         is the case that makes this dangerous: a path that kept the approval would keep it for a \
         regeneration that changed everything, and nothing would say so."
    );

    // And the committed baselines are untouched by all of the above, which is the property that
    // makes this file safe to run in the same suite as the rest.
    assert!(
        Baselines::committed().approval(SUBJECT).is_usable(),
        "this case reached the committed baselines"
    );

    std::env::remove_var(REGENERATE);
    let _ = std::fs::remove_dir_all(&directory);
}
