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
