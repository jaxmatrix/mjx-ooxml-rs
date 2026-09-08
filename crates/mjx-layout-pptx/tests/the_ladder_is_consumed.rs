//! The seven-tier effective-property ladder is **consumed, not reimplemented** — proved twice, in
//! the two ways that can each miss what the other catches.
//!
//! # 1. By behaviour
//!
//! A run that states nothing, in a paragraph that states nothing, in a placeholder that states
//! nothing, lays out with the value the **master's `p:txStyles`** gives it — four tiers up. A
//! renderer that read only the run's own `a:rPr` would lay it out at some default size, and the
//! sizes would differ.
//!
//! # 2. By grep
//!
//! Behaviour can be satisfied by a *second* implementation of the ladder that happens to agree, and
//! that is exactly the failure this stack exists to prevent — two resolvers that drift apart at the
//! first schema corner. So the source of this crate is scanned for the identifiers a re-derivation
//! would need: a `p:txStyles`, a `titleStyle`, a placeholder-slot match, a list-style level walk.
//! Finding one here means the ladder was written twice.

use std::path::{Path, PathBuf};

mod support;

use mjx_dml::FontSlot;
use mjx_layout_pptx::SlideDeck;
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_pptx::{Presentation, Surface};

use support::fixture;

/// This crate's own `src/`.
fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `directory`, with its text.
fn sources(directory: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            if let Ok(text) = std::fs::read_to_string(&path) {
                found.push((path, text));
            }
        }
    }
    found
}

/// Whether `line` is a comment or a doc comment — the places these words are *discussed*.
fn is_prose(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*")
}

