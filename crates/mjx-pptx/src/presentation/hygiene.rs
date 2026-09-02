//! Package hygiene: sweeping orphan parts and redirecting references that point outside the file.
//!
//! These three are the only reasons a caller needs to think about the *package* rather than the
//! deck, and each is a fidelity operation rather than an authoring one. They are inherent methods on
//! [`Presentation`] — not an exposed `package()` accessor — deliberately: handing out `&mut Package`
//! would give a caller mutable access to the whole part graph and make every invariant this crate
//! enforces on [`save`](Presentation::save) unenforceable.

use mjx_opc::{ExternalRelationship, PartName, TargetMode};

use crate::error::PptxError;
use crate::presentation::Presentation;

impl Presentation {
    /// Removes every part the package no longer reaches from its root, and reports what was swept.
    ///
    /// Editing a deck strands parts: removing a slide leaves its notes slide, its images and its
    /// chart behind, each still a real entry in the container and each still counted in its size.
    /// This is the sweep — a transitive reachability walk from `_rels/.rels` over *internal*
    /// relationships, removing everything it does not reach (control parts excepted, which are
    /// reached by convention rather than by relationship).
    ///
    /// # Errors
    /// Returns [`PptxError::Opc`] if removing a swept part's content-type entry fails. The
    /// reachability analysis itself cannot fail: a relationship naming a part that is not present is
    /// simply not followed.
    pub fn remove_unused_parts(&mut self) -> Result<Vec<PartName>, PptxError> {
        Ok(self.package.remove_unreferenced_parts()?)
    }

    /// Every relationship in the package whose target lies **outside** it — a linked image, a
    /// chart's external workbook, a linked OLE object or media file — with the part that owns each.
    ///
    /// These are the references that can be unreachable on some other machine. This library does no
    /// external I/O, so it cannot tell which of them resolve; this is the discovery surface a caller
    /// uses to decide, then neutralizes one at a time with
    /// [`retarget_external_link`](Self::retarget_external_link).
    #[must_use]
    pub fn external_links(&self) -> Vec<ExternalRelationship> {
        self.package.external_relationships()
    }

    /// Repoints the relationship `id` of `source` (`None` = the package root) at `new_target`,
    /// keeping its id and its place in the `.rels`. Returns whether one was found.
    ///
    /// Because the id does not change, the element that binds the relationship — a blip, a chart's
    /// `c:externalData`, an OLE object — resolves at the new target without its own markup being
    /// touched, which is what makes this work for the many element kinds this library does not
    /// model. Pair it with [`TargetMode::Internal`] to point an external reference at a placeholder
    /// part already inside the package.
    ///
    /// # Errors
    /// Returns [`PptxError::Opc`] if the owning `.rels` part is not well-formed XML.
    pub fn retarget_external_link(
        &mut self,
        source: Option<&PartName>,
        id: &str,
        new_target: &str,
        mode: TargetMode,
    ) -> Result<bool, PptxError> {
        Ok(self
            .package
            .retarget_relationship(source, id, new_target, mode)?)
    }
}
