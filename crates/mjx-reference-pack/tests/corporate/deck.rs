//! The corporate deck: one slide shaped like a page somebody would actually send (MJXOFF-300).

use mjx_chart::{ChartData, ChartKind};
use mjx_dml::{
    BulletTypeface, CharacterPropertiesSpec, ColorSpec, CustomGeometrySpec, DrawCommand, Emu,
    FillSpec, Fraction, LineEnd, LineSpec, LineWidth, ParagraphPropertiesSpec, Path2DSpec,
    PathFillMode, Point, SchemeColor, ShapeGeometry,
};
use mjx_ooxml_types::drawingml::{LineEndType, PresetShapeType};
use mjx_pptx::{DiagramContent, Geometry, Package, Presentation, ShapeBounds, SlideSize, Surface};

use super::{add_related_part, part_text, set_part_text, splice, LOGO_PNG};

/// The slide and master parts a blank deck writes.
const SLIDE: &str = "/ppt/slides/slide1.xml";
const MASTER: &str = "/ppt/slideMasters/slideMaster1.xml";

/// The title's first run, which the soft break follows.
const TITLE_HEAD: &str = "Quarterly review";
/// The whole title. One run until `set_text_range_properties` splits it in two.
const TITLE: &str = "Quarterly review across EMEA and APAC";

/// The master's gradient background. `p:bg` is `p:cSld`'s first child, before `p:spTree`.
const BACKGROUND: &str = concat!(
    "<p:bg><p:bgPr><a:gradFill rotWithShape=\"1\"><a:gsLst>",
    "<a:gs pos=\"0\"><a:schemeClr val=\"bg1\"/></a:gs>",
    "<a:gs pos=\"100000\"><a:schemeClr val=\"accent1\"><a:lumMod val=\"40000\"/>",
    "<a:lumOff val=\"60000\"/></a:schemeClr></a:gs>",
    "</a:gsLst><a:lin ang=\"5400000\" scaled=\"0\"/></a:gradFill><a:effectLst/></p:bgPr></p:bg>"
);

/// The crop on the first picture. `a:srcRect` sits between `a:blip` and `a:stretch`.
const CROP: &str = "<a:srcRect l=\"8000\" t=\"6000\" r=\"12000\" b=\"9000\"/>";

/// The footer's slide-number and date fields.
const FIELDS: &str = concat!(
    "<p:sp><p:nvSpPr><p:cNvPr id=\"800\" name=\"Footer fields\"/>",
    "<p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>",
    "<p:spPr><a:xfrm><a:off x=\"548640\" y=\"6217920\"/><a:ext cx=\"5486400\" cy=\"365760\"/></a:xfrm>",
    "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>",
    "<p:txBody><a:bodyPr/><a:lstStyle/><a:p>",
    "<a:fld id=\"{5BCAD085-E8A6-8845-BD4E-CB4CCA059FC4}\" type=\"datetimeFigureOut\">",
    "<a:rPr lang=\"en-US\"/><a:t>5 January 2026</a:t></a:fld>",
    "<a:r><a:rPr lang=\"en-US\"/><a:t> | Corporate review | slide </a:t></a:r>",
    "<a:fld id=\"{C1FF6DA9-008F-8B48-92A6-B652298478BF}\" type=\"slidenum\">",
    "<a:rPr lang=\"en-US\"/><a:t>1</a:t></a:fld>",
    "</a:p></p:txBody></p:sp>"
);

/// The connector. Its arrowhead is written afterwards by `set_shape_outline`, which is typed.
const CONNECTOR: &str = concat!(
    "<p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"810\" name=\"Arrow connector\"/>",
    "<p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>",
    "<p:spPr><a:xfrm><a:off x=\"5715000\" y=\"3108960\"/><a:ext cx=\"1600200\" cy=\"0\"/></a:xfrm>",
    "<a:prstGeom prst=\"straightConnector1\"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>"
);

/// The SmartArt frame's cached drawing, which `add_diagram` deliberately does not write.
const CACHED_DRAWING: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
    "<dsp:drawing xmlns:dsp=\"http://schemas.microsoft.com/office/drawing/2008/diagram\" ",
    "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
    "<dsp:spTree><dsp:nvGrpSpPr><dsp:cNvPr id=\"0\" name=\"\"/><dsp:cNvGrpSpPr/></dsp:nvGrpSpPr>",
    "<dsp:grpSpPr/></dsp:spTree></dsp:drawing>"
);

