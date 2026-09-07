//! **The one file in this crate that names the font engine, and the only one the seam gate lets.**
//!
//! # Why the exemption exists, and why it is exactly one file
//!
//! MJXOFF-163 asks for two things that cannot both be had literally:
//!
//! * *"upload only what changed"* — which means R04's own delta,
//!   [`mjx_text::GlyphAtlas::take_delta`], because re-uploading the atlas per frame is the obvious
//!   wrong implementation; and
//! * *"`mjx-paint`'s dependencies are exactly the display list, the tokens and the graphics
//!   stack"* — which would forbid naming `mjx-text` at all.
//!
//! They collide because **a display list contains no pixels**. An [`mjx_scene::AtlasPlacement`] is a
//! *record of the frame the list was built for* — `mjx-scene` says so in as many words, and copies
//! the eight numbers rather than re-exporting `mjx_text::AtlasEntry` precisely so that a list
//! cached to disk stays readable after the page it named is gone. So the live atlas cannot be
//! reached through `mjx-scene`, and there is no third place to look.
//!
//! The resolution taken, of the two the ticket offers: **the painter defines the contract
//! ([`crate::AtlasSource`]) and this file is the only adapter to it.** What that buys over simply
//! permitting the dependency everywhere:
//!
//! * the painter, the frame plan, the pipelines and the shaders cannot name a font engine — the
//!   gate proves it file by file, not manifest by manifest;
//! * a shell with its own atlas (a server-side raster cache, a test double, R09's exporters)
//!   implements one trait rather than forking a painter; and
//! * the gate asserts `mjx-text` **is** named here, so the exemption cannot rot into a permission
//!   nothing uses.
//!
//! What it costs is honest and is stated rather than hidden: `mjx-paint`'s manifest declares
//! `mjx-text`, and `tests/the_seam_holds.rs` asserts that manifest exactly, so the cost is visible
//! in the one place a reader would look for it.
//!
//! # The ordering this adapter is responsible for
//!
//! [`mjx_text::AtlasDelta`] reports creations and drops in two separate collections, and its own
//! documentation warns that *"their indices may be issued again later, so a painter must release
//! the storage it held for one before honouring a creation that reuses it"*. Two collections cannot
//! express that on their own, so this adapter is where the order is imposed: **every release, then
//! every creation, then every write.** `tests/the_atlas_delta_reaches_the_painter.rs` is what holds
//! it, because getting it the other way round overwrites a live page with a new one's storage and
//! is invisible until an atlas is full enough to evict.

use mjx_text::{AtlasDelta, GlyphAtlas};

use crate::error::PaintError;
use crate::resources::{AtlasPage, AtlasSource, AtlasVisitor, AtlasWrite};

/// Hand one already-taken delta to a visitor, in the order the visitor's contract requires.
///
/// Separate from the [`AtlasSource`] implementation below so that a caller which took its delta
/// elsewhere — a render thread that receives one over a channel, which is how a shell with a
/// separate shaping worker will be built — can still deliver it correctly rather than reimplementing
/// the ordering.
///
/// # Errors
///
/// Whatever `visitor` fails with.
pub fn deliver(delta: &AtlasDelta, visitor: &mut dyn AtlasVisitor) -> Result<(), PaintError> {
    // Releases first. See the module documentation: an index is reissued after the page that held
    // it is dropped, so a creation honoured before its release would free the new page's storage.
    for page in delta.pages_dropped() {
        visitor.page_released(page.as_u32())?;
    }
    for created in delta.pages_created() {
        visitor.page_created(AtlasPage {
            page: created.page.as_u32(),
            format: created.format,
            size: created.size,
        })?;
    }
    for upload in delta.uploads() {
        visitor.write(AtlasWrite {
            page: upload.page.as_u32(),
            format: upload.format,
            x: upload.x,
            y: upload.y,
            width: upload.width,
            height: upload.height,
            pixels: upload.pixels(),
        })?;
    }
    Ok(())
}

impl AtlasSource for GlyphAtlas {
    fn take_changes(&mut self, visitor: &mut dyn AtlasVisitor) -> Result<(), PaintError> {
        let delta = self.take_delta();
        deliver(&delta, visitor)
    }
}
