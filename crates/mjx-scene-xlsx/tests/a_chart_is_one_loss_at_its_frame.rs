//! The worksheet companion answers a chart it cannot resolve as unanswerable, at its frame and at its handles (MJXOFF-299).

mod support;

use mjx_layout::Fragment;
use mjx_layout_xlsx::PageCatalogue;
use mjx_scene::{Resolved, ResourceResolver, SceneLossKind};

use support::{fixture, resolve, viewport};

#[test]
fn a_chart_is_unanswerable_at_its_frame_and_at_the_handles_it_issued() {
    let mut book = mjx_xlsx::Workbook::open(&fixture("chart_in_sheet.xlsx")).expect("it opens");
    let resolved = resolve(&mut book, 0, &viewport(20.0, 20.0));
    let frames = resolved
        .tree
        .nodes()
        .filter(|(_, node)| {
            resolved.resources.unanswerable_content(node.source())
                == Some(SceneLossKind::ChartNotResolved)
        })
        .count();
    assert_eq!(frames, 1, "the sheet anchors one chart");

    let handle = resolved
        .tree
        .nodes()
        .find_map(|(_, node)| {
            match node.fragment() {
                Fragment::Shape(shape) => shape.decoration,
                Fragment::Box(frame) => frame.decoration,
                _ => None,
            }
            .filter(|handle| PageCatalogue::is_chart_handle(handle.number()))
        })
        .expect("a chart issues decoration handles");
    assert_eq!(
        resolved.resources.decoration(handle),
        Resolved::Unanswerable(SceneLossKind::ChartNotResolved)
    );
}
