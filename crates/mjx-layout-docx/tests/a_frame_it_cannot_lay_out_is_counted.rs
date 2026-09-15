//! A drawing whose content this box model lays out as an empty frame is counted at the frame (MJXOFF-299).

mod support;

use mjx_layout::{BoxModel, FrameContent, LayoutLossKind, LayoutLosses, PageIndex};

// A paragraph holding one inline drawing whose graphic data names `uri`.
fn inline_drawing(uri: &str) -> String {
    format!(
        r#"<w:p{namespaces}><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="914400" cy="914400"/><wp:docPr id="1" name="Object"/><a:graphic><a:graphicData uri="{uri}"/></a:graphic></wp:inline></w:drawing></w:r></w:p>"#,
        namespaces = support::DRAWING_NAMESPACES
    )
}

// The losses of the first page of a document of `paragraphs`.
fn losses(paragraphs: &[String]) -> LayoutLosses {
    let mut document = support::document(paragraphs);
    let flow = support::flow(&mut document);
    let mut model = support::model();
    model
        .layout_page(
            &flow,
            PageIndex::FIRST,
            &support::constraints(6.5, 9.0),
            None,
        )
        .expect("the page lays out")
        .losses()
        .clone()
}

#[test]
fn each_kind_of_framed_content_is_counted_as_itself() {
    let frame = LayoutLossKind::FrameContentNotLaidOut;
    for (uri, kind) in [
        (
            "http://schemas.openxmlformats.org/drawingml/2006/diagram",
            frame(FrameContent::Diagram),
        ),
        (
            "http://schemas.openxmlformats.org/drawingml/2006/picture",
            frame(FrameContent::Picture),
        ),
        (
            "http://schemas.openxmlformats.org/presentationml/2006/ole",
            frame(FrameContent::EmbeddedObject),
        ),
        (
            "http://schemas.microsoft.com/office/word/2010/wordprocessingInk",
            frame(FrameContent::Ink),
        ),
        ("urn:test", LayoutLossKind::DroppedByReader),
    ] {
        let found = losses(&[inline_drawing(uri)]);
        assert_eq!(
            (found.count(kind), found.len()),
            (1, 1),
            "{uri} is one loss of {kind:?}"
        );
        assert!(
            found.iter().all(|loss| loss.area.is_some()),
            "{uri}: a frame occupies its own box"
        );
    }
}

#[test]
fn a_document_of_text_loses_nothing() {
    assert!(losses(&[support::paragraph("", "Nothing is framed here.")]).is_empty());
}

// The losses of the first page of `document`.
fn losses_of(mut document: mjx_docx::Document) -> LayoutLosses {
    let flow = support::flow(&mut document);
    let mut model = support::model();
    model
        .layout_page(
            &flow,
            PageIndex::FIRST,
            &support::constraints(6.5, 9.0),
            None,
        )
        .expect("the page lays out")
        .losses()
        .clone()
}

// A paragraph carrying one floating drawing whose graphic data names `uri`, then some text.
fn floating_drawing(uri: &str) -> String {
    support::floating_paragraph(
        "",
        "Text beside the frame.",
        914400,
        914400,
        &support::offset_position(0, 0),
        &support::square_wrap("bothSides"),
    )
    .replace("urn:test", uri)
}

#[test]
fn a_floating_frame_is_counted_as_its_content_at_its_own_box() {
    let frame = LayoutLossKind::FrameContentNotLaidOut;
    for (uri, kind) in [
        (
            "http://schemas.openxmlformats.org/drawingml/2006/diagram",
            frame(FrameContent::Diagram),
        ),
        (
            "http://schemas.openxmlformats.org/drawingml/2006/picture",
            frame(FrameContent::Picture),
        ),
        ("urn:test", LayoutLossKind::DroppedByReader),
    ] {
        let found = losses(&[floating_drawing(uri)]);
        assert_eq!(
            (found.count(kind), found.len()),
            (1, 1),
            "{uri} floating is one loss of {kind:?}"
        );
        let area = found
            .iter()
            .next()
            .and_then(|loss| loss.area)
            .expect("a floating frame occupies its own box");
        assert_eq!(
            (area.rect.width().emu(), area.rect.height().emu()),
            (914400, 914400),
            "{uri}: the placeholder covers the float's own extent"
        );
    }
}

// `chart_in_word.docx` with its one inline chart turned into a floating one.
fn floating_chart() -> mjx_docx::Document {
    let mut package = mjx_docx::Package::open(&mjx_fixtures::fixture("chart_in_word.docx"))
        .expect("the fixture opens");
    let name = mjx_docx::PartName::new(support::DOCUMENT_PART).expect("a part name");
    let inline = String::from_utf8(
        package
            .part_bytes(&name)
            .expect("the main document part")
            .to_vec(),
    )
    .expect("the part is UTF-8");
    let floating = inline
        .replace(
            r#"<wp:inline distT="0" distR="0" distB="0" distL="0"><wp:extent cx="4572000" cy="2857500"/>"#,
            &format!(
                r#"<wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/>{}<wp:extent cx="4572000" cy="2857500"/><wp:effectExtent l="0" t="0" r="0" b="0"/>{}"#,
                support::offset_position(0, 0),
                support::square_wrap("bothSides")
            ),
        )
        .replace("</wp:inline>", "</wp:anchor>");
    assert_ne!(inline, floating, "the fixture's inline chart is rewritten");
    package
        .replace_part_bytes(&name, floating.into_bytes())
        .expect("the part is replaceable");
    mjx_docx::Document::from_package(package).expect("the floating chart opens")
}

#[test]
fn a_floating_chart_that_lays_out_is_measured_not_shaped_and_no_frame_loss() {
    let found = losses_of(floating_chart());
    assert_eq!(
        found.count(LayoutLossKind::FrameContentNotLaidOut(FrameContent::Chart)),
        0,
        "a chart the engine lays out is not a frame not laid out"
    );
    assert_eq!(
        (
            found.count(LayoutLossKind::TextMeasuredNotShaped),
            found.len()
        ),
        (1, 1),
        "its text is measured with nominal metrics, once"
    );
    assert!(
        found.iter().all(|loss| loss.area.is_none()),
        "an approximation draws no placeholder"
    );
}