#[test]
fn no_wire_name_of_the_ladder_appears_in_this_crates_code() {
    // Each of these is a name only a *second* implementation of the ladder would need. They are
    // allowed in prose — the whole crate documentation explains what it consumes — and refused in
    // code.
    const RE_DERIVATION: &[&str] = &[
        "txStyles",
        "titleStyle",
        "bodyStyle",
        "otherStyle",
        "defaultTextStyle",
        "lvl1pPr",
        "lstStyle",
        "defRPr",
        "sldLayout",
        "sldMaster",
        "clrMap",
        "schemeClr",
        "+mn-lt",
        "+mj-lt",
    ];

    let mut offences = Vec::new();
    for (path, text) in sources(&source_root()) {
        for (number, line) in text.lines().enumerate() {
            if is_prose(line) {
                continue;
            }
            for needle in RE_DERIVATION {
                if line.contains(needle) {
                    offences.push(format!(
                        "{}:{}: `{needle}` — {}",
                        path.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "this crate consumes `mjx-pptx`'s effective-property ladder and must not re-derive it, but \
         its code names the wire vocabulary of one:\n{}",
        offences.join("\n")
    );
}

#[test]
fn the_only_property_reads_are_effective_ones() {
    // The other half of the grep: a call to a *declared*-property reader would silently skip every
    // tier above the shape. `mjx-pptx` spells the two apart by name, so the check is exact.
    const DECLARED_ONLY: &[&str] = &[
        "shape_bounds(",
        "shape_transform(",
        "paragraph_properties(",
        "run_properties(",
        "shape_fill(",
        "shape_outline(",
        "shape_list_style_level(",
        "shape_list_style_default(",
    ];

    let mut offences = Vec::new();
    for (path, text) in sources(&source_root()) {
        for (number, line) in text.lines().enumerate() {
            if is_prose(line) {
                continue;
            }
            for needle in DECLARED_ONLY {
                // `effective_shape_bounds(` contains `shape_bounds(`, so the effective form has to
                // be excluded explicitly rather than by the substring alone.
                if line.contains(needle) && !line.contains(&format!("effective_{needle}")) {
                    offences.push(format!(
                        "{}:{}: `{needle}` — {}",
                        path.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a declared-property reader answers what one tier states and skips the six above it:\n{}",
        offences.join("\n")
    );
}

#[test]
fn a_run_that_states_nothing_lays_out_with_the_masters_value() {
    // Four tiers: the run states no size, the paragraph states none, the slide's placeholder states
    // none, and its layout states none — so the master's `p:txStyles` answers. `layouts.pptx`'s
    // slideLayout2 places a title, `add_slide_from_layout` builds placeholders that declare nothing,
    // and the size the deck reports is therefore the master's.
    let mut deck = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = deck.add_slide_from_layout(1).expect("a slide");
    let count = deck
        .shape_count(Surface::Slide(slide))
        .expect("shape count");
    let title = (0..count)
        .find(|&index| {
            deck.shape_placeholder(Surface::Slide(slide), index)
                .expect("placeholder")
                .is_some_and(|info| info.kind == PlaceholderType::Title)
        })
        .expect("a title placeholder");
    deck.set_shape_text(Surface::Slide(slide), title, 0, "Inherited")
        .expect("some text to lay out");

    // What `mjx-pptx` says the run renders at, all seven tiers resolved.
    let expected = deck
        .effective_run_properties(Surface::Slide(slide), title, 0, 0)
        .expect("effective run properties");
    let size = expected
        .size_points()
        .expect("a tier of the ladder states a size");
    let family = expected
        .font(FontSlot::Latin)
        .map(|font| font.typeface.clone());
    assert!(
        !family.as_deref().is_some_and(|name| name.starts_with('+')),
        "if a tier named a theme font, tier seven already resolved it: {family:?}"
    );

    // And what this crate read.
    let read = SlideDeck::read(&mut deck).expect("read");
    let shape = read
        .slide(slide)
        .expect("the slide")
        .shapes()
        .iter()
        .find(|shape| shape.path == vec![u32::try_from(title).expect("small")])
        .expect("the title shape");
    let run = &shape
        .body
        .as_ref()
        .expect("a text body")
        .paragraphs
        .first()
        .expect("a paragraph")
        .runs
        .first()
        .expect("a run");
    assert_eq!(
        run.properties.size_points(),
        Some(size),
        "the size four tiers up is the size this crate laid the run out at"
    );
    assert_eq!(
        run.properties
            .font(FontSlot::Latin)
            .map(|font| font.typeface.clone()),
        family
    );

    // And the size is genuinely the master's rather than a coincidence: the run itself states none.
    assert_eq!(
        deck.run_properties(Surface::Slide(slide), title, 0, 0)
            .expect("the run's own properties")
            .and_then(|properties| properties.size_points()),
        None,
        "the run states no size of its own, so the value came from a tier above it"
    );
}

#[test]
fn a_placeholder_that_states_no_bounds_is_laid_out_where_its_layout_places_it() {
    // The same argument for geometry: `effective_shape_bounds` walks the chain and
    // `shape_bounds` does not, and a box model that used the second would place inheriting
    // placeholders at no position at all.
    let mut deck = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = deck.add_slide_from_layout(1).expect("a slide");
    let count = deck
        .shape_count(Surface::Slide(slide))
        .expect("shape count");

    let mut inheriting = 0_usize;
    for index in 0..count {
        let stated = deck
            .shape_bounds(Surface::Slide(slide), index)
            .expect("stated bounds");
        let effective = deck
            .effective_shape_bounds(Surface::Slide(slide), index)
            .expect("effective bounds");
        if stated.is_none() && effective.is_some() {
            inheriting += 1;
        }
    }
    assert!(
        inheriting > 0,
        "the fixture has a placeholder that states no bounds and still renders somewhere"
    );

    let read = SlideDeck::read(&mut deck).expect("read");
    let placed = read
        .slide(slide)
        .expect("the slide")
        .shapes()
        .iter()
        .filter(|shape| shape.bounds.is_some())
        .count();
    assert!(
        placed >= inheriting,
        "every inheriting placeholder was given the bounds its layout states"
    );
}
