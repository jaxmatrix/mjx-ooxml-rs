//! The addressing scheme this crate issues is the one `mjx-session` already wrote down.
//!
//! # Why this is a gate and not a comment
//!
//! A [`SourceRef`](mjx_layout::SourceRef) means nothing without the box model that issued it, so two
//! crates that both address a presentation must agree or every consumer above them has to know
//! which one produced the value it is holding. `mjx-session` (MJXOFF-167) wrote the scheme first,
//! because editing needed it before any box model existed; this crate adopts it exactly.
//!
//! # Why the gate reads a *file* rather than calling the other crate
//!
//! `crates/mjx-layout-pptx/tests/the_seam_holds.rs` forbids `mjx-session` in either dependency
//! section — not because the edge would be illegal (3.5 is below 3.6, so it would be a legal
//! downward edge) but because `mjx-session`'s default features name all three format crates, and a
//! box model for `.pptx` that dragged `.docx` and `.xlsx` into its own test build would make the
//! seam a comment.
//!
//! So the agreement is checked the only way it can be from here: the constants
//! `PresentationSession` declares are read out of its source, and compared against this crate's.
//! If either side renumbers, this fails and names both files.

mod support;

use std::path::{Path, PathBuf};

use mjx_layout::{PartId, SourcePath};
use mjx_layout_pptx::{address, TextHit};
use mjx_pptx::Surface;

/// `mjx-session`'s presentation residency, as text.
fn session_source() -> String {
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-session/src/ooxml/presentation.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

#[test]
fn the_four_part_numbers_are_the_same_on_both_sides() {
    let source = session_source();
    for (name, ours) in [
        ("SLIDES", address::SLIDES),
        ("LAYOUTS", address::LAYOUTS),
        ("MASTERS", address::MASTERS),
        ("NOTES", address::NOTES),
    ] {
        let declaration = format!("pub const {name}: PartId = PartId::new({});", ours.number());
        assert!(
            source.contains(&declaration),
            "`mjx-session` does not declare `{declaration}`. The two crates address a presentation \
             the same way or every consumer above them has to know which one issued the value it \
             is holding; if the scheme has moved, move both.",
        );
    }
}

#[test]
fn the_surface_kinds_map_the_way_the_session_reads_them() {
    let source = session_source();
    // The session's own `surface` reader, spelled out. A renumbering there and not here would send
    // an edit to the wrong slide with nothing to notice it.
    for (number, arm) in [
        (0, "0 => Ok(Surface::Slide(index))"),
        (1, "1 => Ok(Surface::Layout(index))"),
        (2, "2 => Ok(Surface::Master(index))"),
        (3, "3 => Ok(Surface::Notes(index))"),
    ] {
        assert!(
            source.contains(arm),
            "`mjx-session` no longer reads part {number} as `{arm}`"
        );
    }

    assert_eq!(
        address::surface_of(PartId::new(0), 4),
        Some(Surface::Slide(4))
    );
    assert_eq!(
        address::surface_of(PartId::new(1), 0),
        Some(Surface::Layout(0))
    );
    assert_eq!(
        address::surface_of(PartId::new(2), 1),
        Some(Surface::Master(1))
    );
    assert_eq!(
        address::surface_of(PartId::new(3), 2),
        Some(Surface::Notes(2))
    );
    assert_eq!(address::surface_of(PartId::new(9), 0), None);
}

#[test]
fn the_path_shape_is_the_one_the_session_splits() {
    let source = session_source();
    // The session takes everything after the surface index as the shape path, and the last two
    // segments of a *text* address as the paragraph and the run. Both are asserted, because a
    // scheme that agreed about the first and not the second would send a bounds edit to the right
    // shape and a text edit to the wrong run.
    assert!(
        source.contains("segments[1..].iter().map(|&it| it as usize).collect()"),
        "`mjx-session` no longer reads the shape path as everything after the surface index",
    );
    assert!(
        source.contains("let split = segments.len() - 2;"),
        "`mjx-session` no longer reads the paragraph and run as the last two segments",
    );

    // And this crate builds exactly that.
    assert_eq!(address::shape_path(7, &[2, 5]).segments(), &[7, 2, 5]);
    assert_eq!(
        address::paragraph_path(7, &[2, 5], 3).segments(),
        &[7, 2, 5, 3]
    );
    assert_eq!(
        address::run_path(7, &[2, 5], 3, 1).segments(),
        &[7, 2, 5, 3, 1]
    );
}

#[test]
fn a_grouped_shapes_path_still_leaves_the_paragraph_and_run_last() {
    // The case the two-segment rule exists for: a shape inside two groups has a five-segment path,
    // and the paragraph and run are still the last two. A `TextHit` reconstructs it given the depth
    // the box model recorded — which is the number the path alone cannot carry.
    let path = address::run_path(0, &[1, 4, 2], 6, 0);
    assert_eq!(path.segments(), &[0, 1, 4, 2, 6, 0]);

    let source = mjx_layout::SourceRef::new(address::SLIDES, path, 0..5);
    let hit = TextHit::from_source(&source, 3).expect("a hit");
    assert_eq!(hit.shape, vec![1, 4, 2]);
    assert_eq!(hit.paragraph, Some(6));
    assert_eq!(hit.run, Some(0));
    assert_eq!(hit.offset, Some(0));
}

#[test]
fn a_shape_address_carries_no_paragraph_and_no_run() {
    let source = mjx_layout::SourceRef::node(address::SLIDES, address::shape_path(2, &[3]));
    let hit = TextHit::from_source(&source, 1).expect("a hit");
    assert_eq!(hit.surface_index, 2);
    assert_eq!(hit.shape_indices(), vec![3]);
    assert_eq!(hit.paragraph, None);
    assert_eq!(hit.run, None);
    assert_eq!(hit.offset, None);
}

#[test]
fn a_path_shallower_than_its_shape_is_refused_rather_than_truncated() {
    // A depth that does not fit the path is a caller error, and answering *some* shape would be
    // worse than answering none: it would name a shape that is not the one under the point.
    let source = mjx_layout::SourceRef::node(address::SLIDES, SourcePath::new(&[0]));
    assert_eq!(TextHit::from_source(&source, 2), None);
    let empty = mjx_layout::SourceRef::node(address::SLIDES, SourcePath::root());
    assert_eq!(TextHit::from_source(&empty, 0), None);
}

#[test]
fn the_notes_master_has_no_part_of_its_own() {
    // `Surface::NotesMaster` is the one surface with no number, and inventing a fifth part for a
    // surface this box model never lays out would put an address in the world that nothing reads.
    assert_eq!(address::part_of(Surface::NotesMaster), None);
    assert_eq!(address::part_of(Surface::Slide(0)), Some(address::SLIDES));
    assert_eq!(address::part_of(Surface::Layout(0)), Some(address::LAYOUTS));
    assert_eq!(address::part_of(Surface::Master(0)), Some(address::MASTERS));
    assert_eq!(address::part_of(Surface::Notes(0)), Some(address::NOTES));
}