/// The cached drawing's content type and relationship type, as Office writes them.
const DRAWING_CONTENT_TYPE: &str = "application/vnd.ms-office.drawingml.diagramDrawing+xml";
const DRAWING_REL: &str = "http://schemas.microsoft.com/office/2007/relationships/diagramDrawing";

/// The corporate deck, as bytes.
pub(crate) fn corporate_deck() -> Vec<u8> {
    let mut deck = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    deck.add_slide_from_layout(0).expect("one slide");
    author_master(&mut deck);
    author_slide(&mut deck);
    let saved = deck.save().expect("the authored deck saves");
    finish(splice_the_unwritable(saved))
}

/// The furniture every slide inherits: a logo picture and an accent band.
fn author_master(deck: &mut Presentation) {
    deck.add_picture(
        Surface::Master(0),
        LOGO_PNG,
        ShapeBounds::from_inches(11.9, 0.3, 0.8, 0.5),
    )
    .expect("the master's logo");
    let band = deck
        .add_shape(
            Surface::Master(0),
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(0.0, 6.95, 13.333, 0.55),
        )
        .expect("the master's band");
    deck.set_shape_fill(
        Surface::Master(0),
        band,
        &FillSpec::Solid(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("the band's fill");
}

/// The slide itself, in paint order.
fn author_slide(deck: &mut Presentation) {
    let title = deck
        .add_text_box(0, TITLE, ShapeBounds::from_inches(0.6, 0.35, 9.0, 1.2))
        .expect("the title");
    deck.set_shape_run_properties(
        0,
        title,
        &CharacterPropertiesSpec::new()
            .with_size_points(32.0)
            .with_bold(true),
    )
    .expect("the title's run properties");
    // Colouring part of a run splits it, which is what puts two runs in the title's one paragraph.
    deck.set_text_range_properties(
        0,
        title,
        0,
        0..TITLE_HEAD.len(),
        &CharacterPropertiesSpec::new()
            .with_size_points(32.0)
            .with_bold(true)
            .with_color(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("the title's first run");

    deck.add_picture(0, LOGO_PNG, ShapeBounds::from_inches(0.6, 1.9, 3.0, 2.0))
        .expect("the cropped picture");
    let masked = deck
        .add_picture(0, LOGO_PNG, ShapeBounds::from_inches(3.9, 1.9, 2.0, 2.0))
        .expect("the masked picture");
    deck.set_shape_geometry(
        0,
        masked,
        Geometry::Preset(ShapeGeometry::Unmodeled(PresetShapeType::Ellipse)),
    )
    .expect("the ellipse mask");

    let icon = deck
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(6.2, 1.9, 1.2, 1.2),
        )
        .expect("the icon");
    deck.set_shape_geometry(0, icon, Geometry::Custom(chevron()))
        .expect("the icon's custom geometry");
    deck.set_shape_fill(
        0,
        icon,
        &FillSpec::Solid(ColorSpec::Scheme(SchemeColor::Accent2)),
    )
    .expect("the icon's fill");

    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("Revenue", [12.0, 15.5, 14.0, 19.25])
        .series("Cost", [8.0, 9.5, 9.0, 11.0]);
    deck.add_chart(0, &chart, ShapeBounds::from_inches(0.6, 4.1, 5.2, 2.6))
        .expect("the chart");

    deck.add_diagram(
        0,
        &DiagramContent::vertical_list(&["Plan", "Build", "Ship"]),
        ShapeBounds::from_inches(6.1, 4.1, 3.0, 2.6),
    )
    .expect("the SmartArt frame");

    let table = deck
        .add_table(0, 3, 3, ShapeBounds::from_inches(9.4, 1.9, 3.4, 2.0))
        .expect("the table");
    let rows = [
        ["Region", "Plan", "Actual"],
        ["EMEA", "1,204", "1,310"],
        ["APAC", "987", "1,002"],
    ];
    for (row, cells) in rows.iter().enumerate() {
        for (column, text) in cells.iter().enumerate() {
            deck.set_cell_text(0, table, row, column, 0, text)
                .expect("a table cell");
        }
    }

    let overlay = deck
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(0.6, 3.25, 12.1, 0.6),
        )
        .expect("the overlay");
    deck.set_shape_fill(
        0,
        overlay,
        &FillSpec::Solid(
            ColorSpec::Srgb("1F3864".to_owned()).with_alpha(Fraction::from_ratio(0.35)),
        ),
    )
    .expect("the overlay's fill");
    deck.set_shape_no_outline(0, overlay)
        .expect("the overlay's outline");

    let bullets = deck
        .add_text_box(
            0,
            "Margin expanded\nPipeline healthy\nHeadcount flat",
            ShapeBounds::from_inches(9.4, 4.1, 3.4, 2.0),
        )
        .expect("the list");
    for paragraph in 0..3 {
        deck.set_paragraph_properties(
            0,
            bullets,
            paragraph,
            &ParagraphPropertiesSpec::new()
                .with_bullet_character("\u{a7}")
                .with_bullet_typeface(BulletTypeface::named("Wingdings")),
        )
        .expect("a Wingdings bullet");
    }
}

/// A chevron drawn as a custom geometry, so the slide carries an `a:custGeom`.
fn chevron() -> CustomGeometrySpec {
    let side = Emu::from_emu(1_097_280);
    CustomGeometrySpec {
        paths: vec![Path2DSpec {
            width: Some(side),
            height: Some(side),
            fill: Some(PathFillMode::Normal),
            commands: vec![
                DrawCommand::MoveTo(Point::from_emu(0, 0)),
                DrawCommand::LineTo(Point::from_emu(548_640, 0)),
                DrawCommand::LineTo(Point::from_emu(1_097_280, 548_640)),
                DrawCommand::LineTo(Point::from_emu(548_640, 1_097_280)),
                DrawCommand::LineTo(Point::from_emu(0, 1_097_280)),
                DrawCommand::LineTo(Point::from_emu(548_640, 548_640)),
                DrawCommand::Close,
            ],
            ..Path2DSpec::default()
        }],
        ..CustomGeometrySpec::default()
    }
}

/// The six elements this workspace has a reader for and no writer.
fn splice_the_unwritable(saved: Vec<u8>) -> Package {
    let mut package = Package::open(&saved).expect("the authored deck reopens");

    let mut master = part_text(&package, MASTER);
    splice(
        &mut master,
        "<p:spTree>",
        &format!("{BACKGROUND}<p:spTree>"),
    );
    set_part_text(&mut package, MASTER, master);

    let mut slide = part_text(&package, SLIDE);
    let run_end = format!("<a:t>{TITLE_HEAD}</a:t></a:r>");
    splice(
        &mut slide,
        &run_end,
        &format!("{run_end}<a:br><a:rPr lang=\"en-US\"/></a:br>"),
    );
    // The first `a:stretch` on the slide belongs to the first picture, which is the cropped one.
    splice(&mut slide, "<a:stretch>", &format!("{CROP}<a:stretch>"));
    splice(
        &mut slide,
        "</p:spTree>",
        &format!("{FIELDS}{CONNECTOR}</p:spTree>"),
    );
    set_part_text(&mut package, SLIDE, slide);

    // The cached drawing hangs off the diagram's *data* part, which is where a reader looks for it.
    add_related_part(
        &mut package,
        "/ppt/diagrams/data1.xml",
        "/ppt/diagrams/drawing1.xml",
        DRAWING_CONTENT_TYPE,
        DRAWING_REL,
        "rId10",
        CACHED_DRAWING.as_bytes().to_vec(),
    );
    package
}

/// The connector's arrowhead, written through the typed outline writer.
fn finish(package: Package) -> Vec<u8> {
    let mut deck = Presentation::from_package(package).expect("the spliced deck reopens");
    let connector = deck.shape_count(0).expect("the slide's shapes") - 1;
    let mut line = LineSpec::solid(
        LineWidth::from_points(2.25),
        ColorSpec::Scheme(SchemeColor::Accent2),
    );
    line.tail_end = Some(LineEnd {
        kind: Some(LineEndType::Arrow),
        width: None,
        length: None,
    });
    deck.set_shape_outline(0, connector, &line)
        .expect("the connector's arrowhead");
    deck.save().expect("the corporate deck saves")
}
