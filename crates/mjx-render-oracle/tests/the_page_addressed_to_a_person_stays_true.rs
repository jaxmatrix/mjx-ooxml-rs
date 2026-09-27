//! `docs/validation/08-the-fidelity-oracle.md` is the half of this crate a person reads, and it is
//! held to the code rather than trusted.
//!
//! # Why a documentation test at all
//!
//! Every other gate here is mechanical, and the one step that is not — *a person looks at the
//! picture and says whether it is right* — is described only in prose. Prose rots: a specimen
//! renamed here and not there leaves a page telling somebody to approve a plate that does not exist,
//! and the approval they give is then an approval of nothing.
//!
//! So the page's plate table is compared against [`SPECIMENS`], its command examples against the
//! environment variables the code actually reads, and its §2 against what the committed baselines
//! actually say. The last is the important one: **§2 claims that nobody has looked at these images**,
//! and the day that stops being true the claim has to move rather than quietly staying.

use std::path::PathBuf;

use mjx_render_oracle::baseline::{awaiting_human_review, APPROVED_BY, REGENERATE};
use mjx_render_oracle::specimen::SPECIMENS;
use mjx_render_oracle::Baselines;

fn page() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/validation/08-the-fidelity-oracle.md");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "the page addressed to a person is missing: {}: {error}",
            path.display()
        )
    })
}

#[test]
fn the_page_names_every_specimen_and_no_others() {
    let text = page();
    for specimen in SPECIMENS {
        assert!(
            text.contains(&format!("`{}`", specimen.name)),
            "the page does not tell a person what `{}` should look like, so they cannot approve it",
            specimen.name
        );
    }
    // And nothing it names has gone away. A page that told somebody to approve `rounded-panels`
    // would send them to a command that fails, which is how a documentation page teaches a reader
    // to ignore it.
    for line in text.lines().filter(|line| line.starts_with("| `")) {
        let named = line
            .trim_start_matches("| `")
            .split('`')
            .next()
            .unwrap_or_default();
        assert!(
            SPECIMENS.iter().any(|specimen| specimen.name == named),
            "the page's plate table names `{named}`, which is not a specimen"
        );
    }
}

#[test]
fn the_page_quotes_the_variables_the_code_actually_reads() {
    let text = page();
    for variable in [REGENERATE, APPROVED_BY] {
        assert!(
            text.contains(variable),
            "the page does not mention `{variable}`, so a reader following it would be refused \
             with no idea why"
        );
    }
    // The two commands a person actually types, spelled the way `main.rs` accepts them.
    for command in [
        "cargo run -p mjx-render-oracle -- gallery",
        "cargo run -p mjx-render-oracle -- approve",
        "cargo run -p mjx-render-oracle -- regenerate",
        "cargo run -p mjx-render-oracle -- list",
    ] {
        assert!(text.contains(command), "the page does not give `{command}`");
    }
}

#[test]
fn the_pages_claim_that_nobody_has_looked_is_still_true() {
    let text = page();
    let names: Vec<&str> = SPECIMENS.iter().map(|specimen| specimen.name).collect();
    let awaiting = awaiting_human_review(&Baselines::committed(), &names);
    let claims_nobody_has_looked = text.contains("nobody has looked");
    assert_eq!(
        claims_nobody_has_looked,
        awaiting.len() == names.len(),
        "**§2 of the page and the baselines disagree about whether anybody has looked.** {} of {} \
         baselines carry a human approval, and the page {} that nobody has. Whichever is now true, \
         the other has to move — a page claiming a review that did not happen is worse than no page.",
        names.len() - awaiting.len(),
        names.len(),
        if claims_nobody_has_looked {
            "says"
        } else {
            "does not say"
        }
    );
}

#[test]
fn the_validation_index_names_this_page() {
    // A page nobody can find is a page nobody reads, and this series is navigated from its index
    // rather than from a directory listing. `mjx-reference-pack` holds its own page to the same
    // rule for the same reason.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/validation/01-index.md");
    let index = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    assert!(
        index.contains("08-the-fidelity-oracle.md"),
        "`docs/validation/01-index.md` lists the pages of this series and does not list this one"
    );
}

#[test]
fn the_page_states_the_refusals_rather_than_only_the_commands() {
    let text = page();
    // The four sentences the whole crate is built around. A page that gave the commands and not the
    // reasons would teach a reader to run `regenerate` whenever the build went red, which is the
    // one thing that must never become a habit.
    for claim in [
        "always matches the code under test",
        "fails**, rather than passing",
        "removes the approval it overwrites",
        "the bytes, not the name",
        "not a human review",
    ] {
        assert!(
            text.contains(claim),
            "the page does not say `{claim}`, which is one of the reasons and not one of the steps"
        );
    }
}
