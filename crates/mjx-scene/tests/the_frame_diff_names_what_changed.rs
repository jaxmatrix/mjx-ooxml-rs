//! The frame diff is asserted on frames that genuinely differ, and on the exact record that does.
//!
//! # The trap
//!
//! **A diff test that compares a frame to itself always reports zero changes.** That green is
//! satisfied by a `diff_frames` that returns an empty set unconditionally, so it says nothing about
//! whether the function works — it says only that it is consistent about doing nothing.
//!
//! So the shape here is: build two scenes that differ in **exactly one** record, assert the change
//! set names **that record and no other**, then make a second, unrelated edit and assert the set
//! **grows by exactly the record that edit touched**. A function that reported everything would fail
//! the first half; one that reported nothing would fail both.
//!
//! # Proved by mutation
//!
//! * Making `compare_records` compare only the *lengths* of two tables →
//!   [`one_changed_transform_names_one_changed_transform`] fails, because the tables are the same
//!   length and only the bytes differ.
//! * Making it report every record rather than the ones that differ →
//!   [`the_change_set_names_that_record_and_no_other`] fails on the untouched paint.
//! * Removing the header comparison → [`a_change_of_zoom_changes_the_header`] fails.

use mjx_scene::{
    diff_frames, Color, Command, DisplayList, Geometry, Paint, RecordChange, RecordChangeKind,
    SceneBuilder, SceneRect, SceneTransform, SectionKind,
};
use mjx_text::DeviceScale;

const INK: Color = Color {
    red: 0x11,
    green: 0x22,
    blue: 0x33,
    alpha: 0xff,
};

const OTHER_INK: Color = Color {
    red: 0xee,
    green: 0xdd,
    blue: 0xcc,
    alpha: 0xff,
};

/// Two rectangles inside one transform: the smallest scene with something to change in three
/// different tables without changing the shape of the command stream.
fn a_page(scale_x: f32, first: Color, second: Color) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::from_pixels_per_point(2.0), 320.0, 240.0);
    let transform = builder
        .add_transform(SceneTransform {
            scale_x,
            ..SceneTransform::IDENTITY
        })
        .expect("a transform");
    let left = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 10.0, 10.0)))
        .expect("a rectangle");
    let right = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(20.0, 0.0, 30.0, 10.0)))
        .expect("another rectangle");
    let first_paint = builder.add_paint(Paint::Solid(first)).expect("a paint");
    let second_paint = builder.add_paint(Paint::Solid(second)).expect("a paint");
    builder
        .push(Command::PushTransform(transform))
        .expect("a transform push");
    builder
        .push(Command::FillPath {
            geometry: left,
            paint: first_paint,
        })
        .expect("a fill");
    builder
        .push(Command::FillPath {
            geometry: right,
            paint: second_paint,
        })
        .expect("a fill");
    builder.push(Command::Pop).expect("a pop");
    builder.finish().expect("the scene is well formed")
}

/// The changes, as `(section, index, kind)` triples, so that a failure prints something a reader can
/// compare against the scene above.
fn summary(changes: &[RecordChange]) -> Vec<(SectionKind, u32, RecordChangeKind)> {
    changes
        .iter()
        .map(|change| (change.section, change.index, change.kind))
        .collect()
}

#[test]
fn one_changed_transform_names_one_changed_transform() {
    let before = a_page(1.0, INK, OTHER_INK);
    let after = a_page(2.0, INK, OTHER_INK);
    assert_ne!(
        before.as_bytes(),
        after.as_bytes(),
        "the two frames are byte-identical, so this test would pass on a diff that did nothing"
    );

    let diff = diff_frames(&before, &after);
    assert!(!diff.header_changed(), "the zoom did not change");
    assert_eq!(
        summary(diff.changes()),
        vec![(SectionKind::Transforms, 0, RecordChangeKind::Changed)],
        "a scene differing only in its transform must name the transform and nothing else"
    );
    assert_eq!(diff.len(), 1);
    assert!(!diff.is_empty());
}

#[test]
fn the_change_set_names_that_record_and_no_other() {
    let before = a_page(1.0, INK, OTHER_INK);
    let after = a_page(
        1.0,
        INK,
        Color {
            red: 0x01,
            green: 0x02,
            blue: 0x03,
            alpha: 0xff,
        },
    );
    let diff = diff_frames(&before, &after);
    assert_eq!(
        summary(diff.changes()),
        vec![(SectionKind::Paints, 1, RecordChangeKind::Changed)],
        "only the *second* paint changed; the first, the geometries, the transform and every \
         command are untouched and must not be named"
    );
    for untouched in [
        SectionKind::Commands,
        SectionKind::Transforms,
        SectionKind::Geometries,
    ] {
        assert_eq!(
            diff.changes_in(untouched).count(),
            0,
            "the `{untouched}` section was named by a diff that did not change it"
        );
    }
}

