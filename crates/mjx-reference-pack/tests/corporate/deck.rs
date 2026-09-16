//! The corporate deck: one slide shaped like a page somebody would actually send (MJXOFF-300).

use mjx_chart::{ChartData, ChartKind};
use mjx_dml::{
    BulletTypeface, CharacterPropertiesSpec, ColorSpec, CustomGeometrySpec, DrawCommand, Emu,
    FillSpec, Fraction, LineEnd, LineSpec, LineWidth, ParagraphPropertiesSpec, Path2DSpec,
    PathFillMode, Point, SchemeColor, ShapeGeometry,
};
use mjx_ooxml_types::drawingml::{LineEndType, PresetShapeType, TextStrike, TextUnderline};
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_pptx::{DiagramContent, Geometry, Package, Presentation, ShapeBounds, SlideSize, Surface};

use super::{add_related_part, part_text, set_part_text, splice, LOGO_PNG};

/// The slide and master parts a blank deck writes.
const SLIDE: &str = "/ppt/slides/slide1.xml";
const MASTER: &str = "/ppt/slideMasters/slideMaster1.xml";

/// The title's first run, which the soft break follows.
const TITLE_HEAD: &str = "Quarterly review";
/// The whole title. One run until `set_text_range_properties` splits it in two.
const TITLE: &str = "Quarterly review across EMEA and APAC";
/// The body placeholder's own line. Like the title, it states no size and takes the master's.
const BODY: &str = "Revenue ahead of plan in every region";
/// The bulleted list, one paragraph per line, each decorated differently.
const BULLETS: [&str; 3] = ["Margin expanded", "Pipeline healthy", "Headcount flat"];
/// The corner radius the overlay's `roundRect` states, as a fraction of its shorter side.
const OVERLAY_CORNER_RADIUS: f64 = 0.25;

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

/// The three nodes the diagram's data model states, as (model id, label).
const DIAGRAM_NODES: [(&str, &str); 3] = [
    ("{7F2C4F4E-0001-4E2E-9F4E-000000000001}", "Plan"),
    ("{7F2C4F4E-0002-4E2E-9F4E-000000000002}", "Build"),
    ("{7F2C4F4E-0003-4E2E-9F4E-000000000003}", "Ship"),
];

/// One cached box: a rounded rectangle in `accent1`, with its label, at a vertical offset.
fn cached_node(model_id: &str, label: &str, top: i64) -> String {
    format!(
        concat!(
            "<dsp:sp modelId=\"{model_id}\">",
            "<dsp:nvSpPr><dsp:cNvPr id=\"0\" name=\"\"/><dsp:cNvSpPr/></dsp:nvSpPr>",
            "<dsp:spPr><a:xfrm><a:off x=\"0\" y=\"{top}\"/>",
            "<a:ext cx=\"2743200\" cy=\"709295\"/></a:xfrm>",
            "<a:prstGeom prst=\"roundRect\"><a:avLst/></a:prstGeom>",
            "<a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill></dsp:spPr>",
            "<dsp:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\"/>",
            "<a:t>{label}</a:t></a:r></a:p></dsp:txBody>",
            "</dsp:sp>"
        ),
        model_id = model_id,
        top = top,
        label = label,
    )
}

/// The SmartArt frame's cached drawing, which `add_diagram` deliberately does not write.
///
/// **Three real shapes**, one per node of the vertical list, each a rounded box with its own text —
/// which is what Office caches and what a renderer draws until it runs the layout algorithms
/// itself. An empty `dsp:spTree` would be a drawing with nothing cached in it, and it is this part
/// that puts the Microsoft diagram-drawing namespace on the schema gate's preserved-foreign list.
fn cached_drawing() -> String {
    let nodes: String = DIAGRAM_NODES
        .iter()
        .enumerate()
        .map(|(index, (model_id, label))| cached_node(model_id, label, index as i64 * 790_575))
        .collect();
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
            "<dsp:drawing xmlns:dsp=\"http://schemas.microsoft.com/office/drawing/2008/diagram\" ",
            "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
            "<dsp:spTree><dsp:nvGrpSpPr><dsp:cNvPr id=\"0\" name=\"\"/>",
            "<dsp:cNvGrpSpPr/></dsp:nvGrpSpPr><dsp:grpSpPr/>",
            "{nodes}",
            "</dsp:spTree></dsp:drawing>"
        ),
        nodes = nodes
    )
}

/// The cached drawing's content type and relationship type, as Office writes them.
const DRAWING_CONTENT_TYPE: &str = "application/vnd.ms-office.drawingml.diagramDrawing+xml";
const DRAWING_REL: &str = "http://schemas.microsoft.com/office/2007/relationships/diagramDrawing";

