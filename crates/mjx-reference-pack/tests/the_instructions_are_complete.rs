//! **The actual deliverable: instructions complete enough that the user needs no further
//! conversation.**
//!
//! MJXOFF-207's own "Done when" says so in those words, which makes the written page the one
//! artefact of this child that cannot be checked by running anything. What *can* be checked is that
//! it has not drifted from the code — and drift is the whole failure mode here, because a page that
//! names a file the generator no longer writes sends a person to a Windows machine with the wrong
//! four files.
//!
//! So this is a divergence gate of exactly the shape `xtask/tests/tokens.rs` is: two artefacts
//! derived from one source, held to each other.
//!
//! * Every artefact the pack produces is named in the page, and every file name the page uses is one
//!   the pack produces.
//! * Every key in [`THE_SITTING`] has a heading in the page, and the page introduces no key that is
//!   not in the list.
//! * The page names the directory the exports go in, the command that generates the pack and the
//!   command that ingests it, spelled the way the code spells them.
//! * The **generated** `INSTRUCTIONS.md` — the one that ships beside the artefacts — carries the same
//!   file names and the same keys, because a person who never opens the repository reads that one.
//!
//! # And the two sentences that must never leave either document
//!
//! [`neither_document_calls_a_libreoffice_run_parity`] greps both for the claim this whole child is
//! written against. It is a crude check and it is deliberately crude: the sentence *"a green
//! LibreOffice comparison is parity"* is exactly the thing an editor would add in good faith while
//! tightening the prose.

use std::path::PathBuf;

use mjx_reference_pack::pack::instructions;
use mjx_reference_pack::{ARTEFACTS, OFFICE_EXPORT_DIRECTORY, THE_SITTING};

/// The committed page a person reads before the day.
fn page() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/validation/07-the-reference-pack.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

#[test]
fn the_page_names_every_artefact_and_invents_none() {
    let page = page();
    for artefact in ARTEFACTS {
        assert!(
            page.contains(artefact),
            "`docs/validation/07-the-reference-pack.md` does not name `{artefact}`, which the pack \
             produces — so a person following it would export the wrong set of files"
        );
    }
    // The other direction: a file name in the page that the pack does not write. A bare extension
    // — the page says *"it is a `.docx` and not a deck"* — is prose about a format rather than a
    // file name, so a candidate has to have a stem.
    for word in page.split(|character: char| {
        !(character.is_alphanumeric() || character == '-' || character == '.' || character == '_')
    }) {
        let Some((stem, extension)) = word.rsplit_once('.') else {
            continue;
        };
        if !matches!(extension, "pptx" | "docx") || stem.is_empty() {
            continue;
        }
        assert!(
            ARTEFACTS.contains(&word),
            "the page names `{word}`, which the pack does not produce"
        );
    }
}

#[test]
fn the_page_has_a_heading_for_every_item_and_invents_none() {
    let page = page();
    for item in THE_SITTING {
        let heading = format!("### `{}` —", item.key);
        assert!(
            page.contains(&heading),
            "the page has no `{heading}` section, so the item `{}` is in the code and not in the \
             instructions",
            item.key
        );
    }
    let headings = page.matches("### `").count();
    assert_eq!(
        headings,
        THE_SITTING.len(),
        "the page has {headings} item headings and the code declares {}",
        THE_SITTING.len()
    );
}

#[test]
fn the_page_names_the_directory_and_the_two_commands() {
    let page = page();
    assert!(
        page.contains(OFFICE_EXPORT_DIRECTORY),
        "the page does not say where the exports go; it must name `{OFFICE_EXPORT_DIRECTORY}`"
    );
    for command in [
        "cargo run -p mjx-reference-pack -- generate",
        "cargo run -p mjx-reference-pack -- preliminary",
    ] {
        assert!(
            page.contains(command),
            "the page does not spell `{command}`, so a reader cannot run it"
        );
    }
    // And it must actually say what not to do, which is half of what makes a sitting repeatable.
    for phrase in [
        "Do not print",
        "Do not edit",
        "Do not open the files in LibreOffice",
    ] {
        assert!(
            page.contains(phrase),
            "the page's list of what not to do is missing {phrase:?}"
        );
    }
}

/// The copy that travels with the artefacts, for the reader who never opens the repository.
#[test]
fn the_generated_instructions_carry_the_same_names_and_keys() {
    let generated = instructions();
    for artefact in ARTEFACTS {
        assert!(
            generated.contains(artefact),
            "the generated INSTRUCTIONS.md does not name `{artefact}`"
        );
    }
    for item in THE_SITTING {
        assert!(
            generated.contains(item.key),
            "the generated INSTRUCTIONS.md does not carry the item `{}`",
            item.key
        );
        assert!(
            generated.contains(item.question),
            "the generated INSTRUCTIONS.md names `{}` and does not say what it asks",
            item.key
        );
    }
    assert!(
        generated.contains(OFFICE_EXPORT_DIRECTORY),
        "the generated INSTRUCTIONS.md does not say where the exports go"
    );
    assert!(
        generated.len() > 3_000,
        "the generated INSTRUCTIONS.md is {} bytes, which is not a set of instructions",
        generated.len()
    );
}

#[test]
fn neither_document_calls_a_libreoffice_run_parity() {
    for (name, text) in [
        ("docs/validation/07-the-reference-pack.md", page()),
        ("the generated INSTRUCTIONS.md", instructions()),
    ] {
        let lower = text.to_lowercase();
        assert!(
            lower.contains("not parity")
                || lower.contains("never mean")
                || lower.contains("never been parity"),
            "{name} does not say anywhere that a LibreOffice run is not parity, which is the one \
             sentence this whole child exists to keep in place"
        );
        for claim in [
            "libreoffice confirms",
            "matches powerpoint",
            "libreoffice proves",
            "verified against libreoffice",
        ] {
            assert!(
                !lower.contains(claim),
                "{name} contains {claim:?}, which records a non-authoritative reference as fidelity"
            );
        }
    }
}

/// The page is in the series' own index, or nobody following that series will find it.
#[test]
fn the_validation_index_names_this_page() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/validation/01-index.md");
    let index = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    assert!(
        index.contains("07-the-reference-pack.md"),
        "`docs/validation/01-index.md` lists the pages of this series and does not list this one"
    );
}
