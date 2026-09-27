//! A picture anchored on a sheet is one layout loss under one labelled placeholder, and the resolver adds nothing to it (MJXOFF-299).

mod support;

use mjx_layout::{BoxModel, FrameContent, LayoutLossKind, PageIndex};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_scene::{build_page, LossCategory, ResourceResolver, SceneOptions};
use mjx_scene_xlsx::{SheetPalette, SheetResources};
use mjx_text::GlyphAtlas;
use mjx_xlsx::drawing_geometry::{CellMarker, ResizingBehavior};
use mjx_xlsx::Workbook;

// A valid one-pixel red PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn a_picture_on_a_sheet_is_one_layout_loss_under_one_placeholder_and_no_scene_loss() {
    let mut blank = Workbook::blank().expect("a blank workbook");
    blank
        .add_two_cell_anchored_picture(
            0,
            PNG,
            "Picture 1",
            CellMarker::new(1, 0, 1, 0),
            CellMarker::new(4, 0, 6, 0),
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("the picture anchors");
    let book = Workbook::open(&blank.save().expect("it saves")).expect("it reopens");
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = support::viewport(6.0, 4.0);
    let mut model = SheetBoxModel::new(support::resolver());
    let page = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");
    let resources = SheetResources::new(model.catalogue().clone(), SheetPalette::default());
    let frame = page
        .losses()
        .iter()
        .next()
        .map(|loss| loss.source.clone())
        .expect("the picture is a layout loss");
    assert_eq!(
        resources.unanswerable_content(&frame),
        None,
        "the layout counted the picture at its frame, so the resolver does not count it again"
    );
    let options = SceneOptions::new(constraints.page);
    let list = build_page(
        &page,
        &resources,
        model.rasteriser_mut(),
        &mut GlyphAtlas::new(),
        &options,
    )
    .expect("no loss fails a scene");
    let picture = LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(
        FrameContent::Picture,
    ));
    assert_eq!(list.losses().vector(), vec![(picture, 1)]);
    assert_eq!(
        list.placeholders()
            .into_iter()
            .map(|placeholder| (placeholder.category, placeholder.label, placeholder.source))
            .collect::<Vec<_>>(),
        vec![(picture, "Picture not rendered".to_owned(), frame)],
        "one labelled placeholder over the picture's anchor"
    );
}
