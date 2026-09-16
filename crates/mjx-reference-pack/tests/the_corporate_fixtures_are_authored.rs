//! **The three corporate fixtures are authored here, by this workspace's own writers** (MJXOFF-300).
//!
//! `the_corporate_fixtures_are_corporate.rs` asserts what the committed files *carry*. This file is
//! where they come from, and it holds the committed bytes to these writers: run it and the fixtures
//! are re-derived; they must come out identical, or the fixture and the code that claims to author
//! it have parted company.
//!
//! ```sh
//! MJX_AUTHOR_FIXTURES=1 cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_authored
//! ```
//!
//! # Two tiers, and why the second one exists
//!
//! Everything a typed writer can author is authored by one: the master's logo and accent band, the
//! title's coloured runs, the cropped and masked pictures, the custom geometry, the chart, the
//! SmartArt frame, the styled table, the alpha overlay, the Wingdings bullets, the workbook's table
//! style, its conditional formats, its borders and column widths, its chart and picture, the
//! document's styles, numbering, table, inline picture, footnote, equation and page border.
//!
//! The rest is **markup spliced into a part**, and every one of them is a feature this workspace has
//! a reader for and no writer:
//!
//! * a slide background (`p:bg`), a soft line break (`a:br`), a field (`a:fld`), a picture crop
//!   (`a:srcRect`), a connector (`p:cxnSp`) and a diagram's cached drawing (`dsp:drawing`);
//! * a worksheet's frozen pane and `sheetViews`, a custom `numFmt`, a rich-text cell, an `xdr:sp`
//!   text box and an `x:alignment` inside an `x:xf` (the `applyAlignment` flag beside it *is*
//!   typed, and is written by the typed writer);
//! * Word's tracked changes (`w:ins`/`w:del`), a run's own decorations (`w:u` / `w:strike` /
//!   `w:highlight`), an anchored picture (`wp:anchor`), a text box (`w:txbxContent`), a header's
//!   content and a theme part.
//!
//! That list is not a shortcut: it is the same list the three format crates' own suites splice by
//! hand for the same reason, and each entry is a writer somebody may add later. [`splice`] refuses a
//! splice whose anchor is missing, so a writer that starts emitting one of these — and quietly
//! changes the markup this file anchors on — fails here rather than producing a fixture with an
//! element silently absent.

use mjx_opc::{Package, PartName, Relationship, TargetMode};

// Reached by path rather than declared as siblings: every `tests/*.rs` is its own Cargo target, and
// these three are modules of this one.
#[path = "corporate/deck.rs"]
mod deck;
#[path = "corporate/document.rs"]
mod document;
#[path = "corporate/workbook.rs"]
mod workbook;

/// A valid one-pixel PNG. Both pictures on a surface share it, so the package holds one media part.
pub const LOGO_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// What authors one fixture: bytes out, nothing in.
type Author = fn() -> Vec<u8>;

/// The three fixtures RC03 commits, and the writer that authors each.
const CORPORATE: [(&str, Author); 3] = [
    ("corporate.pptx", deck::corporate_deck),
    ("corporate.xlsx", workbook::corporate_workbook),
    ("corporate.docx", document::corporate_document),
];

/// Replaces `anchor` once, panicking when it is absent.
///
/// A splice that silently found nothing is a fixture missing the element it was spliced for, and
/// every probe in `the_corporate_fixtures_are_corporate.rs` would then fail somewhere far from the
/// cause. This is where it fails instead.
pub fn splice(text: &mut String, anchor: &str, replacement: &str) {
    let found = text.find(anchor).unwrap_or_else(|| {
        panic!(
            "the anchor `{anchor}` is not in this part, so the markup it carries would be absent \
             from the fixture. A writer that started emitting this element, or that changed the \
             markup around it, lands here."
        )
    });
    text.replace_range(found..found + anchor.len(), replacement);
}

/// Reads one part as text.
pub fn part_text(package: &Package, part: &str) -> String {
    let name = PartName::new(part).expect("a part name");
    let bytes = package
        .part_payload(&name)
        .unwrap_or_else(|| panic!("`{part}` is not in the package"));
    String::from_utf8(bytes.into_owned()).expect("the part is UTF-8")
}

/// Writes one part back as text.
pub fn set_part_text(package: &mut Package, part: &str, text: String) {
    let name = PartName::new(part).expect("a part name");
    package
        .replace_part_bytes(&name, text.into_bytes())
        .unwrap_or_else(|error| panic!("replacing `{part}`: {error}"));
}

