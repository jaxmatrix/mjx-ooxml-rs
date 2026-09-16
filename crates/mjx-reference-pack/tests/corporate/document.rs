//! The corporate document: a first page shaped like one somebody would actually send
//! (MJXOFF-300).

use mjx_docx::{
    AbstractNumbering, Color, Document, Fonts, HeaderFooterType, LevelNumberFormat,
    LevelTextTemplate, NumberingInstance, NumberingLevel, PageBorderSet, PageSize, RunProperties,
    SectionLocation, StyleDefinition, TableLook, TopPageBorder,
};
use mjx_omml::{Math, MathElement, Run as MathRun};
use mjx_ooxml_types::wordprocessingml::{
    BorderStyle, NumberFormat, StyleType, ThemeColor, TwoDigitHexadecimalNumber,
};
use mjx_opc::{Package, PartName};

use super::{part_text, relate, set_part_text, splice, LOGO_PNG};

/// The main part a blank document writes.
const DOCUMENT: &str = "/word/document.xml";

/// The theme, so a `w:themeColor` resolves to something. `Document::blank` writes none.
const THEME_PART: &str = "/word/theme/theme1.xml";
const THEME_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.theme+xml";
const THEME_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";
const IMAGE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// A tracked insertion and a tracked deletion, which no public writer can attach to a paragraph.
const TRACKED_CHANGES: &str = concat!(
    "<w:p><w:r><w:t xml:space=\"preserve\">Reviewed by the regional leads</w:t></w:r>",
    "<w:ins w:id=\"901\" w:author=\"Second Reviewer\" w:date=\"2026-01-05T11:00:00Z\">",
    "<w:r><w:t xml:space=\"preserve\"> and by finance</w:t></w:r></w:ins>",
    "<w:del w:id=\"902\" w:author=\"Second Reviewer\" w:date=\"2026-01-05T11:02:00Z\">",
    "<w:r><w:delText xml:space=\"preserve\"> pending audit</w:delText></w:r></w:del>",
    "</w:p>"
);

/// A floating picture. The namespaces are declared on the drawing, because the body's root
/// declares only `w:` and `r:`.
const ANCHORED_PICTURE: &str = concat!(
    "<w:p><w:r><w:drawing ",
    "xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" ",
    "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" ",
    "xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
    "<wp:anchor distT=\"0\" distB=\"0\" distL=\"114300\" distR=\"114300\" simplePos=\"0\" ",
    "relativeHeight=\"251658240\" behindDoc=\"0\" locked=\"0\" layoutInCell=\"1\" allowOverlap=\"1\">",
    "<wp:simplePos x=\"0\" y=\"0\"/>",
    "<wp:positionH relativeFrom=\"column\"><wp:posOffset>3200400</wp:posOffset></wp:positionH>",
    "<wp:positionV relativeFrom=\"paragraph\"><wp:posOffset>0</wp:posOffset></wp:positionV>",
    "<wp:extent cx=\"914400\" cy=\"457200\"/><wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/>",
    "<wp:wrapSquare wrapText=\"bothSides\"/>",
    "<wp:docPr id=\"910\" name=\"Floating logo\"/>",
    "<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
    "<pic:pic><pic:nvPicPr><pic:cNvPr id=\"911\" name=\"Floating logo\"/><pic:cNvPicPr/></pic:nvPicPr>",
    "<pic:blipFill><a:blip r:embed=\"rId900\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>",
    "<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"914400\" cy=\"457200\"/></a:xfrm>",
    "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>",
    "</a:graphicData></a:graphic></wp:anchor></w:drawing></w:r></w:p>"
);

/// A text box. VML, because that is the shape this workspace can read back end to end.
const TEXT_BOX: &str = concat!(
    "<w:p><w:r><w:pict xmlns:v=\"urn:schemas-microsoft-com:vml\" ",
    "xmlns:o=\"urn:schemas-microsoft-com:office:office\">",
    "<v:shapetype id=\"_x0000_t202\" coordsize=\"21600,21600\" path=\"m,l,21600r21600,l21600,xe\"/>",
    "<v:shape id=\"_x0000_s1026\" type=\"#_x0000_t202\" ",
    "style=\"position:absolute;margin-left:0;margin-top:0;width:180pt;height:54pt\">",
    "<v:textbox><w:txbxContent><w:p><w:r>",
    "<w:t>Pull quote: every region grew this quarter.</w:t>",
    "</w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p>"
);

