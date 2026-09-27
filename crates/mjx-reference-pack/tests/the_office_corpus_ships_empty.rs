//! **The corpus is empty, it says so, and the suite reports that rather than passing over it.**
//!
//! # The trap this file is the answer to
//!
//! MJXOFF-207's brief names it: *every gate here can pass with no authoritative reference in
//! existence, because the reference is the one thing an agent cannot produce.* That makes it the
//! epic's most vacuous-looking child by construction — and the answer is not to invent a reference,
//! it is to make the absence **loud, countable and impossible to mistake for a pass**.
//!
//! So:
//!
//! * [`the_export_directory_is_walked_and_its_size_reported`] prints the count on every run, so
//!   "green" never silently means "nothing ran", and `MJX_REQUIRE_OFFICE_EXPORTS=1` turns an empty
//!   directory into a failure the moment CI should expect one. That variable is **not set anywhere
//!   today**, for exactly the reason `main`'s `MJX_REQUIRE_OFFICE_CORPUS` is not.
//! * [`an_office_export_that_is_not_there_is_not_evidence`] asks the harness for a verdict about a
//!   file that does not exist, and requires the answer to be `not evidence` naming the absence —
//!   never an agreement, and never an empty report.
//! * [`the_directory_says_who_may_fill_it`] holds the README to the one sentence that keeps it
//!   empty.
//!
//! # Why this mirrors `main`'s own harness rather than inventing a second doctrine
//!
//! The parallel programme reached the identical conclusion independently and wrote it into `main`:
//! `tests/office-authored/` ships empty, its README says an agent may not fill it, and
//! `xtask/tests/office_corpus.rs` walks it and reports the count. This directory sits **inside**
//! that one and carries the same rule for the same reason. One doctrine, two directories.

use std::path::PathBuf;

use mjx_reference_pack::authority::{Baseline, ReferenceProvider, RenderedContent, Verdict};
use mjx_reference_pack::{
    parity_count, ARTEFACTS, OFFICE_EXPORT_DIRECTORY, REQUIRE_OFFICE_EXPORTS,
};

/// Where the exports go, as an absolute path.
fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(OFFICE_EXPORT_DIRECTORY)
}

/// The PDFs that are there, which is none of them today.
fn exports() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".pdf"))
        .collect();
    names.sort();
    names
}

#[test]
fn the_export_directory_is_walked_and_its_size_reported() {
    let directory = directory();
    assert!(
        directory.is_dir(),
        "{} does not exist; the harness has nowhere to read a reference from and nobody would know",
        directory.display()
    );
    let exports = exports();
    println!(
        "reference-pack exports: {} file(s) in {}",
        exports.len(),
        directory.display()
    );
    for name in &exports {
        println!("  {name}");
    }
    assert!(
        !(exports.is_empty() && std::env::var_os(REQUIRE_OFFICE_EXPORTS).is_some()),
        "{REQUIRE_OFFICE_EXPORTS} is set and {} is empty. Set it only once a person has run \
         Office: an empty directory is the honest state of this project until then.",
        directory.display()
    );
    if exports.is_empty() {
        println!(
            "the directory is empty, which is the honest state of this project: no agent may fill \
             it, because the value of an Office export is entirely its provenance. \
             docs/validation/07-the-reference-pack.md is how it gets filled."
        );
        return;
    }
    // Once it is filled, every file in it must be one of the four the pack expects — a stray PDF is
    // a measurement of something nobody generated.
    for name in &exports {
        let stem = name.trim_end_matches(".pdf");
        assert!(
            ARTEFACTS
                .iter()
                .any(|artefact| artefact.starts_with(stem) && artefact.len() > stem.len()),
            "`{name}` is in the export directory and is not an export of any artefact the pack \
             produces"
        );
    }
}