/// Adds one part and relates it to `source`, which must sit in the same directory.
///
/// The relationship target is the part's own file name, which is what a same-directory target is;
/// a part elsewhere is related with [`relate`] and a spelled-out target instead.
pub fn add_related_part(
    package: &mut Package,
    source: &str,
    part: &str,
    content_type: &str,
    rel_type: &str,
    rel_id: &str,
    bytes: Vec<u8>,
) {
    let name = PartName::new(part).expect("a part name");
    package
        .insert_part(&name, content_type, bytes)
        .unwrap_or_else(|error| panic!("inserting `{part}`: {error}"));
    let target = part
        .rsplit('/')
        .next()
        .expect("a part name has a last segment");
    relate(package, source, rel_type, rel_id, target);
}

/// Adds one relationship from `source`.
pub fn relate(package: &mut Package, source: &str, rel_type: &str, rel_id: &str, target: &str) {
    let from = PartName::new(source).expect("a part name");
    package
        .add_relationship(
            Some(&from),
            Relationship {
                id: rel_id.to_owned(),
                rel_type: rel_type.to_owned(),
                target: target.to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .unwrap_or_else(|error| panic!("relating `{target}` from `{source}`: {error}"));
}

/// Whether `MJX_AUTHOR_FIXTURES` asks for the fixtures to be rewritten.
///
/// **Only `1`.** The variable was read with `is_some()` until RC03's audit, which made
/// `MJX_AUTHOR_FIXTURES=0` rewrite the fixtures and then compare them against themselves — a run in
/// which the byte-identity check below passes unconditionally and proves nothing. Every other value,
/// the empty string and `0` included, leaves the gate asserting.
pub fn authoring_is_requested(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| value == std::ffi::OsStr::new("1"))
}

/// **The committed fixtures are what these writers produce.**
///
/// With `MJX_AUTHOR_FIXTURES=1` this writes them instead, which is how they are (re)generated.
#[test]
fn the_committed_corporate_fixtures_are_what_the_writers_produce() {
    let authoring = authoring_is_requested(std::env::var_os("MJX_AUTHOR_FIXTURES").as_deref());
    for (name, author) in CORPORATE {
        let authored = author();
        assert!(
            authored.len() > 4_000,
            "`{name}` authored {} bytes, which is not a corporate-shaped document",
            authored.len()
        );
        let path = mjx_fixtures::fixtures_dir().join(name);
        if authoring {
            std::fs::write(&path, &authored).expect("the fixture is writable");
            continue;
        }
        let committed = std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "{}: {error}. Run this suite with MJX_AUTHOR_FIXTURES=1 to author it.",
                path.display()
            )
        });
        assert_eq!(
            committed.len(),
            authored.len(),
            "`{name}` is {} committed bytes and {} authored ones: the committed fixture is not what \
             this suite's writers produce today",
            committed.len(),
            authored.len()
        );
        assert!(
            committed == authored,
            "`{name}` is committed with different bytes than these writers produce"
        );
    }
}

/// **Only `1` turns the authoring on**, so no other value can disable the gate above.
///
/// Without this, `MJX_AUTHOR_FIXTURES=0` — which reads as *off* to every reader and every shell —
/// rewrote the three fixtures and compared them against what had just been written.
#[test]
fn every_value_but_one_leaves_the_gate_asserting() {
    use std::ffi::OsStr;
    assert!(
        authoring_is_requested(Some(OsStr::new("1"))),
        "`MJX_AUTHOR_FIXTURES=1` is how the fixtures are regenerated"
    );
    for refused in ["0", "", "true", "yes", "on", "11", " 1"] {
        assert!(
            !authoring_is_requested(Some(OsStr::new(refused))),
            "`MJX_AUTHOR_FIXTURES={refused:?}` turned the authoring on, which rewrites the \
             committed fixtures and makes the byte-identity gate compare them against themselves"
        );
    }
    assert!(
        !authoring_is_requested(None),
        "an unset variable is not a request to rewrite the corpus"
    );
}

/// The authored packages open, and each holds more parts than a near-empty container would.
#[test]
fn every_authored_fixture_is_a_package_with_real_parts() {
    for (name, author) in CORPORATE {
        let bytes = author();
        let package = Package::open(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            package.entries().len() >= 8,
            "`{name}` authored {} part(s)",
            package.entries().len()
        );
    }
}
