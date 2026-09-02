//! Index conversion between the facade's `u32` addressing and the model's `usize`.
//!
//! Every index the facade takes or hands back is a `u32`, so a caller — and a binding generated over
//! this surface — sees the same width on every platform, rather than a type whose size depends on the
//! host. The two conversions below are the only place that width difference is crossed.
//!
//! Neither can panic. Widening `u32` to `usize` is lossless on every target this library builds for
//! (32- and 64-bit, plus `wasm32`); on a hypothetical 16-bit target it saturates at [`usize::MAX`],
//! which is an index no document holds, so the call fails with an out-of-range error rather than
//! silently addressing the wrong thing. Narrowing back saturates at [`u32::MAX`] for the same reason:
//! a count that large cannot arise from a document that fits in memory.

use mjx_opc::PartName;

use crate::error::Error;

/// A caller's `u32` index as the `usize` the model addresses with.
pub(crate) fn index(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// A model `usize` count or index as the `u32` the facade hands back.
pub(crate) fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// A caller's part name, as the validated [`PartName`] the package addresses with.
///
/// The facade speaks part names as `&str` — `/ppt/slides/slide1.xml` — because a binding cannot carry
/// an opaque handle across the boundary and back without inventing an object lifetime for it.
pub(crate) fn part_name(value: &str) -> Result<PartName, Error> {
    PartName::new(value).map_err(Error::from)
}
