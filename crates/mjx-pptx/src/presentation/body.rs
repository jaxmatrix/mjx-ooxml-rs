//! A text body's geometry: what a shape's `a:bodyPr` states, and what it renders as once the
//! placeholder chain has been walked.
//!
//! # Why this is a resolver and not an accessor
//!
//! A title placeholder that declares no `a:bodyPr` of its own still anchors its text somewhere, and
//! where is on its layout — or, failing that, on its master. That is the same question
//! [`effective_shape_bounds`](Presentation::effective_shape_bounds) answers for position and
//! [`effective_run_properties`](Presentation::effective_run_properties) answers for character
//! formatting, walked down the same spine: the placeholder-candidate walk gives the shape and then the
//! same-slot placeholder on each part the surface inherits from, and each tier contributes only what
//! the tiers above it left unset.
//!
//! A shape that is **not** a placeholder has no slot to be matched on, so it inherits nothing and
//! this answers exactly what it states — which is the same rule every other effective property here
//! follows.
//!
//! # What is deliberately *not* resolved
//!
//! The schema's own defaults. `a:bodyPr@lIns`'s default is 0.1 inch, but a spec field left `None`
//! here means *no tier stated it*, not *it is 0.1 inch* — because a reader that cannot tell an
//! authored `0` from an unstated one has lost information a round-trip needs. The defaults are named
//! constants on [`TextBodyPropertiesSpec`], applied by whoever lays the text out.

use mjx_dml::{TextBody, TextBodyPropertiesSpec};
use mjx_ooxml_core::{FromXml, RawDocument};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::slide;
use crate::surface::Surface;

use super::effective::candidate_shape;
use super::Presentation;

impl Presentation {
    /// The geometry shape `shape_idx` on `surface` **states itself** (`p:txBody > a:bodyPr`), or
    /// `None` when it has no text body or the body states no `a:bodyPr`.
    ///
    /// A `None` field of the returned spec is not a zero: it means the shape does not state that
    /// attribute, so a placeholder takes it from its layout and then its master — resolve *that*
    /// with [`effective_body_properties`](Self::effective_body_properties). Reading does not dirty
    /// the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn body_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<TextBodyPropertiesSpec>, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        stated_body_properties(doc, surface, &path)
    }

    /// The **effective** geometry of shape `shape_idx`'s text body — the insets, anchor, wrap,
    /// columns, writing direction and autofit the text actually lays out under, with the layout and
    /// master consulted for a placeholder that states none of its own.
    ///
    /// Every tier contributes attribute by attribute: a slide title that states only `anchor="ctr"`
    /// keeps its layout's insets and its master's column count. A shape that is not a placeholder
    /// answers exactly what it states, because it has no slot to inherit through.
    ///
    /// Returns an empty spec when no tier states anything. Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the surface's part is malformed, or a
    /// relationship in the inheritance chain points outside the package.
    pub fn effective_body_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<TextBodyPropertiesSpec, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let candidates = self.placeholder_candidates(surface, &path)?;

        let mut resolved = TextBodyPropertiesSpec::new();
        for (part, candidate) in candidates {
            let doc = self.package.part_tree(&part)?;
            let Some(shape) = candidate_shape(doc, candidate)? else {
                continue; // This tier does not define the slot at all.
            };
            let Some(stated) = body_properties_of(doc, shape) else {
                continue; // …or defines it with no text body, or no `a:bodyPr` on the body.
            };
            resolved = resolved.merge_under(&stated);
        }
        Ok(resolved)
    }

    /// Restates the geometry of shape `shape_idx`'s text body, writing **only** what `spec` names.
    ///
    /// An attribute the spec leaves unset is left exactly as it was, so one inset can be changed
    /// without restating the other three, and every unmodeled attribute (`@fromWordArt`,
    /// `@forceAA`) and child (`a:prstTxWarp`, `a:extLst`) survives. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has
    /// no text body ([`ShapeHasNoTextBody`](PptxError::ShapeHasNoTextBody)).
    pub fn set_body_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        spec: &TextBodyPropertiesSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        self.edit_text_body(surface, shape_idx.into(), |body, interner| {
            match body.body_properties(interner) {
                Some(mut properties) => {
                    properties.apply(spec, interner);
                    body.set_body_properties(interner, &properties);
                }
                None => {
                    // `a:bodyPr` is schema-required, so a body without one is malformed; giving it
                    // the one the caller asked for repairs the body rather than refusing the edit.
                    let properties = spec.to_properties(interner);
                    body.set_body_properties(interner, &properties);
                }
            }
            Ok(())
        })
    }
}

/// The `a:bodyPr` of the shape at `path` on `surface`, read from the document the caller holds.
fn stated_body_properties(
    doc: &RawDocument,
    surface: Surface,
    path: &ShapePath,
) -> Result<Option<TextBodyPropertiesSpec>, PptxError> {
    let shape = super::effective::resolve_shape_ref(doc, surface, path)?;
    Ok(body_properties_of(doc, shape))
}

/// The `a:bodyPr` of one already-resolved shape, as a spec — `None` when it has no `p:txBody`, when
/// the body will not parse, or when the body states no `a:bodyPr`.
///
/// A malformed body is *absence* rather than an error, because an effective walk steps past a tier
/// that says nothing and a file this crate cannot read must still lay out.
fn body_properties_of(
    doc: &RawDocument,
    shape: &mjx_ooxml_core::RawElement,
) -> Option<TextBodyPropertiesSpec> {
    let txbody = slide::shape_txbody(shape, &doc.interner)?;
    let body = TextBody::from_xml(txbody, &doc.interner).ok()?;
    body.body_properties(&doc.interner)
        .map(|properties| properties.spec(&doc.interner))
}
