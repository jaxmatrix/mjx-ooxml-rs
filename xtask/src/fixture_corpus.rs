//! Facts about the committed fixture corpus that more than one test binary needs.
//!
//! # Why this is a module rather than a helper in a test
//!
//! `xtask/tests/fixture_provenance.rs` (MJXOFF-249) holds a ledger of the fixtures whose
//! `docProps/app.xml` names Microsoft, and it derives that set from the corpus rather than listing
//! it — a fixture committed tomorrow is inside the negative the moment it lands.
//!
//! `xtask/tests/derived_rosters.rs` then needs the *same* set, because the ledger is a literal list
//! of fixture names and its sweep reads it as a roster over the committed corpus. Answering that
//! with a second walk — a second `Package::open`, a second `<Application>` reader — would be two
//! derivations of one fact that could disagree with no way to say which was wrong, which is this
//! repository's standing objection and the reason `facade_surface::handle_types` exists (MJXOFF-252
//! item 1). So the derivation lives here and both binaries call it.
//!
//! # What this module does not do
//!
//! It never concludes anything about who wrote a file. [`names_microsoft`] is a test on a
//! **string**; matching it raises a question that only a person can answer.
//! `fixture_provenance.rs`'s header states the rule in full, and it is the rule this module serves
//! rather than one it restates.

use std::collections::BTreeSet;

use mjx_opc::{Package, PartName};

/// The part every claim here is read out of.
pub const APP_PROPERTIES: &str = "/docProps/app.xml";

/// The needle that raises the question — **and answers none of it**.
///
/// Every application Microsoft ships writes its own name into `Application`, and every one of those
/// names contains this word: `Microsoft Excel`, `Microsoft Office Word`,
/// `Microsoft Macintosh PowerPoint`. So a fixture whose value contains it is a fixture somebody has
/// to have looked at. That is the entire content of the match.
pub const NAMES_MICROSOFT: &str = "microsoft";

/// The `Application` element of one fixture, or `None` when it ships no `docProps/app.xml`.
///
/// Read out of the package rather than out of the zip directly, so the corpus is walked the way
/// every other suite walks it.
///
/// # Panics
/// If the fixture cannot be opened. This corpus is the committed one and every file in it
/// round-trips elsewhere, so an unopenable fixture is a failure rather than a skip.
#[must_use]
pub fn application_of(fixture: &str) -> Option<String> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).unwrap_or_else(|e| panic!("opening {fixture}: {e}"));
    let part = PartName::new(APP_PROPERTIES).expect("a literal part name");
    let payload = package.part_payload(&part)?;
    let text = String::from_utf8_lossy(&payload).into_owned();
    let start = text.find("<Application>")? + "<Application>".len();
    let end = text[start..].find("</Application>")? + start;
    Some(text[start..end].to_owned())
}

/// Whether a value carries [`NAMES_MICROSOFT`], case-insensitively.
#[must_use]
pub fn names_microsoft(application: &str) -> bool {
    application.to_ascii_lowercase().contains(NAMES_MICROSOFT)
}

/// Every committed package fixture whose `Application` element names Microsoft.
///
/// This is a set of *questions*, not a set of Office-authored files: the two members today were
/// written by XlsxWriter and python-pptx. `xtask/tests/fixture_provenance.rs`'s ledger is what
/// carries the answers, and that suite holds this set and the ledger to each other in both
/// directions.
#[must_use]
pub fn fixtures_claiming_microsoft_authorship() -> BTreeSet<String> {
    mjx_fixtures::package_fixtures()
        .into_iter()
        .filter(|fixture| application_of(fixture).is_some_and(|value| names_microsoft(&value)))
        .collect()
}
