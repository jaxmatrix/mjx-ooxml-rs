//! A page's losses travel with its fragments and are what the box model recorded (MJXOFF-299).

use mjx_layout::{
    FragmentTreeBuilder, FrameContent, LayoutLossKind, LayoutLosses, LayoutRect, LossArea,
    PageFragments, PageIndex, PartId, SourcePath, SourceRef, TransformId,
};
use mjx_ooxml_core::measure::Emu;

#[test]
fn a_page_built_without_losses_reports_none_and_one_built_with_them_reports_them() {
    let bare = PageFragments::new(PageIndex::FIRST, FragmentTreeBuilder::new().finish(), None);
    assert!(bare.losses().is_empty());

    let mut losses = LayoutLosses::new();
    let frame = SourceRef::node(PartId::PRIMARY, SourcePath::new(&[0, 3]));
    let area = LossArea {
        rect: LayoutRect::from_edges(
            Emu::from_emu(0),
            Emu::from_emu(0),
            Emu::from_emu(914_400),
            Emu::from_emu(914_400),
        ),
        transform: TransformId::IDENTITY,
        clip: None,
    };
    let kind = LayoutLossKind::FrameContentNotLaidOut(FrameContent::Diagram);
    losses.record_at(frame.clone(), kind, area);
    let page = PageFragments::new(PageIndex::FIRST, FragmentTreeBuilder::new().finish(), None)
        .with_losses(losses);

    assert_eq!(page.losses().len(), 1);
    assert_eq!(page.losses().count(kind), 1);
    let loss = page.losses().iter().next().expect("one loss");
    assert_eq!((&loss.source, loss.area), (&frame, Some(area)));
    assert_eq!(
        (loss.kind.label(), loss.kind.draws_placeholder()),
        ("Diagram not rendered", true),
        "the recorded loss reads as the diagram it stands for and draws over it"
    );
    assert!(
        FrameContent::ALL.iter().all(|content| {
            LayoutLossKind::FrameContentNotLaidOut(*content).draws_placeholder()
                && FrameContent::ALL
                    .iter()
                    .filter(|other| other.label() == content.label())
                    .count()
                    == 1
        }),
        "every frame content not laid out draws a placeholder under a label of its own"
    );
}
