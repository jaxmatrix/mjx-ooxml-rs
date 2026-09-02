//! Package hygiene: sweeping orphan parts and redirecting references that point outside the file.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::part_name;
use crate::{Deck, Error, ExternalLink, TargetMode};

impl Deck {
    /// Removes every part the package no longer reaches from its root, and reports what was swept.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_unused_parts`](mjx_pptx::Presentation::remove_unused_parts).
    pub fn remove_unused_parts(&mut self) -> Result<Vec<String>, Error> {
        Ok(self
            .presentation
            .remove_unused_parts()?
            .into_iter()
            .map(|p| p.as_str().to_owned())
            .collect())
    }

    /// Every relationship in the package whose target lies **outside** it — a linked image, a chart's
    /// external workbook, a linked OLE object or media file — with the part that owns each.
    ///
    /// See [`Presentation::external_links`](mjx_pptx::Presentation::external_links).
    #[must_use]
    pub fn external_links(&self) -> Vec<ExternalLink> {
        self.presentation
            .external_links()
            .into_iter()
            .map(ExternalLink::from)
            .collect()
    }

    /// Repoints the relationship `id` of `source` (`None` = the package root) at `new_target`, keeping
    /// its id and its place in the `.rels`. Returns whether one was found.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::retarget_external_link`](mjx_pptx::Presentation::retarget_external_link).
    pub fn retarget_external_link(
        &mut self,
        source: Option<&str>,
        id: &str,
        new_target: &str,
        mode: TargetMode,
    ) -> Result<bool, Error> {
        Ok(self.presentation.retarget_external_link(
            source.map(part_name).transpose()?.as_ref(),
            id,
            new_target,
            mode,
        )?)
    }
}