#[test]
fn an_unrelated_second_edit_makes_the_change_set_grow_by_exactly_that_record() {
    let before = a_page(1.0, INK, OTHER_INK);

    // One edit: the transform.
    let one_edit = diff_frames(&before, &a_page(2.0, INK, OTHER_INK));
    assert_eq!(
        summary(one_edit.changes()),
        vec![(SectionKind::Transforms, 0, RecordChangeKind::Changed)]
    );

    // Two edits: the transform *and* an unrelated paint. The set is the first set plus one entry.
    let two_edits = diff_frames(
        &before,
        &a_page(
            2.0,
            Color {
                red: 0x09,
                green: 0x08,
                blue: 0x07,
                alpha: 0xff,
            },
            OTHER_INK,
        ),
    );
    assert_eq!(
        summary(two_edits.changes()),
        vec![
            (SectionKind::Transforms, 0, RecordChangeKind::Changed),
            (SectionKind::Paints, 0, RecordChangeKind::Changed),
        ],
        "an unrelated edit must add its own record to the set and change nothing about the first"
    );
    assert_eq!(two_edits.len(), one_edit.len() + 1);
}

#[test]
fn a_record_that_only_one_frame_has_is_added_or_removed() {
    let before = a_page(1.0, INK, OTHER_INK);

    let mut builder = SceneBuilder::new(DeviceScale::from_pixels_per_point(2.0), 320.0, 240.0);
    let transform = builder
        .add_transform(SceneTransform {
            scale_x: 1.0,
            ..SceneTransform::IDENTITY
        })
        .expect("a transform");
    let left = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 10.0, 10.0)))
        .expect("a rectangle");
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    builder
        .push(Command::PushTransform(transform))
        .expect("a push");
    builder
        .push(Command::FillPath {
            geometry: left,
            paint,
        })
        .expect("a fill");
    builder.push(Command::Pop).expect("a pop");
    let after = builder.finish().expect("well formed");

    let diff = diff_frames(&before, &after);
    let changes = summary(diff.changes());
    assert!(
        changes.contains(&(SectionKind::Commands, 2, RecordChangeKind::Changed)),
        "the third command was a second fill and is now the pop: {changes:?}"
    );
    assert!(
        changes.contains(&(SectionKind::Commands, 3, RecordChangeKind::Removed)),
        "the fourth command is gone: {changes:?}"
    );
    assert!(
        changes.contains(&(SectionKind::Geometries, 1, RecordChangeKind::Removed)),
        "the second rectangle is gone: {changes:?}"
    );
    assert!(
        changes.contains(&(SectionKind::Paints, 1, RecordChangeKind::Removed)),
        "the second paint is gone: {changes:?}"
    );

    // And the reverse says `Added` where this one says `Removed`, so the direction is real.
    let reversed = summary(diff_frames(&after, &before).changes());
    assert!(reversed.contains(&(SectionKind::Paints, 1, RecordChangeKind::Added)));
    assert!(reversed.contains(&(SectionKind::Commands, 3, RecordChangeKind::Added)));
}

#[test]
fn a_change_of_zoom_changes_the_header() {
    let mut builder = SceneBuilder::new(DeviceScale::from_pixels_per_point(4.0), 320.0, 240.0);
    let geometry = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 10.0, 10.0)))
        .expect("a rectangle");
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    let zoomed = builder.finish().expect("well formed");

    let diff = diff_frames(&a_page(1.0, INK, OTHER_INK), &zoomed);
    assert!(
        diff.header_changed(),
        "a frame at a different device scale carries glyphs rasterised for different buckets, so \
         the whole list has to be re-uploaded and the diff has to say so"
    );
    assert!(!diff.is_empty());
}

#[test]
fn a_frame_compared_with_itself_reports_nothing() {
    // Kept, but kept *last* and kept small, because on its own it proves nothing: it is the green a
    // `diff_frames` that always returned an empty set would also give. What it does establish is
    // that the diff has no false positives, which the tests above cannot show.
    let page = a_page(1.0, INK, OTHER_INK);
    let diff = diff_frames(&page, &page);
    assert!(diff.is_empty());
    assert_eq!(diff.len(), 0);
    assert!(!diff.header_changed());

    // Two separately built but identical frames, too — so the encoding is deterministic and the
    // "nothing changed" answer is not an artefact of comparing one allocation with itself.
    let rebuilt = a_page(1.0, INK, OTHER_INK);
    assert_eq!(page.as_bytes(), rebuilt.as_bytes());
    assert!(diff_frames(&page, &rebuilt).is_empty());
}
