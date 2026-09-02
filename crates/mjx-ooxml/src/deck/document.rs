//! Deck addressing: how many slides, layouts and masters there are, what each is called, and the
//! theme and colour map a surface resolves against.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{ColorMap, Deck, Error, LayoutInfo, SlideLayoutKind, SlideSize, Surface, ThemeInfo};

impl Deck {
    /// The number of slides, in presentation order.
    ///
    /// See [`Presentation::slide_count`](mjx_pptx::Presentation::slide_count).
    #[must_use]
    pub fn slide_count(&self) -> u32 {
        count(self.presentation.slide_count())
    }

    /// The number of slide masters, in `p:sldMasterIdLst` order.
    ///
    /// See [`Presentation::master_count`](mjx_pptx::Presentation::master_count).
    #[must_use]
    pub fn master_count(&self) -> u32 {
        count(self.presentation.master_count())
    }

    /// The name of master `idx` (`p:cSld@name`, e.g. `Office Theme`), or `None` if it is unnamed.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::master_name`](mjx_pptx::Presentation::master_name).
    pub fn master_name(&mut self, idx: u32) -> Result<Option<String>, Error> {
        Ok(self.presentation.master_name(index(idx))?)
    }

    /// Every slide layout the deck offers, in layout-index order — the inventory a caller reads before
    /// choosing one to build a slide on.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::layouts`](mjx_pptx::Presentation::layouts).
    pub fn layouts(&mut self) -> Result<Vec<LayoutInfo>, Error> {
        Ok(self.presentation.layouts()?)
    }

    /// The number of slide layouts across the whole deck, in (master order, `p:sldLayoutIdLst` order) —
    /// so layout indices run master by master. `layout_master` says which master an index belongs to.
    ///
    /// See [`Presentation::layout_count`](mjx_pptx::Presentation::layout_count).
    #[must_use]
    pub fn layout_count(&self) -> u32 {
        count(self.presentation.layout_count())
    }

    /// The index of the master that lists layout `idx`.
    ///
    /// See [`Presentation::layout_master`](mjx_pptx::Presentation::layout_master).
    #[must_use]
    pub fn layout_master(&self, idx: u32) -> Option<u32> {
        self.presentation.layout_master(index(idx)).map(count)
    }

    /// The name of layout `idx` (`p:cSld@name`, e.g. `Title and Content` — the name PowerPoint shows in
    /// its layout gallery), or `None` if it is unnamed.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::layout_name`](mjx_pptx::Presentation::layout_name).
    pub fn layout_name(&mut self, idx: u32) -> Result<Option<String>, Error> {
        Ok(self.presentation.layout_name(index(idx))?)
    }

    /// How layout `idx` arranges its content (`p:sldLayout@type`) — a coarse description of which
    /// placeholders it offers, which an application can use to map between layouts.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::layout_kind`](mjx_pptx::Presentation::layout_kind).
    pub fn layout_kind(&mut self, idx: u32) -> Result<SlideLayoutKind, Error> {
        Ok(self.presentation.layout_kind(index(idx))?)
    }

    /// The index of the layout slide `slide_idx` is built on, or `None` if the slide relates to no
    /// layout (or to one no master lists).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::slide_layout`](mjx_pptx::Presentation::slide_layout).
    pub fn slide_layout(&self, slide_idx: u32) -> Result<Option<u32>, Error> {
        Ok(self.presentation.slide_layout(index(slide_idx))?.map(count))
    }

    /// The size of every slide in the deck (`p:sldSz`) — the extent shape bounds are laid out in.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::slide_size`](mjx_pptx::Presentation::slide_size).
    pub fn slide_size(&mut self) -> Result<SlideSize, Error> {
        Ok(self.presentation.slide_size()?)
    }

    /// The theme that governs `surface`, as an interner-free `ThemeInfo` (its color scheme + fill-style
    /// matrix) — the theme related to the last part of the surface's inheritance chain (slide →
    /// slideLayout → slideMaster → theme, and the shorter walks from a layout or master). Returns
    /// `Ok(None)` if any hop is absent (a deck without a theme). Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::theme`](mjx_pptx::Presentation::theme).
    pub fn theme(&mut self, surface: Surface) -> Result<Option<ThemeInfo>, Error> {
        Ok(self.presentation.theme(surface.to_model())?)
    }

    /// The effective theme `ColorMap` for `surface`: the master's `p:clrMap` (reached along the
    /// surface's inheritance chain), replaced by the surface's own `p:clrMapOvr > a:overrideClrMapping`
    /// when it supplies a full mapping (a `masterClrMapping`, an absent override, or a schema-loose
    /// attribute-less override all inherit the master's map). It maps the logical color names a shape
    /// may reference (`bg1`/`tx1`/…) to the theme's concrete scheme slots. `Ok(None)` when there is no
    /// reachable master or no `p:clrMap`. Reading does not dirty a part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::color_map`](mjx_pptx::Presentation::color_map).
    pub fn color_map(&mut self, surface: Surface) -> Result<Option<ColorMap>, Error> {
        Ok(self.presentation.color_map(surface.to_model())?)
    }
}
