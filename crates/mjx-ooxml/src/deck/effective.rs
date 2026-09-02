//! The effective readers: what a renderer *shows*, after walking the layout, master, theme and
//! presentation defaults and baking every colour to a concrete `RRGGBB`.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::index;
use crate::{
    CellBorder, CharacterPropertiesSpec, Deck, EffectListSpec, Error, FillSpec, LineSpec,
    ParagraphPropertiesSpec, ShapeBounds, ShapePath, Surface, Transform2D,
};

impl Deck {
    /// The **effective** fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec` whose
    /// colors are resolved to concrete `RRGGBB` values — the fill the shape actually renders. Three
    /// sources are tried, in order: an explicit `p:spPr` fill; a `p:style > a:fillRef` (the theme fill-
    /// style at that index, `phClr` substituted by the reference's color); and, for a placeholder shape
    /// (`p:ph`), **inheritance** from the same-slot placeholder on the layout then the master. Scheme
    /// colors and color transforms are baked against the surface's theme + map.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_shape_fill`](mjx_pptx::Presentation::effective_shape_fill).
    pub fn effective_shape_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self
            .presentation
            .effective_shape_fill(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **effective** outline of shape `shape_idx` on `surface`, as an interner-free `LineSpec`
    /// whose stroke color is resolved to a concrete `RRGGBB` value — the outline the shape actually
    /// renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`; a `p:style > a:lnRef`
    /// (the theme line-style at that index, `phClr` substituted by the reference's color); and, for a
    /// placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the slide layout
    /// then the master. Scheme colors and color transforms are baked against the slide's theme + map.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_shape_outline`](mjx_pptx::Presentation::effective_shape_outline).
    pub fn effective_shape_outline(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<LineSpec>, Error> {
        Ok(self
            .presentation
            .effective_shape_outline(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **effective** effects of shape `shape_idx` on `surface`, as an interner-free
    /// `EffectListSpec` whose colors are resolved to concrete `RRGGBB` values — the effects the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:effectLst`; a
    /// `p:style > a:effectRef` (the theme effect-style at that index, `phClr` substituted by the
    /// reference's color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot
    /// placeholder on the slide layout then the master. Scheme colors and color transforms are baked
    /// against the slide's theme + map.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_shape_effects`](mjx_pptx::Presentation::effective_shape_effects).
    pub fn effective_shape_effects(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<EffectListSpec>, Error> {
        Ok(self
            .presentation
            .effective_shape_effects(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **effective** transform of shape `shape_idx` on `surface` — where the shape actually
    /// renders, not what it declares. For a placeholder that places itself nowhere, this is the same-
    /// slot placeholder's transform on the slide layout, and failing that on the master.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_shape_transform`](mjx_pptx::Presentation::effective_shape_transform).
    pub fn effective_shape_transform(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Transform2D>, Error> {
        Ok(self
            .presentation
            .effective_shape_transform(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **effective** position and size of shape `shape_idx` on `surface` — where the shape actually
    /// renders, with the layout and master consulted for a placeholder that declares no bounds of its
    /// own.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_shape_bounds`](mjx_pptx::Presentation::effective_shape_bounds).
    pub fn effective_shape_bounds(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<ShapeBounds>, Error> {
        Ok(self
            .presentation
            .effective_shape_bounds(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **effective** character properties of run `run_idx` — what the run actually renders as, with
    /// every tier of inheritance resolved and its colors baked to concrete `RRGGBB`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_run_properties`](mjx_pptx::Presentation::effective_run_properties).
    pub fn effective_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<CharacterPropertiesSpec, Error> {
        Ok(self.presentation.effective_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// The **effective** paragraph properties of paragraph `para_idx` — the layout it actually renders
    /// with, every tier of inheritance resolved.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_paragraph_properties`](mjx_pptx::Presentation::effective_paragraph_properties).
    pub fn effective_paragraph_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<ParagraphPropertiesSpec, Error> {
        Ok(self.presentation.effective_paragraph_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?)
    }

    /// The **effective** fill of the cell at `(row, column)` of the table shape `shape_idx` frames — an
    /// interner-free `FillSpec` with its colour baked to concrete `RRGGBB`, or `None` if nothing fills
    /// the cell. The cell's own `a:tcPr` fill wins; else the first applicable style part with a fill
    /// (explicit or a theme `fillRef`).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_cell_fill`](mjx_pptx::Presentation::effective_cell_fill).
    pub fn effective_cell_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self.presentation.effective_cell_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// The **effective** border on one `edge` of the cell at `(row, column)` — an interner-free
    /// `LineSpec` with its stroke colour baked, or `None`. The cell's own `a:tcPr` edge wins; else the
    /// applicable style parts' `a:tcBdr`, taking the outer edge (`top`/`left`/…) for a cell on the
    /// table's rim and the interior edge (`insideH`/`insideV`) for one within it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_cell_border`](mjx_pptx::Presentation::effective_cell_border).
    pub fn effective_cell_border(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, Error> {
        Ok(self.presentation.effective_cell_border(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            edge,
        )?)
    }

    /// The **effective** run properties of a cell's text run — the `CharacterPropertiesSpec` it
    /// actually renders with, colours baked. A shorter ladder than a shape's (a cell inherits from its
    /// table style, not a placeholder chain), highest first: the run's own `a:rPr`, the paragraph's
    /// `a:defRPr`, the table style's `a:tcTxStyle` for each applicable part (bold / italic / colour),
    /// then the presentation's `p:defaultTextStyle`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::effective_cell_run_properties`](mjx_pptx::Presentation::effective_cell_run_properties).
    pub fn effective_cell_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<CharacterPropertiesSpec, Error> {
        Ok(self.presentation.effective_cell_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            index(run_idx),
        )?)
    }
}
