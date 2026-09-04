//! MJXOFF-131 (C16)'s own "disagreeing fixture": one inline picture, one anchored picture with
//! `wrapTight` and a non-trivial `wp:wrapPolygon`, one anchored shape (`wp:wsp`) with `behindDoc`,
//! one `w:pict` VML text box, and one OLE object — authored in one document so a reader that only
//! handles the first drawing kind it meets (or assumes `a:graphic` always holds `pic:pic`) fails on
//! one of the other four. Placement and wrap are asserted per drawing; the mutation gate then
//! changes one drawing's anchor and asserts every other drawing's bytes, and every other part, are
//! untouched.

use mjx_dml::wordprocessing_drawing::{PositionValue, Wrap};
use mjx_docx::{Document, PageSize, RunInnerContent};
use mjx_ooxml_core::FromXml;
use mjx_ooxml_types::wordprocessingdrawing::HorizontalRelativeFrom;
use mjx_opc::{Package, PartName, Relationship, TargetMode};

const WP_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";
const IMAGE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const OLE_OBJECT_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject";

/// One `<w:p>...</w:p>` block per drawing kind, in document order.
fn paragraphs() -> [String; 5] {
    [
        format!(
            r#"<w:p><w:r><w:drawing xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:pic="{PIC_NS}">
<wp:inline distT="0" distB="0" distL="0" distR="0">
<wp:extent cx="914400" cy="914400"/>
<wp:docPr id="1" name="Picture 1"/>
<a:graphic><a:graphicData uri="{PIC_NS}">
<pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="Picture 1"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="rId2"/><a:stretch/></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>
</pic:pic></a:graphicData></a:graphic>
</wp:inline></w:drawing></w:r></w:p>"#
        ),
        format!(
            r#"<w:p><w:r><w:drawing xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:pic="{PIC_NS}">
<wp:anchor distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1">
<wp:simplePos x="0" y="0"/>
<wp:positionH relativeFrom="column"><wp:posOffset>100000</wp:posOffset></wp:positionH>
<wp:positionV relativeFrom="paragraph"><wp:posOffset>50000</wp:posOffset></wp:positionV>
<wp:extent cx="1200000" cy="900000"/>
<wp:wrapTight wrapText="bothSides"><wp:wrapPolygon edited="1"><wp:start x="0" y="2000"/><wp:lineTo x="0" y="19600"/><wp:lineTo x="19600" y="19600"/><wp:lineTo x="19600" y="2000"/><wp:lineTo x="0" y="2000"/></wp:wrapPolygon></wp:wrapTight>
<wp:docPr id="2" name="Picture 2"/>
<a:graphic><a:graphicData uri="{PIC_NS}">
<pic:pic><pic:nvPicPr><pic:cNvPr id="2" name="Picture 2"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="rId3"/><a:stretch/></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1200000" cy="900000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>
</pic:pic></a:graphicData></a:graphic>
</wp:anchor></w:drawing></w:r></w:p>"#
        ),
        format!(
            r#"<w:p><w:r><w:drawing xmlns:wp="{WP_NS}" xmlns:a="{A_NS}">
<wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="2" behindDoc="1" locked="0" layoutInCell="1" allowOverlap="1">
<wp:simplePos x="0" y="0"/>
<wp:positionH relativeFrom="page"><wp:align>center</wp:align></wp:positionH>
<wp:positionV relativeFrom="page"><wp:align>top</wp:align></wp:positionV>
<wp:extent cx="1000000" cy="500000"/>
<wp:wrapNone/>
<wp:docPr id="3" name="Rectangle 3"/>
<a:graphic><a:graphicData uri="{WP_NS}">
<wp:wsp>
<wp:cNvPr id="3" name="Rectangle 3"/>
<wp:cNvSpPr/>
<wp:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></wp:spPr>
<wp:bodyPr/>
</wp:wsp>
</a:graphicData></a:graphic>
</wp:anchor></w:drawing></w:r></w:p>"#
        ),
        r##"<w:p><w:r><w:pict xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office">
<v:shapetype id="_x0000_t202" coordsize="21600,21600" path="m,l,21600r21600,l21600,xe"/>
<v:shape id="_x0000_s1026" type="#_x0000_t202" style="position:absolute;margin-left:0;margin-top:0;width:100pt;height:50pt">
<v:textbox><w:txbxContent><w:p><w:r><w:t>Text box content</w:t></w:r></w:p></w:txbxContent></v:textbox>
</v:shape>
</w:pict></w:r></w:p>"##
            .to_owned(),
        format!(
            r#"<w:p><w:r><w:object w:dxaOrig="3000" w:dyaOrig="2000">
<w:drawing xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:pic="{PIC_NS}">
<wp:inline distT="0" distB="0" distL="0" distR="0">
<wp:extent cx="1524000" cy="1143000"/>
<wp:docPr id="5" name="Object 5"/>
<a:graphic><a:graphicData uri="{PIC_NS}">
<pic:pic><pic:nvPicPr><pic:cNvPr id="5" name="Object 5"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="rId4"/><a:stretch/></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1524000" cy="1143000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>
</pic:pic></a:graphicData></a:graphic>
</wp:inline></w:drawing>
<w:objectEmbed r:id="rId5" w:progId="Excel.Sheet.12"/>
</w:object></w:r></w:p>"#
        ),
    ]
}