/// The corporate deck, as bytes.
pub(crate) fn corporate_deck() -> Vec<u8> {
    let mut deck = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    deck.add_slide_from_layout(0).expect("one slide");
    author_master(&mut deck);
    let cropped = author_slide(&mut deck);
    // The cropped picture's own relationship, read before the package is taken apart: the crop is
    // spliced onto *that* picture rather than onto whichever one a writer happens to emit first.
    let crop_rel_id = deck
        .picture_image_rel_id(0, cropped)
        .expect("the cropped picture reads")
        .expect("the cropped picture names an image relationship");
    let saved = deck.save().expect("the authored deck saves");
    finish(splice_the_unwritable(saved, &crop_rel_id))
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

/// The slide itself, in paint order. Answers the index of the picture the crop belongs to.
fn author_slide(deck: &mut Presentation) -> usize {
    // The title and the summary line go in the **placeholders the layout offers**, which is where a
    // corporate deck puts them — and neither states a size of its own, so the master's
    // `p:titleStyle` and `p:bodyStyle` are what lay them out. A placeholder holding an empty run
    // exercises that inheritance with no glyph, which is what it did until MJXOFF-300's audit.
    let title = deck
        .shape_for_placeholder(0, PlaceholderType::Title)
        .expect("the slide's shapes read")
        .expect("the layout offers a title placeholder");
    deck.set_shape_text_content(0, title, TITLE)
        .expect("the title's text");
    deck.set_shape_run_properties(0, title, &CharacterPropertiesSpec::new().with_bold(true))
        .expect("the title's run properties");
    // Colouring part of a run splits it, which is what puts two runs in the title's one paragraph.
    deck.set_text_range_properties(
        0,
        title,
        0,
        0..TITLE_HEAD.len(),
        &CharacterPropertiesSpec::new()
            .with_bold(true)
            .with_color(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("the title's first run");

    let summary = deck
        .shape_for_placeholder(0, PlaceholderType::Body)
        .expect("the slide's shapes read")
        .expect("the layout offers a body placeholder");
    deck.set_shape_text_content(0, summary, BODY)
        .expect("the body placeholder's text");

    let cropped = deck
        .add_picture(0, LOGO_PNG, ShapeBounds::from_inches(0.6, 1.9, 3.0, 2.0))
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
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(0.6, 3.25, 12.1, 0.6),
        )
        .expect("the overlay");
    // A stated adjustment, so the deck's `a:avLst` says something: every other preset here takes
    // the generated table's default, which exercises the adjustment path with nothing.
    deck.set_shape_geometry(
        0,
        overlay,
        Geometry::Preset(ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(OVERLAY_CORNER_RADIUS),
        }),
    )
    .expect("the overlay's corner radius");
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
            &BULLETS.join("\n"),
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
    // One decoration per bullet: the three a reviewer's mark-up leaves on a list.
    let decorations = [
        CharacterPropertiesSpec::new().with_underline(TextUnderline::Single),
        CharacterPropertiesSpec::new().with_strike(TextStrike::SingleStrike),
        CharacterPropertiesSpec::new().with_highlight(ColorSpec::Srgb("FFFF00".to_owned())),
    ];
    for (paragraph, decoration) in decorations.iter().enumerate() {
        deck.set_text_range_properties(
            0,
            bullets,
            paragraph,
            0..BULLETS[paragraph].len(),
            decoration,
        )
        .expect("a decorated bullet");
    }

    cropped
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

/// Splices the crop into the picture whose `a:blip` names `rel_id`, before that picture's own
/// `a:stretch`.
///
/// **Anchored on the picture, not on a position.** Until MJXOFF-300's audit this spliced before *the
/// first `a:stretch` on the slide*, so a change to the order the writers emit shapes in would have
/// moved the crop to the other picture with every gate still green.
fn splice_crop_onto(slide: &mut String, rel_id: &str) {
    let embed = format!("r:embed=\"{rel_id}\"");
    let mut at = 0usize;
    let insert = loop {
        let Some(offset) = slide[at..].find("<p:pic") else {
            panic!(
                "no `p:pic` on the slide carries `{embed}`, so the crop has no picture to sit on. \
                 A writer that renamed the relationship, or a picture that stopped naming one, \
                 lands here."
            );
        };
        let start = at + offset;
        let end = slide[start..]
            .find("</p:pic>")
            .map_or(slide.len(), |end| start + end);
        if slide[start..end].contains(&embed) {
            let stretch = slide[start..end].find("<a:stretch>").unwrap_or_else(|| {
                panic!("the cropped picture states no `a:stretch` for the crop to precede")
            });
            break start + stretch;
        }
        at = end;
    };
    slide.insert_str(insert, CROP);
}

/// The six elements this workspace has a reader for and no writer.
fn splice_the_unwritable(saved: Vec<u8>, crop_rel_id: &str) -> Package {
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
    splice_crop_onto(&mut slide, crop_rel_id);
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
        cached_drawing().into_bytes(),
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
