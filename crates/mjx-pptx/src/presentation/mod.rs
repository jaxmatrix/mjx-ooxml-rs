//! The [`Presentation`] entry point: open, read shape text, edit a run, save.
//!
//! The surface is large, so it is split by subject — the same seams the guide reads in. Every
//! method below is an inherent method on [`Presentation`], so the split changes no path a caller
//! imports: `mjx_pptx::Presentation` has them all, wherever their source lives.

use mjx_ooxml_types::namespaces::PML;
use mjx_opc::{Package, PartName, TargetMode};

use crate::error::PptxError;
use crate::geometry::SlideSize;
use crate::{blank, constants, nav};

mod appearance;
mod bounds;
mod cells;
mod chart_decoration;
mod charts;
mod deck;
mod effective;
mod element_builders;
mod hyperlinks;
mod legacy_content;
mod notes;
mod pictures;
mod shapes;
mod slides;
mod tables;
#[cfg(test)]
mod tests;
mod text;

use deck::referenced_parts;

pub use chart_decoration::{
    ChartErrorBarData, ChartLabelScope, ChartPointFormatData, ChartTrendlineData,
};
pub use charts::{ChartAxisData, ChartLegendData, ChartSeriesData};
pub use deck::LayoutInfo;

/// An open PresentationML document: an OPC [`Package`] plus its resolved presentation part and the
/// ordered list of slide parts.
///
/// Reads and edits are addressed by a [`Surface`](crate::Surface) (a slide, layout, or master — a bare `usize` means
/// a slide) plus `shape_idx` / `run_idx`. Reading a part never dirties it; editing marks only that one
/// part dirty, so [`save`](Self::save) re-emits every other part byte-identically.
///
/// Editing a **layout or master** is how one change reaches many slides: a slide placeholder that
/// declares no property of its own inherits from the same-slot placeholder up its chain (see
/// [`effective_shape_fill`](Self::effective_shape_fill)).
#[derive(Debug)]
pub struct Presentation {
    package: Package,
    presentation_part: PartName,
    slides: Vec<PartName>,
    masters: Vec<PartName>,
    /// Every master's layouts, master by master (see [`Presentation::layout_count`]).
    layouts: Vec<PartName>,
    /// `layout_owners[i]` is the index in `masters` of the master that lists `layouts[i]`.
    layout_owners: Vec<usize>,
}

impl Presentation {
    /// Opens a presentation from its container bytes, resolving the presentation part and the ordered
    /// slide parts.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the package is unreadable, has no `officeDocument` relationship, or its
    /// `presentation.xml` / relationships are malformed.
    pub fn open(bytes: &[u8]) -> Result<Self, PptxError> {
        Self::from_package(Package::open(bytes)?)
    }

    /// Creates a blank deck: one slide master, one slide layout, a theme, and no slides.
    ///
    /// This is the constructor that does not need a file. Every part is authored from code (see the
    /// `blank` module) rather than unpacked from a committed template, so the markup is markup this
    /// project can explain and the same schema gate that validates an edited deck validates this one.
    ///
    /// The deck it hands back is a *starting point*, not a finished document: it has no slides.
    /// [`add_slide_from_layout(0)`](Self::add_slide_from_layout) builds one on the layout, carrying a
    /// title and a body placeholder to fill with
    /// [`set_shape_text_content`](Self::set_shape_text_content); [`add_slide`](Self::add_slide)
    /// builds an empty one for [`add_text_box`](Self::add_text_box).
    ///
    /// ```
    /// # fn main() -> Result<(), mjx_pptx::PptxError> {
    /// use mjx_pptx::{Presentation, SlideSize};
    /// use mjx_ooxml_types::presentationml::SlideSizeKind;
    ///
    /// let mut deck = Presentation::blank(SlideSize {
    ///     width_emu: 12_192_000,
    ///     height_emu: 6_858_000,
    ///     kind: SlideSizeKind::Screen16X9,
    /// })?;
    /// let slide = deck.add_slide_from_layout(0)?;
    /// deck.set_shape_text_content(slide, 0, "Hello")?;
    /// let bytes = deck.save()?;
    /// # let _ = bytes;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`PptxError::InvalidSlideSize`] if either extent is outside the range `p:sldSz` can
    /// express (`914400`..=`51206400` EMU), or another [`PptxError`] if building the package fails.
    pub fn blank(size: SlideSize) -> Result<Self, PptxError> {
        Self::from_package(blank::package(size)?)
    }