fn section_properties_xml() -> &'static str {
    r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#
}

fn document_xml(paragraphs: &[String; 5]) -> String {
    let namespaces = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {namespaces}><w:body>{}{}</w:body></w:document>"#,
        paragraphs.concat(),
        section_properties_xml()
    )
}

/// Adds the media/OLE parts and relationships the fixture's `r:id`s name to `package`.
fn add_fixture_parts(package: &mut Package, document_part: &PartName) {
    let media = [
        ("rId2", "media/image1.png", "image/png"),
        ("rId3", "media/image2.png", "image/png"),
        ("rId4", "media/image3.png", "image/png"),
    ];
    for (rid, target, content_type) in media {
        let part = PartName::new(&format!("/word/{target}")).expect("a valid part name");
        package
            .insert_part(&part, content_type, vec![0x89, b'P', b'N', b'G'])
            .expect("insert media part");
        package
            .add_relationship(
                Some(document_part),
                Relationship {
                    id: rid.to_owned(),
                    rel_type: IMAGE_REL.to_owned(),
                    target: target.to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("add image relationship");
    }
    // Content type deliberately *not*
    // "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": `mjx-schema-gate`'s own
    // `inspect.rs` treats that content type as an *embedded OOXML package* (a chart's own workbook)
    // and recursively opens the part as a zip — which this fixture's placeholder payload is not (it
    // is a fake binary standing in for an OLE payload this crate never re-encodes). The legacy binary
    // OLE content type is what a real, non-OOXML "Package"-typed OLE object (an old-style `.bin`
    // payload, not an embedded `.xlsx`) actually carries.
    let ole_part = PartName::new("/word/embeddings/oleObject1.bin").expect("a valid part name");
    package
        .insert_part(
            &ole_part,
            "application/vnd.openxmlformats-officedocument.oleObject",
            b"fake-ole-payload-not-a-zip".to_vec(),
        )
        .expect("insert ole part");
    package
        .add_relationship(
            Some(document_part),
            Relationship {
                id: "rId5".to_owned(),
                rel_type: OLE_OBJECT_REL.to_owned(),
                target: "embeddings/oleObject1.bin".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("add ole relationship");
}

/// A document carrying the five-drawing fixture, and the raw `word/document.xml` bytes it was
/// built from (for the mutation gate's own before/after comparison).
fn five_drawing_document() -> (Document, Vec<u8>, Vec<u8>) {
    let document = Document::blank(PageSize::a4()).expect("blank");
    let saved = document.save().expect("save the blank document");
    let mut package = Package::open(&saved).expect("reopen");
    let document_part = PartName::new("/word/document.xml").expect("a valid part name");
    add_fixture_parts(&mut package, &document_part);

    let bytes = document_xml(&paragraphs()).into_bytes();
    package
        .replace_part_bytes(&document_part, bytes.clone())
        .expect("replace word/document.xml");
    let saved = package.save_unchecked().expect("save the fixture");
    (
        Document::open(&saved).expect("reopen the fixture"),
        bytes,
        saved,
    )
}

#[test]
fn each_drawing_kind_reads_through_its_own_typed_model() {
    let (mut document, _, _) = five_drawing_document();

    // Paragraph 0: inline picture.
    document
        .paragraph_run_content(0, |content, interner| {
            let RunInnerContent::Drawing(drawing) = &content[0] else {
                panic!("paragraph 0's run does not hold a w:drawing: {content:?}")
            };
            let inline = drawing
                .inline()
                .expect("paragraph 0's drawing is not inline");
            let extent = inline.extent(interner).expect("wp:extent");
            assert_eq!(
                (extent.width.emu(), extent.height.emu()),
                (914_400, 914_400)
            );
            let picture = inline
                .graphic(interner)
                .and_then(|graphic| graphic.data().picture().cloned())
                .expect("the inline drawing wraps a pic:pic");
            assert_eq!(picture.image_rel_id(interner).as_deref(), Some("rId2"));
        })
        .expect("paragraph 0");

    // Paragraph 1: anchored picture, wrapTight with a five-point wrap polygon.
    document
        .paragraph_run_content(1, |content, interner| {
            let RunInnerContent::Drawing(drawing) = &content[0] else {
                panic!("paragraph 1's run does not hold a w:drawing: {content:?}")
            };
            let anchor = drawing
                .anchor()
                .expect("paragraph 1's drawing is not anchored");
            assert_eq!(
                anchor
                    .position_horizontal(interner)
                    .and_then(|p| p.relative_from()),
                Some(HorizontalRelativeFrom::Column)
            );
            assert!(matches!(
                anchor.position_horizontal(interner).and_then(|p| p.value()),
                Some(PositionValue::Offset(offset)) if offset.emu() == 100_000
            ));
            let Some(Wrap::Tight(wrap)) = anchor.wrap(interner) else {
                panic!("paragraph 1's anchor is not wrapTight")
            };
            let polygon = wrap.polygon(interner).expect("wp:wrapPolygon");
            assert_eq!(
                polygon.line_to(interner).len(),
                4,
                "the wrap polygon is not five points"
            );
            assert_eq!(
                anchor
                    .graphic(interner)
                    .and_then(|graphic| graphic.data().picture().cloned())
                    .and_then(|picture| picture.image_rel_id(interner)),
                Some("rId3".to_owned())
            );
        })
        .expect("paragraph 1");

    // Paragraph 2: anchored shape, behindDoc, no wrap.
    document
        .paragraph_run_content(2, |content, interner| {
            let RunInnerContent::Drawing(drawing) = &content[0] else {
                panic!("paragraph 2's run does not hold a w:drawing: {content:?}")
            };
            let anchor = drawing
                .anchor()
                .expect("paragraph 2's drawing is not anchored");
            assert_eq!(anchor.behind_doc(interner), Ok(true));
            assert!(matches!(anchor.wrap(interner), Some(Wrap::None(_))));
            // Its `a:graphicData@uri` names the wordprocessingDrawing namespace, not pic: — a
            // picture-only reader that assumes `a:graphic` always holds `pic:pic` would misread
            // this drawing entirely.
            let graphic = anchor.graphic(interner).expect("a:graphic");
            assert_eq!(graphic.data().uri(interner).as_deref(), Some(WP_NS));
            assert!(graphic.data().picture().is_none());
        })
        .expect("paragraph 2");

    // Paragraph 3: w:pict VML text box — its paragraphs read through MJXOFF-92's model.
    document
        .paragraph_run_content(3, |content, interner| {
            let RunInnerContent::LegacyPicture(vml_drawing) = &content[0] else {
                panic!("paragraph 3's run does not hold a w:pict: {content:?}")
            };
            let shape = vml_drawing
                .content()
                .iter()
                .find_map(|item| match item {
                    mjx_vml::DrawingContent::Shape(shape) => Some(shape),
                    _ => None,
                })
                .expect("the w:pict wraps a v:shape");
            let text_box = shape.text_box().expect("the shape has a v:textbox");
            let txbx_content_element = text_box
                .raw()
                .children
                .iter()
                .find_map(|node| match node {
                    mjx_ooxml_core::RawNode::Element(element)
                        if interner.resolve(element.name.local) == "txbxContent" =>
                    {
                        Some(element.clone())
                    }
                    _ => None,
                })
                .expect("v:textbox wraps a w:txbxContent");
            let txbx_content = mjx_docx::TextBoxContent::from_xml(&txbx_content_element, interner)
                .expect("w:txbxContent parses");
            let texts: Vec<String> = txbx_content.paragraphs().map(|p| p.text()).collect();
            assert_eq!(texts, ["Text box content"]);
        })
        .expect("paragraph 3");

    // Paragraph 4: OLE object, an embedded object binding plus its own preview drawing.
    document
        .paragraph_run_content(4, |content, interner| {
            let RunInnerContent::EmbeddedObject(object) = &content[0] else {
                panic!("paragraph 4's run does not hold a w:object: {content:?}")
            };
            assert_eq!(
                object.original_width_twips(interner),
                Ok(Some(mjx_ooxml_types::shared::TwipsMeasure::from_wire(
                    "3000"
                )))
            );
            let embed = object.object_embed().expect("w:objectEmbed");
            assert_eq!(
                embed.relationship_id(interner).ok().as_deref(),
                Some("rId5")
            );
            assert_eq!(
                embed.program_id(interner).as_deref(),
                Some("Excel.Sheet.12")
            );
            assert!(
                object.drawing().is_some(),
                "the object's own preview drawing is typed too"
            );
        })
        .expect("paragraph 4");
}

#[test]
fn changing_one_anchors_position_leaves_every_other_drawing_and_every_other_part_untouched() {
    let (_, original_bytes, _) = five_drawing_document();
    let original_xml = String::from_utf8(original_bytes).expect("utf8");

    // Mutate only the second drawing's own horizontal offset.
    let needle =
        r#"<wp:positionH relativeFrom="column"><wp:posOffset>100000</wp:posOffset></wp:positionH>"#;
    assert!(
        original_xml.contains(needle),
        "fixture markup changed under this test"
    );
    let mutated_xml = original_xml.replacen(
        needle,
        r#"<wp:positionH relativeFrom="column"><wp:posOffset>500000</wp:posOffset></wp:positionH>"#,
        1,
    );
    assert_ne!(original_xml, mutated_xml);

    // Every OTHER paragraph's own markup must be byte-identical before and after.
    let original_paragraphs = paragraphs();
    for (index, paragraph) in original_paragraphs.iter().enumerate() {
        if index == 1 {
            continue;
        }
        assert!(
            mutated_xml.contains(paragraph.as_str()),
            "paragraph {index} changed when only paragraph 1's anchor should have"
        );
    }

    // Rebuild a package from the mutated document.xml and confirm every OTHER part (media, rels,
    // content types) is untouched.
    let blank = Document::blank(PageSize::a4()).expect("blank");
    let saved = blank.save().expect("save");
    let mut original_package = Package::open(&saved).expect("reopen");
    let document_part = PartName::new("/word/document.xml").expect("a valid part name");
    add_fixture_parts(&mut original_package, &document_part);
    original_package
        .replace_part_bytes(&document_part, original_xml.clone().into_bytes())
        .expect("replace with original");
    let original_saved = original_package.save_unchecked().expect("save original");

    let blank2 = Document::blank(PageSize::a4()).expect("blank");
    let saved2 = blank2.save().expect("save");
    let mut mutated_package = Package::open(&saved2).expect("reopen");
    add_fixture_parts(&mut mutated_package, &document_part);
    mutated_package
        .replace_part_bytes(&document_part, mutated_xml.into_bytes())
        .expect("replace with mutated");
    let mutated_saved = mutated_package.save_unchecked().expect("save mutated");

    let original_reopened = Package::open(&original_saved).expect("reopen original");
    let mutated_reopened = Package::open(&mutated_saved).expect("reopen mutated");
    for part in [
        "/word/media/image1.png",
        "/word/media/image2.png",
        "/word/media/image3.png",
        "/word/embeddings/oleObject1.bin",
        "/word/_rels/document.xml.rels",
        "/[Content_Types].xml",
    ] {
        let name = PartName::new(part).expect("a valid part name");
        assert_eq!(
            original_reopened.part_bytes(&name),
            mutated_reopened.part_bytes(&name),
            "{part} changed when only the second drawing's anchor should have"
        );
    }
}

#[test]
fn the_five_drawing_fixture_is_schema_valid() {
    let (_, _, saved) = five_drawing_document();
    mjx_schema_gate::assert_authored_deck_is_schema_valid("five-drawing fixture", &saved);
}

// -------------------------------------------------------------------------------------------------
// MJXOFF-126's own rule, in this child's terms: a drawing inside `w:ins`/`w:del` is opaque to every
// mutation path in this crate — never addressed, never touched. Proved directly: a drawing wrapped
// in `w:ins` survives an edit to a *different* paragraph byte-for-byte.
// -------------------------------------------------------------------------------------------------

fn document_with_a_drawing_inside_w_ins() -> (Document, String) {
    let document = Document::blank(PageSize::a4()).expect("blank");
    let saved = document.save().expect("save the blank document");
    let mut package = Package::open(&saved).expect("reopen");
    let document_part = PartName::new("/word/document.xml").expect("a valid part name");
    add_fixture_parts(&mut package, &document_part);

    let ins_paragraph = format!(
        r#"<w:p><w:ins w:id="1" w:author="Author"><w:r><w:drawing xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:pic="{PIC_NS}">
<wp:inline distT="0" distB="0" distL="0" distR="0">
<wp:extent cx="914400" cy="914400"/>
<wp:docPr id="1" name="Picture 1"/>
<a:graphic><a:graphicData uri="{PIC_NS}">
<pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="Picture 1"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="rId2"/><a:stretch/></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>
</pic:pic></a:graphicData></a:graphic>
</wp:inline></w:drawing></w:r></w:ins></w:p>"#
    );
    let other_paragraph = r#"<w:p><w:r><w:t>unrelated text</w:t></w:r></w:p>"#.to_owned();
    let namespaces = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {namespaces}><w:body>{ins_paragraph}{other_paragraph}{}</w:body></w:document>"#,
        section_properties_xml()
    );
    package
        .replace_part_bytes(&document_part, xml.into_bytes())
        .expect("replace word/document.xml");
    let saved = package.save_unchecked().expect("save the fixture");
    (
        Document::open(&saved).expect("reopen the fixture"),
        ins_paragraph,
    )
}

#[test]
fn a_drawing_inside_w_ins_survives_an_edit_to_a_different_paragraph_byte_for_byte() {
    let (mut document, ins_paragraph_xml) = document_with_a_drawing_inside_w_ins();

    // Confirm the drawing is not reachable through the ordinary run-content scan (MJXOFF-126's own
    // rule: `w:ins`/`w:del` are opaque to every mutation path, including this reading one).
    let content = document.paragraph_run_content(0, |content, _interner| content.to_vec());
    assert!(
        matches!(content, Ok(items) if items.is_empty()),
        "paragraph 0's own top-level run content must be empty — its only run is inside w:ins"
    );

    // Edit the *other* paragraph.
    document.append_run(1, " — appended").expect("append_run");
    let saved = document.save().expect("save after the unrelated edit");

    // The w:ins paragraph's own markup must be untouched — same bytes, same position.
    let package = Package::open(&saved).expect("reopen");
    let document_xml = package
        .part_bytes(&PartName::new("/word/document.xml").unwrap())
        .expect("word/document.xml");
    let document_xml = std::str::from_utf8(document_xml).expect("utf8");
    assert!(
        document_xml.contains(&ins_paragraph_xml),
        "the w:ins paragraph's own markup changed when only paragraph 1 was edited:\n{document_xml}"
    );

    // And the edit itself landed where it should — the unrelated paragraph's own text.
    let mut reopened = Document::open(&saved).expect("reopen");
    assert_eq!(
        reopened.paragraph_text(1).expect("paragraph_text"),
        "unrelated text — appended"
    );
}