/// The corporate document, as bytes.
pub(crate) fn corporate_document() -> Vec<u8> {
    let mut document = Document::blank(PageSize::a4()).expect("a blank document");
    author_styles(&mut document);
    author_text(&mut document);
    author_numbering(&mut document);
    author_table(&mut document);
    author_pictures_and_notes(&mut document);
    author_section(&mut document);
    let header = document
        .create_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("a default header");
    let saved = document.save().expect("the authored document saves");
    let package = splice_the_unwritable(saved, header.as_str());
    Document::from_package(package)
        .expect("the spliced document reopens")
        .save()
        .expect("the corporate document saves")
}

/// Four styles, two of which state a theme colour and a theme shade.
fn author_styles(document: &mut Document) {
    document
        .edit_style_sheet(|sheet, interner| {
            let styles = [
                ("CorporateTitle", StyleType::Paragraph, true),
                ("CorporateBody", StyleType::Paragraph, false),
                ("CorporateAccent", StyleType::Character, true),
                ("CorporateTable", StyleType::Table, false),
            ];
            for (id, kind, themed) in styles {
                let mut style = StyleDefinition::new(interner, kind, id);
                let mut color = Color::new(interner, "1F3864");
                if themed {
                    color.set_theme_color(interner, Some(ThemeColor::Accent1));
                    color.set_theme_shade(
                        interner,
                        Some(TwoDigitHexadecimalNumber("BF".to_owned())),
                    );
                }
                style
                    .run_properties_or_insert(interner)
                    .set_color(Some(color));
                sheet.add_style(style);
            }
        })
        .expect("the style sheet");
}

/// The body's paragraphs. The blank document's first one is empty, so it takes the title.
fn author_text(document: &mut Document) {
    document
        .append_run(0, "Quarterly Business Review")
        .expect("the title");
    // `append_paragraph` appends at the end and answers nothing, so the index is the count so far:
    // the blank document's own paragraph is 0, and these become 1 to 5.
    for (offset, text) in [
        "Every region grew this quarter, and the pipeline entering next quarter is healthy.",
        "Margin expanded",
        "Headcount flat",
        "The break-even point follows from the contribution margin:",
        "Regional detail is in the table below.",
    ]
    .into_iter()
    .enumerate()
    {
        document.append_paragraph().expect("a paragraph");
        document.append_run(offset + 1, text).expect("a run");
    }
}

/// A bulleted list whose bullets are drawn from Symbol.
fn author_numbering(document: &mut Document) {
    document
        .edit_numbering(|numbering, interner| {
            let mut abstract_numbering = AbstractNumbering::new(interner, 0);
            let mut level = NumberingLevel::new(interner, 0);
            level.set_start(interner, Some(1));
            level.set_format(Some(LevelNumberFormat::new(interner, NumberFormat::Bullet)));
            level.set_text_template(Some(LevelTextTemplate::new(interner, "\u{f0b7}")));
            let mut properties = RunProperties::new(interner);
            let mut fonts = Fonts::new(interner);
            fonts.set_ascii_font(interner, Some("Symbol"));
            properties.set_fonts(Some(fonts));
            level.set_run_properties(Some(properties));
            abstract_numbering.push_level(level);
            numbering.push_abstract_numbering(abstract_numbering);
            numbering.push_instance(NumberingInstance::new(interner, 1, 0));
        })
        .expect("the numbering definition");
    for paragraph in [2, 3] {
        document
            .attach_paragraph_to_list(paragraph, 1, 0)
            .expect("a list paragraph");
    }
}

/// A styled table with a banded header row.
fn author_table(document: &mut Document) {
    let table = document.append_table(3, 3).expect("the table");
    document
        .edit_table(table, |table, interner| {
            let properties = table
                .properties_mut()
                .expect("Table::new always writes a w:tblPr");
            properties.set_style_id(interner, Some("CorporateTable"));
            let mut look = TableLook::new(interner);
            look.set_first_row(interner, Some(true));
            look.set_no_horizontal_band(interner, Some(false));
            properties.set_look(Some(look));
        })
        .expect("the table's style");
    let rows = [
        ["Region", "Plan", "Actual"],
        ["EMEA", "1,204", "1,310"],
        ["APAC", "987", "1,002"],
    ];
    for (row, cells) in rows.iter().enumerate() {
        for (column, text) in cells.iter().enumerate() {
            document
                .set_cell_text(table, row, column, text)
                .expect("a table cell");
        }
    }
}

