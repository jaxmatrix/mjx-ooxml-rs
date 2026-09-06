//! Index conversion between this crate's `usize` addressing and the `u32` its re-exported read
//! structures carry.
//!
//! `ShapeInfo::index`, `LayoutInfo::index` and `LayoutInfo::master_index` are `u32` because they
//! **leave** this crate: `mjx-ooxml` re-exports all three verbatim and hands them to a caller, and
//! every index the facade hands back is a `u32` so that a caller — and a binding generated over that
//! surface — sees one width on every host rather than a type whose size depends on the target. Both
//! bindings already read these three fields as `u32` and used to cast on the way out; MJXOFF-118
//! moved the width to where the value is built, so the cast happens once here instead of once per
//! binding.
//!
//! Neither conversion can panic and neither is lossy in practice: widening `u32` to `usize` is exact
//! on every target this library builds for, and narrowing saturates at [`u32::MAX`], a count no
//! presentation that fits in memory can reach. `crates/mjx-ooxml/src/index.rs` makes the same two
//! conversions at the facade for the same reason.

/// A model `usize` index or count as the `u32` a re-exported read structure carries.
pub(crate) fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// A `u32` index from a re-exported read structure as the `usize` this crate addresses with.
pub(crate) fn index(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
