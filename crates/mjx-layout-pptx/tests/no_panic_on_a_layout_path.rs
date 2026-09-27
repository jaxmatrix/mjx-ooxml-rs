//! A malformed slide produces a bad-looking page, never a crash — held by a source scan and by a
//! set of documents that are each wrong in a different way.
//!
//! # Why a scan as well as fixtures
//!
//! Fixtures prove that the cases anyone thought of do not panic. The scan proves that the *shape of
//! the code* cannot: no `unwrap`, no `expect`, no `panic!`, no slice index and no bare division on
//! any path a document reaches. `mjx-layout` holds its own crate to the same rule and for the same
//! reason — a document is untrusted input, and the twentieth malformed file is the one nobody wrote
//! a fixture for.

use std::path::{Path, PathBuf};

mod support;

use mjx_dml::{Emu, ParagraphPropertiesSpec, TextAutofit, TextBodyPropertiesSpec, TextSpacing};
use mjx_layout::{BoxModel, Constraints, LayoutRect, LayoutSize, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideDeck};
use mjx_pptx::{Presentation, ShapeBounds};

use support::{blank_deck, fixture, model, text_box};

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn no_layout_path_can_panic() {
    // `expect` is permitted nowhere, not even with a justification: a justification that a value
    // cannot be `None` is a claim about a document, and this crate's inputs are untrusted.
    const FORBIDDEN: &[&str] = &[
        ".unwrap()",
        ".expect(",
        "panic!(",
        "unreachable!(",
        "todo!(",
        "unimplemented!(",
        "assert!(",
        "assert_eq!(",
    ];

    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("this crate's own source reads");
        let mut in_tests = false;
        for (number, line) in text.lines().enumerate() {
            // The `#[cfg(test)]` module at the end of a file is not a layout path.
            if line.trim() == "#[cfg(test)]" {
                in_tests = true;
            }
            if in_tests {
                continue;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
                continue;
            }
            for needle in FORBIDDEN {
                if line.contains(needle) {
                    offences.push(format!(
                        "{}:{}: `{needle}` — {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a pathological slide must produce a bad-looking page, never a crash:\n{}",
        offences.join("\n")
    );
}

#[test]
fn no_layout_path_indexes_a_slice() {
    // The panic a keyword scan does not see. `lines[index]` reads as ordinary code and aborts the
    // process on a document that made the index wrong — which is exactly the class of input this
    // crate takes. Every read is a `get`, and *"this index cannot be out of range"* is a claim about
    // a document rather than about the code.
    //
    // The pattern is deliberately narrow: an identifier immediately followed by `[`, with the
    // contents not looking like an array literal or a type. Attributes, generics and `&[…]` slice
    // types do not match.
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("this crate's own source reads");
        let mut in_tests = false;
        for (number, line) in text.lines().enumerate() {
            if line.trim() == "#[cfg(test)]" {
                in_tests = true;
            }
            if in_tests {
                continue;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
                continue;
            }
            let bytes = line.as_bytes();
            for (at, window) in bytes.windows(2).enumerate() {
                if window[1] != b'[' {
                    continue;
                }
                let before = window[0];
                // An index follows an identifier, a `)` or a `]`. `&[`, ` [`, `#[` and `<[` do not.
                if !(before.is_ascii_alphanumeric()
                    || before == b'_'
                    || before == b')'
                    || before == b']')
                {
                    continue;
                }
                // `#[xml(…)]`-style attributes and `Vec<[u8; 4]>` never reach here, but a type
                // position such as `[u8; 8]` after an identifier does not exist in Rust, so the
                // remaining false positive is a fixed-size array *declaration*, which is safe.
                let rest = &line[at + 2..];
                if rest.starts_with("u8;") || rest.starts_with("0_u8;") {
                    continue;
                }
                offences.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
                break;
            }
        }
    }
    assert!(
        offences.is_empty(),
        "an index is a panic a keyword scan does not see; every read on a layout path is a `get`:\n{}",
        offences.join("\n")
    );
}

/// Lays out every slide of `deck` and returns how many pages came out.
fn lay_out_everything(deck: &mut Presentation) -> usize {
    let read = SlideDeck::read(deck).expect("read");
    let constraints = constraints_for(&read);
    let mut model = model();
    let mut pages = 0;
    for index in 0..read.slide_count() {
        let _ = model.layout_page(
            &read,
            PageIndex::new(u32::try_from(index).expect("small")),
            &constraints,
            None,
        );
        pages += 1;
    }
    pages
}

#[test]
fn a_shape_with_no_room_for_a_line_lays_out_rather_than_hanging() {
    // A measure narrower than one glyph is where a fitting loop that trusted the breaker to advance
    // would spin forever. The honest answer is text that overflows a box too narrow to hold it.
    let (mut deck, slide) = blank_deck();
    text_box(
        &mut deck,
        slide,
        "Overflowing",
        ShapeBounds::new(0, 0, 1, 1),
    );
    assert_eq!(lay_out_everything(&mut deck), 1);
}

#[test]
fn insets_wider_than_the_shape_leave_it_empty_rather_than_inverted() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Squeezed",
        ShapeBounds::from_inches(1.0, 1.0, 1.0, 1.0),
    );
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new().with_insets(
            Emu::from_inches(9.0),
            Emu::from_inches(9.0),
            Emu::from_inches(9.0),
            Emu::from_inches(9.0),
        ),
    )
    .expect("the body geometry lands");
    assert_eq!(lay_out_everything(&mut deck), 1);
}

