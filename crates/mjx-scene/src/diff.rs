//! What changed between two frames, record by record.
//!
//! # Why this is a byte comparison and not a semantic one
//!
//! The question a repaint asks is *what do I have to send to the GPU again*, and the answer is a
//! list of records whose **bytes** differ — because bytes are what a buffer holds. A comparison that
//! decoded each record and compared the values would answer the same question more slowly and would
//! be wrong in one direction: two records that decode alike but encode differently would be reported
//! identical while the buffer still needed the new bytes.
//!
//! It is also why [`crate::geometry::finite`] exists. A `NaN` is not equal to itself, so a display
//! list that stored one would report the record changed on every frame for ever; normalising it out
//! at the boundary is what makes "unchanged" a stable answer.
//!
//! # The trap this module is written against
//!
//! *A diff test that compares a frame to itself always reports zero changes.* That green says
//! nothing at all — it is satisfied by a function that returns an empty set unconditionally. So
//! `tests/the_frame_diff_names_what_changed.rs` builds two scenes that differ in exactly one
//! transform, asserts the change set names **that record and no other**, then makes a second,
//! unrelated edit and asserts the set **grows** by exactly the record that edit touched.

use crate::encoding::SectionKind;
use crate::list::DisplayList;

/// What happened to one record.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecordChangeKind {
    /// It is new: the previous frame's table was shorter.
    Added,
    /// It is gone: the next frame's table is shorter.
    Removed,
    /// It is in both frames and its bytes differ.
    Changed,
}

/// One record that is not the same in both frames.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RecordChange {
    /// Which table.
    pub section: SectionKind,
    /// Which record of it — a resource index for a fixed-stride table, and a position in the
    /// stream for [`SectionKind::Commands`].
    pub index: u32,
    /// What happened to it.
    pub kind: RecordChangeKind,
}

/// Everything that differs between two frames of the same page.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FrameDiff {
    header_changed: bool,
    changes: Vec<RecordChange>,
}

impl FrameDiff {
    /// Whether the two frames are identical: same header, same every record.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.header_changed && self.changes.is_empty()
    }

    /// Whether the device scale or the page size differ, which invalidates every glyph in the list
    /// and means a repaint has to upload all of it rather than a delta.
    #[must_use]
    pub fn header_changed(&self) -> bool {
        self.header_changed
    }

    /// The records that differ, grouped by section in wire order and ascending by index within
    /// each — which is the order a buffer is written in, so a consumer never seeks backwards.
    #[must_use]
    pub fn changes(&self) -> &[RecordChange] {
        &self.changes
    }

    /// How many records differ.
    #[must_use]
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// The changes in one section, for a caller that uploads one buffer at a time.
    pub fn changes_in(&self, section: SectionKind) -> impl Iterator<Item = &RecordChange> {
        self.changes
            .iter()
            .filter(move |change| change.section == section)
    }
}

/// What has to be re-uploaded to turn `previous` into `next`.
///
/// Both lists are assumed to be of the same page; comparing two different pages is legal and simply
/// reports most of both.
#[must_use]
pub fn diff_frames(previous: &DisplayList, next: &DisplayList) -> FrameDiff {
    let mut diff = FrameDiff {
        header_changed: previous.device_scale().pixels_per_point()
            != next.device_scale().pixels_per_point()
            || previous.page_size() != next.page_size(),
        changes: Vec::new(),
    };
    for section in SectionKind::ALL {
        match section {
            SectionKind::Commands => compare_commands(previous, next, &mut diff),
            // Path data is addressed by byte range from a geometry, so a change to it always shows
            // up as a changed geometry record; reporting the bytes separately would name a range
            // no consumer can upload on its own.
            SectionKind::PathData => {}
            _ => compare_records(section, previous, next, &mut diff),
        }
    }
    diff
}

/// Compare one fixed-stride table, record by record.
fn compare_records(
    section: SectionKind,
    previous: &DisplayList,
    next: &DisplayList,
    diff: &mut FrameDiff,
) {
    let Some(stride) = section.stride() else {
        return;
    };
    let before = previous.section_bytes(section);
    let after = next.section_bytes(section);
    let before_count = before.len() / stride.max(1);
    let after_count = after.len() / stride.max(1);
    for index in 0..before_count.max(after_count) {
        let at = index * stride;
        let old = before.get(at..at + stride);
        let new = after.get(at..at + stride);
        let kind = match (old, new) {
            (Some(old), Some(new)) if old == new => continue,
            (Some(_), Some(_)) => RecordChangeKind::Changed,
            (None, Some(_)) => RecordChangeKind::Added,
            (Some(_), None) => RecordChangeKind::Removed,
            (None, None) => continue,
        };
        diff.changes.push(RecordChange {
            section,
            // A table cannot hold more records than an index can name, so the cast is exact.
            index: u32::try_from(index).unwrap_or(u32::MAX),
            kind,
        });
    }
}

/// Compare the command streams, position by position.
///
/// Positional rather than a longest-common-subsequence: a display list is rebuilt in the same walk
/// order every frame, so a page whose third shape changed colour produces the same commands in the
/// same positions with one word different. An edit that *inserts* a shape shifts everything after it
/// and is reported as changed from there on, which is the honest answer — the buffer really does
/// have to be rewritten from there.
fn compare_commands(previous: &DisplayList, next: &DisplayList, diff: &mut FrameDiff) {
    let mut before = previous.commands();
    let mut after = next.commands();
    let mut index = 0_u32;
    loop {
        let kind = match (before.next(), after.next()) {
            (None, None) => break,
            (Some(old), Some(new)) if old == new => {
                index = index.saturating_add(1);
                continue;
            }
            (Some(_), Some(_)) => RecordChangeKind::Changed,
            (None, Some(_)) => RecordChangeKind::Added,
            (Some(_), None) => RecordChangeKind::Removed,
        };
        diff.changes.push(RecordChange {
            section: SectionKind::Commands,
            index,
            kind,
        });
        index = index.saturating_add(1);
    }
}
