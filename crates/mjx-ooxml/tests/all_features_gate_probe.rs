//! TEMPORARY — MJXOFF-110 clause 7. Removed by the commit that follows this one.
//!
//! A case that fails **only** under `cargo test --workspace --all-features`. `lint-test` runs the
//! default features and `--features mjx-pptx/vml`; neither enables `mjx-ooxml/vml`, so neither
//! compiles this file. It exists to make the new `test (--all-features)` job go red on a real pull
//! request, and to show the merge step refuse, rather than to assert that it would.

#[test]
#[cfg(feature = "vml")]
fn the_all_features_job_is_the_only_thing_that_runs_this() {
    panic!(
        "deliberate failure — MJXOFF-110 clause 7. Reached only under --all-features; the next \
         commit removes this file."
    );
}
