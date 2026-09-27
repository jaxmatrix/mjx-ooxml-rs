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
        &[0, u32::MAX, 3],
        "the wrapper is the fourth child element of the slide's shape tree"
    );
    assert!(
        loss.area.is_none(),
        "an unplaced part has no box to draw over"
    );
    let (surface, element) = mjx_layout_pptx::address::wrapped_element(&loss.source)
        .expect("the address names an element outside the shape index space");
    let mut presentation =
        Presentation::open(&support::fixture("ink.pptx")).expect("the deck opens");
    let reference = presentation
        .ink_references(surface)
        .expect("the references read")
        .into_iter()
        .find(|reference| reference.element_index == element)
        .expect("the address resolves to the ink reference");
    assert_eq!(
        (
            reference.shape_index,
            reference.part.as_ref().map(|part| part.as_str().to_owned())
        ),
        (None, Some("/ppt/ink/ink1.xml".to_owned())),
        "the element the address names is the wrapped content part that reaches the ink"
    );
}

// `ink.pptx` with its wrapped content part given a transform of 3 by 2 inches at one inch in.
fn placed_wrapped_ink() -> Vec<u8> {
    let mut package = Package::open(&support::fixture("ink.pptx")).expect("the fixture opens");
    let name = PartName::new("/ppt/slides/slide1.xml").expect("a part name");
    let slide = String::from_utf8(package.part_bytes(&name).expect("the slide").to_vec())
        .expect("the slide is UTF-8");
    let placed = slide.replace(
        r#"<p:contentPart r:id="rId2"/>"#,
        r#"<p:contentPart r:id="rId2"><p14:xfrm xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main"><a:off x="914400" y="914400"/><a:ext cx="2743200" cy="1828800"/></p14:xfrm></p:contentPart>"#,
    );
    assert_ne!(slide, placed, "the fixture's content part is rewritten");
    package
        .replace_part_bytes(&name, placed.into_bytes())
        .expect("the slide is replaceable");
    package.save().expect("the deck saves")
}

#[test]
fn ink_wrapped_in_alternate_content_is_counted_over_its_own_box() {
    let found = losses(&placed_wrapped_ink());
    let ink = LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink);
    assert_eq!((found.count(ink), found.len()), (1, 1));
    let loss = found.iter().next().expect("one loss");
    let area = loss.area.expect("a placed part has a box to draw over");
    assert_eq!(
        (
            area.rect.left.emu(),
            area.rect.top.emu(),
            area.rect.width().emu(),
            area.rect.height().emu()
        ),
        (914400, 914400, 2743200, 1828800)
    );
    assert_eq!(loss.source.path().segments(), &[0, u32::MAX, 3]);
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
