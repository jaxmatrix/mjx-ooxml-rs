//! A frame whose content this box model places and does not lay out is counted at the frame (MJXOFF-299).

mod support;

use mjx_layout::{BoxModel, FrameContent, LayoutLossKind, LayoutLosses, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideDeck};
use mjx_pptx::{Package, PartName, Presentation, SlideSize};

const NAMESPACES: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

// A one-slide deck whose shape tree is one graphic frame framing data of `uri`.
fn framing(uri: &str) -> Vec<u8> {
    let mut blank = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    blank.add_slide_from_layout(0).expect("one slide");
    let mut package = Package::open(&blank.save().expect("it saves")).expect("it reopens");
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {NAMESPACES}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="Frame"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="914400" y="914400"/><a:ext cx="3657600" cy="2743200"/></p:xfrm><a:graphic><a:graphicData uri="{uri}"/></a:graphic></p:graphicFrame></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    );
    package
        .replace_part_bytes(
            &PartName::new("/ppt/slides/slide1.xml").expect("a part name"),
            slide.into_bytes(),
        )
        .expect("the slide is replaceable");
    package.save().expect("the deck saves")
}

// The losses of slide 0 of `bytes`.
fn losses(bytes: &[u8]) -> LayoutLosses {
    let mut presentation = Presentation::open(bytes).expect("the deck opens");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let mut model = support::model();
    model
        .layout_page(&deck, PageIndex::FIRST, &constraints_for(&deck), None)
        .expect("the slide lays out")
        .losses()
        .clone()
}

#[test]
fn a_diagram_an_embedded_object_and_an_unknown_frame_are_each_counted_once() {
    let frame = LayoutLossKind::FrameContentNotLaidOut;
    for (uri, kind) in [
        (
            "http://schemas.openxmlformats.org/drawingml/2006/diagram",
            frame(FrameContent::Diagram),
        ),
        (
            "http://schemas.openxmlformats.org/presentationml/2006/ole",
            frame(FrameContent::EmbeddedObject),
        ),
        ("urn:test", LayoutLossKind::DroppedByReader),
    ] {
        let found = losses(&framing(uri));
        assert_eq!((found.count(kind), found.len()), (1, 1), "{uri}");
        let loss = found.iter().next().expect("one loss");
        assert_eq!(
            loss.source.path().segments(),
            &[0, 0],
            "{uri}: the frame's own address"
        );
        assert!(loss.area.is_some(), "{uri}: a frame occupies its own box");
    }
}

#[test]
fn an_ink_part_no_tier_places_is_counted_with_no_area() {
    let found = losses(&support::fixture("ink.pptx"));
    let ink = LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink);
    assert_eq!(
        (found.count(ink), found.len()),
        (1, 1),
        "`ink.pptx` references one InkML part from a `p:contentPart` that states no transform"
    );
    let loss = found.iter().next().expect("one loss");
    assert_eq!(
        loss.source.path().segments(),
        &[0, 1],
        "the content part is the slide's second shape"
    );
    assert!(
        loss.area.is_none(),
        "an unplaced part has no box to draw over"
    );
}

#[test]
fn a_slide_of_text_loses_nothing() {
    let (mut deck, slide) = support::blank_deck();
    support::text_box(
        &mut deck,
        slide,
        "Nothing is framed here.",
        mjx_pptx::ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.0),
    );
    let bytes = deck.save().expect("the deck saves");
    assert!(losses(&bytes).is_empty());
}
