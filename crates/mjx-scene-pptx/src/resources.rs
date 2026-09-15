//! [`SlideResources`] — what the handles in a slide's fragment tree resolve to.

use mjx_layout::{DecorationRef, ImageRef, SourceRef};
use mjx_layout_pptx::PageCatalogue;
use mjx_scene::{
    Decoration, DeviceScale, FillStyle, Image, Resolved, ResourceResolver, SceneLossKind,
};

use crate::effects::effect_styles;
use crate::paint::{fill_style, stroke_style};

/// The resolver for one page of a slide deck.
///
/// It owns a [`PageCatalogue`] rather than borrowing one because
/// [`SlideBoxModel::catalogue`](mjx_layout_pptx::SlideBoxModel::catalogue) is rebuilt on every
/// `layout_page` and `build_scene` needs the rasteriser mutably at the same time — so a borrow of
/// the box model would have to be held across a call that takes another. The catalogue for one page
/// is a handful of `Vec`s; cloning it is cheaper than the borrow it avoids, and the ownership is
/// what makes *a resolver is only ever paired with the page it was made from* checkable rather than
/// hoped for.
#[derive(Clone, PartialEq, Debug)]
pub struct SlideResources {
    catalogue: PageCatalogue,
    scale: DeviceScale,
}

impl SlideResources {
    /// The resolver for the page `catalogue` describes, drawn at `scale`.
    ///
    /// The scale is here because a stroke's width, a shadow's blur radius and its offset are all
    /// stated in **device pixels** in `mjx-scene` and in EMU in the document, and
    /// [`ResourceResolver`] takes no scale of its own — an answer must be a value, not a function of
    /// one. It has to be the same scale [`mjx_scene::SceneOptions::device_scale`] is built with, or
    /// a shape's outline will be the right colour at the wrong weight.
    #[must_use]
    pub fn new(catalogue: PageCatalogue, scale: DeviceScale) -> Self {
        Self { catalogue, scale }
    }

    /// The catalogue it answers from.
    #[must_use]
    pub fn catalogue(&self) -> &PageCatalogue {
        &self.catalogue
    }

    /// The scale its lengths are converted at.
    #[must_use]
    pub fn scale(&self) -> DeviceScale {
        self.scale
    }

    /// The handle a picture's relationship id was issued under, or `None` when the page names no
    /// such image.
    ///
    /// A `a:blipFill` inside a shape's `p:spPr` names a relationship, and the catalogue's image
    /// table was built from the page's `p:pic` shapes — so a shape filled with a picture that no
    /// `p:pic` also shows finds nothing here, and paints nothing rather than painting a wrong
    /// colour. That is a real gap and it is a small one: giving a *fill's* blip its own handle needs
    /// the box model to issue one, which needs `effective_shape_fill` to be asked for a rectangle it
    /// does not have.
    fn image_handle(&self, rel_id: &str) -> Option<u64> {
        self.catalogue
            .images()
            .iter()
            .position(|request| request.image_rel_id == rel_id)
            .map(|index| index as u64)
    }
}

impl ResourceResolver for SlideResources {
    fn decoration(&self, reference: DecorationRef) -> Resolved<Decoration> {
        if PageCatalogue::is_chart_handle(reference.number()) {
            // Nothing on this side resolves a chart's paints; the chart is counted once, at its frame.
            return Resolved::Unanswerable(SceneLossKind::ChartNotResolved);
        }
        let Some(entry) = self.catalogue.decoration(reference) else {
            return Resolved::NothingToDraw;
        };
        let image = |rel_id: &str| self.image_handle(rel_id);
        let built = || -> Result<Decoration, SceneLossKind> {
            Ok(Decoration {
                fill: match entry.fill.as_ref() {
                    Some(fill) => fill_style(fill, &image)?,
                    None => FillStyle::None,
                },
                stroke: match entry.outline.as_ref() {
                    Some(outline) => stroke_style(outline, self.scale, &image)?,
                    None => None,
                },
                // A shape's own transparency is `a:alpha` on its fill's colour, not a group opacity, and
                // that alpha is gone one crate below (see `crate::paint`). Stating `1.0` is therefore
                // not a placeholder for a value that exists — there is no per-shape opacity in
                // DrawingML for this to carry — and a `PushOpacity` this crate never emits is one the
                // painter never has to open a layer for.
                opacity: 1.0,
                effects: match entry.effects.as_ref() {
                    Some(effects) => effect_styles(effects, self.scale, &image)?,
                    None => Vec::new(),
                },
            })
        };
        match built() {
            Ok(decoration) => Resolved::Answered(decoration),
            Err(kind) => Resolved::Unanswerable(kind),
        }
    }

    fn text_decoration(&self, _source: &SourceRef) -> Resolved<Decoration> {
        // A run's own fill is `a:rPr > a:solidFill`, and it reaches this crate through nothing: the
        // box model's catalogue indexes decorations by the handle a *shape* fragment carries, and a
        // `GlyphRunFragment` carries none — which is why this method is addressed by `SourceRef` at
        // all. Answering that it cannot say makes the builder draw the run in `DEFAULT_TEXT_COLOR`
        // and count it as a text colour approximated, so the approximation is visible in the list.
        //
        // Wiring it needs the box model to publish a per-run decoration table keyed by the same
        // addresses it puts on its glyph runs. That is a table `mjx-layout-pptx` can build — every
        // run's `CharacterPropertiesSpec` is already read — and it is not built yet, so this is a
        // stated gap rather than a silent one. `DEFAULT_TEXT_COLOR` is named here so a reader can
        // see what the answer collapses to.
        // Owned by MJXOFF-311 (RC16), run colour.
        Resolved::Unanswerable(SceneLossKind::TextPaintDefaulted)
    }

    fn image(&self, reference: ImageRef) -> Resolved<Image> {
        let Some(request) = self.catalogue.image(reference) else {
            return Resolved::NothingToDraw;
        };
        if request.image_rel_id.is_empty() {
            // A `p:pic` with no `a:blip` names no image at all. Drawing nothing is right; drawing
            // handle zero would draw whatever the first picture on the page happens to be.
            return Resolved::NothingToDraw;
        }
        // `a:srcRect` and the image adjustments are not modelled in `mjx-dml` — see the crate's own
        // documentation — so the picture is shown whole and unadjusted. `Image::stretched` states
        // exactly that, rather than a crop and an adjustment set invented here.
        Resolved::Answered(Image::stretched(reference.number()))
    }

    fn unanswerable_content(&self, source: &SourceRef) -> Option<SceneLossKind> {
        self.catalogue
            .is_chart_frame(source)
            .then_some(SceneLossKind::ChartNotResolved)
    }
}
