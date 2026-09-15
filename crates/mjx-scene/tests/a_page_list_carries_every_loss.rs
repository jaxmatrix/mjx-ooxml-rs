//! A page's display list carries every loss its layout recorded, approximations and losses with no area included (MJXOFF-299).

use mjx_layout::{
    DecorationRef, FragmentTreeBuilder, FrameContent, ImageRef, LayoutLossKind, LayoutLosses,
    LayoutRect, LayoutSize, LossArea, PageFragments, PageIndex, PartId, SourcePath, SourceRef,
    TransformId,
};
use mjx_ooxml_core::measure::Emu;
use mjx_scene::{
    build_page, Decoration, Image, LossCategory, PainterLossKind, PainterLosses, Resolved,
    ResourceResolver, SceneOptions,
};
use mjx_text::{GlyphAtlas, GlyphRasteriser};

// A resolver with nothing to say about anything.
struct Silent;

impl ResourceResolver for Silent {
    fn decoration(&self, _reference: DecorationRef) -> Resolved<Decoration> {
        Resolved::NothingToDraw
    }

    fn text_decoration(&self, _source: &SourceRef) -> Resolved<Decoration> {
        Resolved::NothingToDraw
    }

    fn image(&self, _reference: ImageRef) -> Resolved<Image> {
        Resolved::NothingToDraw
    }
}

fn at(segments: &[u32]) -> SourceRef {
    SourceRef::node(PartId::PRIMARY, SourcePath::new(segments))
}

fn area() -> LossArea {
    LossArea {
        rect: LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_points(72.0),
            Emu::from_points(36.0),
        ),
        transform: TransformId::IDENTITY,
        clip: None,
    }
}

#[test]
fn every_layout_loss_reaches_the_list_and_the_painter_adds_its_own() {
    let mut losses = LayoutLosses::new();
    losses.record_at(at(&[1]), LayoutLossKind::DroppedByReader, area());
    losses.record_at(at(&[2]), LayoutLossKind::ValueApproximated, area());
    losses.record(at(&[3]), LayoutLossKind::TextMeasuredNotShaped);
    losses.record(
        at(&[4]),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink),
    );
    let page = PageFragments::new(PageIndex::FIRST, FragmentTreeBuilder::new().finish(), None)
        .with_losses(losses);
    let list = build_page(
        &page,
        &Silent,
        &mut GlyphRasteriser::new(),
        &mut GlyphAtlas::new(),
        &SceneOptions::new(LayoutSize::new(
            Emu::from_points(200.0),
            Emu::from_points(200.0),
        )),
    )
    .expect("the page builds");

    let carried = list.losses();
    assert_eq!(
        carried
            .iter()
            .map(|loss| (loss.category, loss.source.path().segments().to_vec()))
            .collect::<Vec<_>>(),
        vec![
            (
                LossCategory::Layout(LayoutLossKind::DroppedByReader),
                vec![1]
            ),
            (
                LossCategory::Layout(LayoutLossKind::ValueApproximated),
                vec![2]
            ),
            (
                LossCategory::Layout(LayoutLossKind::TextMeasuredNotShaped),
                vec![3]
            ),
            (
                LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink)),
                vec![4]
            ),
        ],
        "every layout loss, in the order it was recorded"
    );
    assert_eq!(
        list.placeholders().len(),
        1,
        "only the dropped content with an area draws"
    );

    let mut painted = PainterLosses::default();
    painted.add(PainterLossKind::LineEndNotDrawn, 2);
    let page_losses = list.losses().with_painter(painted);
    assert_eq!(
        page_losses.vector(),
        vec![
            (
                LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink)),
                1
            ),
            (
                LossCategory::Layout(LayoutLossKind::TextMeasuredNotShaped),
                1
            ),
            (LossCategory::Layout(LayoutLossKind::DroppedByReader), 1),
            (LossCategory::Layout(LayoutLossKind::ValueApproximated), 1),
            (LossCategory::Paint(PainterLossKind::LineEndNotDrawn), 2),
        ],
        "the page's whole vector, layout then scene then painter"
    );
    assert_eq!(page_losses.len(), 6);
}