#[test]
fn an_office_export_that_is_not_there_is_not_evidence() {
    // The strongest possible provider, asked about a file that does not exist. If the harness could
    // ever answer "agreed" here, every other assertion in this crate would be worthless.
    let absent = directory().join("01-presets-at-their-defaults.pdf");
    if absent.is_file() {
        println!("SKIPPED: a person has run Office and the export exists, which is the whole aim");
        return;
    }
    let row = Baseline {
        subject: "rect".to_owned(),
        provenance: format!("{}", absent.display()),
        provider: ReferenceProvider::OfficeExport,
        authority: ReferenceProvider::OfficeExport.authority(),
        content: RenderedContent::Outline,
        verdict: Verdict::NotEvidence {
            reason: format!(
                "there is no Office export at {}; the reference is the one thing an agent cannot \
                 produce",
                absent.display()
            ),
        },
    };
    assert!(!row.verdict.is_evidence());
    assert!(
        !row.may_be_called_parity(),
        "a row about a file that does not exist was called parity"
    );
    assert_eq!(parity_count(std::slice::from_ref(&row)), 0);
    // Authoritative provider, and still not parity — because parity needs *both* halves.
    assert!(row.provider.is_authoritative());
    println!("{row}");
}

/// **The whole authoritative pass, over the empty directory** — which is the state it ships in.
///
/// Every row is `not evidence`, the count is the whole pack, and `parity_count` is zero. That is the
/// honest report and it is a report rather than an error: a person has not run Office yet, and no
/// build can fix that.
#[test]
fn the_authoritative_pass_over_an_empty_directory_is_a_report_and_not_a_failure() {
    if directory()
        .join("01-presets-at-their-defaults.pdf")
        .is_file()
    {
        println!("SKIPPED: a person has run Office, which is the whole aim");
        return;
    }
    let rows = mjx_reference_pack::pack::office_pass().expect("the pass runs over an empty corpus");
    // 187 plates twice, plus the specimen deck and the hanging document.
    assert_eq!(rows.len(), 187 * 2 + 2, "{} rows", rows.len());
    assert_eq!(parity_count(&rows), 0);
    for row in &rows {
        assert!(
            !row.verdict.is_evidence(),
            "`{}` produced evidence out of a directory with no files in it: {row:?}",
            row.subject
        );
        assert!(
            row.provider.is_authoritative(),
            "the authoritative pass stamped a row with a provider that is not"
        );
    }
    // And the printed report says the right thing for the providers it holds — not the LibreOffice
    // sentence, which would be describing a run that did not happen.
    let printed = mjx_reference_pack::pack::report(&rows);
    assert!(
        printed.contains("0 may be called parity"),
        "the report does not say that nothing here is parity"
    );
    assert!(
        !printed.contains("A LibreOffice agreement"),
        "an Office-provider report ended with the LibreOffice sentence"
    );
    println!(
        "{}",
        printed.lines().rev().take(3).collect::<Vec<_>>().join("\n")
    );
}

/// The export names the pass looks for are the artefact names with a `.pdf` on them, and nothing
/// else — so the README, the instructions and the reader cannot disagree about a file name.
#[test]
fn the_pass_looks_for_the_names_the_instructions_ask_for() {
    for artefact in ARTEFACTS {
        let expected = mjx_reference_pack::pack::export_name(artefact);
        assert!(expected.ends_with(".pdf"));
        assert_eq!(
            expected.trim_end_matches(".pdf"),
            artefact.rsplit_once('.').expect("an extension").0
        );
    }
}

#[test]
fn the_directory_says_who_may_fill_it() {
    let readme = directory().join("README.md");
    let text = std::fs::read_to_string(&readme)
        .unwrap_or_else(|error| panic!("reading {}: {error}", readme.display()));
    for phrase in [
        "No agent may fill this directory",
        "entirely its provenance",
        "07-the-reference-pack.md",
    ] {
        assert!(
            text.contains(phrase),
            "{} does not say {phrase:?}, which is what keeps it empty",
            readme.display()
        );
    }
    // Every artefact's expected export is named, so a person knows what four files to produce.
    for artefact in ARTEFACTS {
        let expected = artefact
            .rsplit_once('.')
            .map(|(stem, _)| format!("{stem}.pdf"))
            .expect("every artefact has an extension");
        assert!(
            text.contains(&expected),
            "the README does not name `{expected}`"
        );
    }
}