    /// Resolves an already-loaded [`Package`] into a presentation: the `officeDocument` relationship,
    /// the presentation part, and the slide / master / layout part graph.
    ///
    /// This is the constructor for a caller who already holds the package — one opened by
    /// [`mjx_opc`] directly, one a facade opened once and dispatched on by content type, or one
    /// authored part by part. [`open`](Self::open) is this with `Package::open` in front of it, and
    /// [`blank`](Self::blank) is this with an authored package in front of it, so a deck built from
    /// nothing is navigated by exactly the same code that navigates a deck read from a file — a
    /// blank package this could not resolve fails here rather than surviving as a special case.
    ///
    /// The package is taken by value because the presentation owns it from here: every read borrows
    /// from it and every edit dirties a part of it, and [`save`](Self::save) hands the bytes back.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use mjx_pptx::{Package, Presentation};
    ///
    /// let package = Package::open(&std::fs::read("deck.pptx")?)?;
    /// let mut deck = Presentation::from_package(package)?;
    /// # let _ = deck.slide_count();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`PptxError`] if the package has no `officeDocument` relationship, or its
    /// `presentation.xml` or relationships are malformed.
    pub fn from_package(mut package: Package) -> Result<Self, PptxError> {
        // Package root -> officeDocument relationship -> the presentation part.
        let presentation_part = {
            let root_rels = package
                .relationships_for(None)
                .ok_or(PptxError::MissingOfficeDocument)?;
            let rel = root_rels
                .by_type(constants::REL_OFFICE_DOCUMENT)
                .next()
                .ok_or(PptxError::MissingOfficeDocument)?;
            if rel.mode == TargetMode::External {
                return Err(PptxError::ExternalTarget {
                    target: rel.target.clone(),
                });
            }
            nav::resolve_from_root(&rel.target)?
        };
        if package.part_bytes(&presentation_part).is_none() {
            return Err(PptxError::MissingPresentationPart(
                presentation_part.as_str().to_owned(),
            ));
        }

        // presentation.xml -> p:sldIdLst -> each p:sldId's r:id -> the slide parts. A deck must have
        // the list (an empty one is fine); the same walk resolves masters and, per master, layouts.
        {
            let doc = package.part_tree(&presentation_part)?;
            if nav::child(&doc.root, &doc.interner, PML, "sldIdLst").is_none() {
                return Err(PptxError::MalformedPresentation("missing p:sldIdLst"));
            }
        }
        let slides = referenced_parts(&mut package, &presentation_part, "sldIdLst", "sldId")?;
        let masters = referenced_parts(
            &mut package,
            &presentation_part,
            "sldMasterIdLst",
            "sldMasterId",
        )?;

        // Each master lists its own layouts; the flat layout index runs master by master, in order.
        let mut layouts = Vec::new();
        let mut layout_owners = Vec::new();
        for (master_idx, master) in masters.iter().enumerate() {
            let master = master.clone();
            for layout in referenced_parts(&mut package, &master, "sldLayoutIdLst", "sldLayoutId")?
            {
                layouts.push(layout);
                layout_owners.push(master_idx);
            }
        }

        Ok(Self {
            package,
            presentation_part,
            slides,
            masters,
            layouts,
            layout_owners,
        })
    }

    /// Validates the deck, then serializes it back to container bytes (only edited parts
    /// re-serialize).
    ///
    /// # The check is not optional
    ///
    /// [`validate`](Self::validate) runs first, and a deck that violates a packaging or a
    /// PresentationML invariant — a `p:sldId` naming a relationship that is not there, a slide the
    /// deck relates to but never lists, two shapes sharing a non-visual id — is **not written**.
    /// Those are the faults that make PowerPoint offer to repair a file, and none of them is visible
    /// to a per-part schema check. [`save_unchecked`](Self::save_unchecked) is the deliberate escape
    /// hatch.
    ///
    /// # Errors
    /// Returns [`PptxError::InvalidPresentation`] or [`PptxError::Opc`] (carrying an
    /// [`OpcError::Invalid`](mjx_opc::OpcError::Invalid)) if the deck violates an invariant, or
    /// another [`PptxError`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, PptxError> {
        self.validate()?;
        self.save_unchecked()
    }

    /// Serializes the presentation back to container bytes **without** checking its invariants.
    ///
    /// Identical to [`save`](Self::save) but for the validation pass. Reach for it only when writing
    /// a deck you know to be inconsistent is the point: re-saving a file that was already broken when
    /// it was opened, or a state you mean to finish later. Anything this writes that
    /// [`validate`](Self::validate) would have rejected is a file PowerPoint may offer to repair.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the ZIP writer fails.
    pub fn save_unchecked(&self) -> Result<Vec<u8>, PptxError> {
        Ok(self.package.save_unchecked()?)
    }

    /// Checks every invariant [`save`](Self::save) enforces, without writing anything: first the
    /// packaging graph ([`Package::validate`](mjx_opc::Package::validate)), then the PresentationML
    /// identifier and list invariants on top of it.
    ///
    /// Both passes are read-only and are scoped to the markup this library will actually write (see
    /// [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts)), so a deck opened and
    /// left alone is never faulted for markup it arrived with, and reading a slide can never change
    /// the answer.
    ///
    /// # Errors
    /// Returns the first invariant broken, as [`PptxError::Opc`] for a packaging defect or
    /// [`PptxError::InvalidPresentation`] for a PresentationML one.
    pub fn validate(&self) -> Result<(), PptxError> {
        self.package.validate().map_err(mjx_opc::OpcError::from)?;
        crate::validate::check(&self.package)
    }
}
