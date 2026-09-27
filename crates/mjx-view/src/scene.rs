//! [`SceneSource`] — the seam between a viewport and whatever turns a page into a display list.
//!
//! # Why this is a trait and not a call to `mjx_scene::build_scene`
//!
//! `build_scene` needs three things a viewport has no business owning: a `ResourceResolver` (the
//! document's theme, its images, its relationships), a `GlyphRasteriser` and a `GlyphAtlas`. The
//! first is the *document's*, and the last two are the **painter's** — an atlas is a texture, and
//! its residency belongs to whatever is going to sample it. A viewport that constructed them would
//! have to know what a device is, and every one of this crate's tests would need a font.
//!
//! So the viewport asks, and the shell answers. That is the same shape as
//! [`BoxModel`](mjx_layout::BoxModel) one layer down, and it has the same payoff: the windowing,
//! the caching and the frame budget are written once and are true for a `.pptx`, a `.docx`, an HTML
//! paste and a synthetic test document alike.
//!
//! # What an implementation owes
//!
//! [`build`](SceneSource::build) is called for a page whose fragments are already in hand, at most
//! once per page per invalidation, and its answer is **cached by byte size**. Two consequences:
//!
//! * it must be a pure function of the fragments it is given — a source that consulted mutable state
//!   the viewport cannot see would produce a display list the viewport goes on serving after that
//!   state changed;
//! * it may be slow. It is scheduled inside a frame budget, and a source that overruns defers the
//!   next page rather than dropping it.

use mjx_layout::{PageFragments, PageIndex};
use mjx_scene::DisplayList;

/// Anything that can turn one laid-out page into a display list.
pub trait SceneSource {
    /// What building one can fail with. The source's own: only it knows what a document can be
    /// wrong about, exactly as [`BoxModel::Error`](mjx_layout::BoxModel::Error) is the box model's.
    type Error: std::error::Error;

    /// Builds the display list for `page` from `fragments`.
    ///
    /// # Errors
    /// The source's own.
    fn build(
        &mut self,
        page: PageIndex,
        fragments: &PageFragments,
    ) -> Result<DisplayList, Self::Error>;
}