/// The inline picture, the equation and the footnote.
fn author_pictures_and_notes(document: &mut Document) {
    document
        .append_math(4, |interner| {
            // The run is built first: `with_elements` borrows the interner too.
            let run = MathRun::new(interner, "F = P / (1 - v)");
            Math::with_elements(interner, &[MathElement::Run(run)])
        })
        .expect("the equation");
    document
        .add_inline_picture(
            5,
            LOGO_PNG.to_vec(),
            "image/png",
            "png",
            914_400,
            457_200,
            "Inline logo",
        )
        .expect("the inline picture");
    document
        .add_footnote(5, "Figures are unaudited and subject to revision.")
        .expect("the footnote");
}

/// A page border on the section every page inherits.
fn author_section(document: &mut Document) {
    document
        .edit_section_properties(SectionLocation::Body, |properties, interner| {
            let mut borders = PageBorderSet::new(interner);
            borders.set_top(Some(TopPageBorder::new(interner, BorderStyle::Single)));
            properties.set_page_borders(Some(borders));
        })
        .expect("the page border");
}

/// The header's own content: a three-cell table and the logo.
fn header_markup(rel_id: &str) -> String {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
            "<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" ",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" ",
            "xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" ",
            "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" ",
            "xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
            "<w:tbl><w:tblPr><w:tblStyle w:val=\"CorporateTable\"/>",
            "<w:tblW w:w=\"0\" w:type=\"auto\"/>",
            "<w:tblLook w:val=\"04A0\" w:firstRow=\"1\" w:lastRow=\"0\" w:firstColumn=\"1\" ",
            "w:lastColumn=\"0\" w:noHBand=\"0\" w:noVBand=\"1\"/></w:tblPr>",
            "<w:tblGrid><w:gridCol w:w=\"3000\"/><w:gridCol w:w=\"3000\"/>",
            "<w:gridCol w:w=\"3000\"/></w:tblGrid>",
            "<w:tr><w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/></w:tcPr>",
            "<w:p><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">",
            "<wp:extent cx=\"457200\" cy=\"228600\"/>",
            "<wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/>",
            "<wp:docPr id=\"920\" name=\"Header logo\"/>",
            "<a:graphic><a:graphicData ",
            "uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
            "<pic:pic><pic:nvPicPr><pic:cNvPr id=\"921\" name=\"Header logo\"/>",
            "<pic:cNvPicPr/></pic:nvPicPr>",
            "<pic:blipFill><a:blip r:embed=\"{rel_id}\"/>",
            "<a:stretch><a:fillRect/></a:stretch></pic:blipFill>",
            "<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"457200\" cy=\"228600\"/></a:xfrm>",
            "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>",
            "</a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p></w:tc>",
            "<w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/></w:tcPr>",
            "<w:p><w:r><w:t>Quarterly Business Review</w:t></w:r></w:p></w:tc>",
            "<w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/></w:tcPr>",
            "<w:p><w:r><w:t>Internal</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
            "<w:p/></w:hdr>"
        ),
        rel_id = rel_id
    )
}

/// The five elements this workspace has a reader for and no writer, plus the theme part.
fn splice_the_unwritable(saved: Vec<u8>, header: &str) -> Package {
    let mut package = Package::open(&saved).expect("the authored document reopens");

    // The image the inline picture already stored, reused by the header and the floating picture.
    let media = package
        .part_names()
        .find(|name| name.as_str().starts_with("/word/media/"))
        .expect("the inline picture stored an image part");
    let media_target = media
        .as_str()
        .strip_prefix("/word/")
        .expect("the media part sits under /word/")
        .to_owned();

    let theme = PartName::new(THEME_PART).expect("a part name");
    package
        .insert_part(&theme, THEME_CONTENT_TYPE, mjx_dml::default_theme_xml())
        .expect("the theme part");
    relate(
        &mut package,
        DOCUMENT,
        THEME_REL,
        "rId800",
        "theme/theme1.xml",
    );
    relate(&mut package, DOCUMENT, IMAGE_REL, "rId900", &media_target);
    relate(&mut package, header, IMAGE_REL, "rId901", &media_target);

    set_part_text(&mut package, header, header_markup("rId901"));

    let mut body = part_text(&package, DOCUMENT);
    splice(
        &mut body,
        "<w:body>",
        &format!("<w:body>{TRACKED_CHANGES}{ANCHORED_PICTURE}{TEXT_BOX}"),
    );
    set_part_text(&mut package, DOCUMENT, body);

    package
}