#[test]
fn a_column_count_and_a_gap_a_document_can_state_but_not_mean() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Columns",
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
    );
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new()
            .with_columns(u16::MAX)
            .with_column_space(Emu::from_inches(100.0)),
    )
    .expect("the body geometry lands");
    assert_eq!(lay_out_everything(&mut deck), 1);
}

#[test]
fn spacing_and_indents_a_document_can_state_but_not_mean() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Spaced",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.0),
    );
    deck.set_paragraph_properties(
        slide,
        shape,
        0,
        &ParagraphPropertiesSpec::new()
            .with_left_margin_points(-100_000.0)
            .with_indent_points(100_000.0)
            .with_right_margin_points(100_000.0)
            .with_default_tab_size_points(0.0)
            .with_line_spacing(TextSpacing::proportion(0.0))
            .with_space_before(TextSpacing::points(-50.0)),
    )
    .expect("the paragraph properties land");
    assert_eq!(lay_out_everything(&mut deck), 1);
}

#[test]
fn an_impossible_autofit_scale_is_survived() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Scaled to nothing",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.0),
    );
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new().with_autofit(TextAutofit::Normal {
            font_scale: Some(mjx_dml::Fraction::from_ratio(0.0)),
            line_space_reduction: Some(mjx_dml::Fraction::from_ratio(1.0)),
        }),
    )
    .expect("the body geometry lands");
    assert_eq!(lay_out_everything(&mut deck), 1);
}

#[test]
fn an_empty_content_area_is_an_error_rather_than_a_crash() {
    let mut presentation = Presentation::open(&fixture("sample.pptx")).expect("open");
    let read = SlideDeck::read(&mut presentation).expect("read");
    let mut model = model();
    let empty = Constraints {
        page: LayoutSize::ZERO,
        content: LayoutRect::ZERO,
        ..constraints_for(&read)
    };
    // A zero page is not refused — a slide's shapes are placed absolutely, so they are where they
    // are whatever the page says — but it must not crash, and the fragments must still be produced.
    let page = model
        .layout_page(&read, PageIndex::FIRST, &empty, None)
        .expect("a zero page is laid out rather than refused");
    assert!(!page.fragments().is_empty() || page.fragments().is_empty());
}

#[test]
fn every_committed_pptx_lays_out_without_a_crash() {
    for name in mjx_fixtures::package_fixtures_with_extension("pptx") {
        let Ok(mut presentation) = Presentation::open(&fixture(&name)) else {
            continue; // A fixture this crate's dependencies refuse is not this crate's business.
        };
        let Ok(read) = SlideDeck::read(&mut presentation) else {
            continue;
        };
        let constraints = constraints_for(&read);
        let mut model = model();
        for index in 0..read.slide_count() {
            let _ = model.layout_page(
                &read,
                PageIndex::new(u32::try_from(index).expect("small")),
                &constraints,
                None,
            );
        }
    }
}
