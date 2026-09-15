//! RC02 render tests: every loss is counted, named and drawn as a labelled placeholder (MJXOFF-299).

use std::path::{Path, PathBuf};
use std::process::Command as Process;

use mjx_layout::{
    BoxModel, Fragment, FragmentTree, FrameContent, LayoutLossKind, LayoutLosses, LayoutSize,
    PageIndex,
};
use mjx_layout_pptx::{PageCatalogue as SlideCatalogue, SlideBoxModel, SlideDeck};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_paint::{
    plan_frame_from, DrawOp, DrawReport, EncodedImages, FaceLibrary, FontSource, OffscreenSurface,
    PaintError, Painter, PainterLossKind, PdfPainter, Pixels, PlanOptions, PlanSources, Resources,
    SoftwarePainter, SvgPainter, Viewport, WgpuPainter,
};
use mjx_pptx::Presentation;
use mjx_scene::{
    build_page, Command, DisplayList, GeometryProvider, LossCategory, OutlineProvenance,
    SceneLossKind, SceneOptions, SceneRect, Tessellator,
};
use mjx_scene_pptx::{SlideGeometry, SlideResources};
use mjx_scene_xlsx::{SheetGeometry, SheetPalette, SheetResources};
use mjx_text::{FontResolver, GlyphAtlas};
use mjx_xlsx::Workbook;

// The labels D8 placeholders carry, one per loss kind.
const LABEL_CHART: &str = "Chart not rendered";
const LABEL_DIAGRAM: &str = "Diagram not rendered";
const LABEL_OBJECT: &str = "Embedded object not rendered";
const LABEL_INK: &str = "Ink not rendered";
const LABEL_PICTURE: &str = "Picture not rendered";
const LABEL_NOT_READ: &str = "Content not read";
const LABEL_APPROXIMATED: &str = "Approximated";
const LABEL_COLOUR: &str = "Colour not resolved";
const LABEL_FILL_IMAGE: &str = "Fill picture not available";
const LABEL_TEXT_PAINT: &str = "Text colour approximated";
const LABEL_IMAGE_PIXELS: &str = "Picture not available";
const LABEL_TEXT_NOT_EMBEDDED: &str = "Text not embedded";
const LABEL_EFFECT: &str = "Effect not drawn";
const LABEL_LINE_END: &str = "Arrowhead not drawn";
const LABEL_OUTLINE: &str = "Shape outline not resolved";

// Unzoomed device pixels are 96 per inch, so one pixel is 9525 EMU.
const EMU_PER_PIXEL: f64 = 9525.0;

// The folder a test reads its input from and writes its output into.
fn folder(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/render/RC02-losses")
        .join(name)
}

// Writes the generated input beside the README when it changed, and hands the bytes back.
fn input(name: &str, file: &str, bytes: Vec<u8>) -> Vec<u8> {
    let path = folder(name).join(file);
    if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
        std::fs::create_dir_all(folder(name)).expect("the test folder is creatable");
        std::fs::write(&path, &bytes).expect("the input is writable");
    }
    bytes
}

// Writes one of our renders into the test's output folder.
fn output(name: &str, file: &str, bytes: &[u8]) -> PathBuf {
    let directory = folder(name).join("output");
    std::fs::create_dir_all(&directory).expect("the output folder is creatable");
    let path = directory.join(file);
    std::fs::write(&path, bytes).expect("the output is writable");
    path
}

// The bundled faces only, so a render does not depend on the machine.
fn resolver() -> FontResolver {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&fonts)
        .expect("the committed faces index")
        .build()
}

// A rectangle stated in EMU, rounded to unzoomed device pixels.
fn px(left: i64, top: i64, right: i64, bottom: i64) -> [i32; 4] {
    [left, top, right, bottom].map(|value| (value as f64 / EMU_PER_PIXEL).round() as i32)
}

// A scene rectangle, rounded to whole device pixels.
fn rounded(rect: SceneRect) -> [i32; 4] {
    [rect.left, rect.top, rect.right, rect.bottom].map(|value| value.round() as i32)
}

// One page on its way to a painter, with everything a painter borrows.
struct Page {
    losses: LayoutLosses,
    tree: FragmentTree,
    list: DisplayList,
    atlas: GlyphAtlas,
    geometry: Box<dyn GeometryProvider>,
    images: EncodedImages,
    width: u32,
    height: u32,
}

// Slide 0 of a deck, laid out and built into a display list with its losses.
fn deck_page(bytes: &[u8]) -> Page {
    let mut presentation = Presentation::open(bytes).expect("the input opens");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let constraints = mjx_layout_pptx::constraints_for(&deck);
    let mut model = SlideBoxModel::new(resolver());
    let page = model
        .layout_page(&deck, PageIndex::new(0), &constraints, None)
        .expect("the slide lays out");
    let mut geometry = SlideGeometry::new();
    geometry.register_all(model.catalogue(), |request| {
        outline_of(
            &mut presentation,
            request.surface_index as usize,
            request.shape.iter().map(|&index| index as usize).collect(),
            mjx_dml::Size::from_emu(request.rect.width().emu(), request.rect.height().emu()),
        )
    });
    let images = package_images(&mut presentation, model.catalogue());
    let options = SceneOptions::new(constraints.page);
    let resources = SlideResources::new(model.catalogue().clone(), options.device_scale);
    let mut atlas = GlyphAtlas::new();
    let list = build_page(
        &page,
        &resources,
        model.rasteriser_mut(),
        &mut atlas,
        &options,
    )
    .expect("no loss fails a scene");
    let losses = page.losses().clone();
    let (tree, _) = page.into_parts();
    finish(losses, tree, list, atlas, Box::new(geometry), images)
}

// The first band of sheet 0 in a viewport of `width` by `height` inches.
fn sheet_page(bytes: &[u8], width: f64, height: f64) -> Page {
    let mut book = Workbook::open(bytes).expect("the input opens");
    let theme = book.theme_colors().expect("the theme part reads");
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = mjx_layout_xlsx::constraints_for(LayoutSize {
        width: Emu::from_inches(width),
        height: Emu::from_inches(height),
    });
    let mut model = SheetBoxModel::new(resolver());
    let page = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");
    let formatting = grid.formatting();
    let interner = formatting
        .resolver()
        .expect("a format resolver")
        .formats()
        .interner();
    let mut palette = SheetPalette::from_stylesheet(formatting.stylesheet(), interner);
    if let Some(theme) = theme {
        palette = palette.with_theme(theme);
    }
    let resources = SheetResources::new(model.catalogue().clone(), palette);
    let options = SceneOptions::new(constraints.page);
    let mut atlas = GlyphAtlas::new();
    let list = build_page(
        &page,
        &resources,
        model.rasteriser_mut(),
        &mut atlas,
        &options,
    )
    .expect("no loss fails a scene");
    let losses = page.losses().clone();
    let (tree, _) = page.into_parts();
    finish(
        losses,
        tree,
        list,
        atlas,
        Box::new(SheetGeometry::new()),
        EncodedImages::new(),
    )
}

// Sizes the target from the display list.
fn finish(
    losses: LayoutLosses,
    tree: FragmentTree,
    list: DisplayList,
    atlas: GlyphAtlas,
    geometry: Box<dyn GeometryProvider>,
    images: EncodedImages,
) -> Page {
    let (width, height) = list.page_size();
    Page {
        losses,
        tree,
        width: width.ceil().max(1.0) as u32,
        height: height.ceil().max(1.0) as u32,
        list,
        atlas,
        geometry,
        images,
    }
}

// The document's own preset geometry for one shape, as the deck journey registers it.
fn outline_of(
    presentation: &mut Presentation,
    surface: usize,
    path: Vec<usize>,
    extents: mjx_dml::Size,
) -> Option<mjx_geometry::ShapeOutline> {
    let surface = mjx_pptx::Surface::Slide(surface);
    let preset = presentation.shape_preset(surface, path.clone()).ok()??;
    let adjustments = presentation
        .shape_adjustments(surface, path, mjx_dml::GuideContext::from_size(extents))
        .unwrap_or_default();
    Some(mjx_geometry::ShapeOutline {
        preset,
        extents,
        adjustments: adjustments
            .into_iter()
            .filter(|adjustment| adjustment.is_overridden)
            .map(|adjustment| {
                mjx_geometry::AdjustmentOverride::new(adjustment.spec.wire_name, adjustment.value)
            })
            .collect(),
    })
}

// The encoded bytes of every picture the page asks for, read out of the package itself.
fn package_images(presentation: &mut Presentation, catalogue: &SlideCatalogue) -> EncodedImages {
    let mut images = EncodedImages::new();
    for (handle, request) in catalogue.images().iter().enumerate() {
        let surface = request.surface_index as usize;
        let shapes = presentation.shape_count(surface).unwrap_or(0);
        for shape in 0..shapes {
            let rel_id = presentation
                .picture_image_rel_id(surface, shape)
                .ok()
                .flatten();
            if rel_id.as_deref() != Some(request.image_rel_id.as_str()) {
                continue;
            }
            if let Ok(Some(bytes)) = presentation.picture_image_bytes(surface, shape) {
                images.insert(handle as u64, bytes.into_owned());
            }
            break;
        }
    }
    images
}

// Draws the page through one painter; no loss may fail the frame.
fn paint(
    painter: &mut dyn Painter,
    page: &mut Page,
    fonts: &dyn FontSource,
) -> (DrawReport, Option<Pixels>) {
    let mut host = OffscreenSurface::new(page.width, page.height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter
        .begin(&mut host, viewport)
        .expect("the frame begins");
    let drawn = {
        let mut resources =
            Resources::new(&mut page.atlas, page.geometry.as_ref(), &page.images).with_fonts(fonts);
        painter
            .draw(&frame, &page.list, &mut resources)
            .expect("no loss fails a frame")
    };
    painter.end(frame).expect("the frame ends");
    let pixels = painter.read_pixels().expect("readback answers");
    (drawn, pixels)
}

// Draws the page through the software painter and keeps the PNG in the output folder.
fn software(name: &str, page: &mut Page) -> (DrawReport, Pixels) {
    let mut painter = SoftwarePainter::new();
    let (drawn, pixels) = paint(&mut painter, page, &FaceLibrary::new());
    let pixels = pixels.expect("the software painter reads back");
    output(
        name,
        "software.png",
        &mjx_paint::export::png(pixels.width, pixels.height, &pixels.rgba),
    );
    (drawn, pixels)
}

// Inked pixels inside and outside a device-pixel rectangle.
fn ink_split(pixels: &Pixels, rect: [i32; 4]) -> (usize, usize) {
    let (mut inside, mut outside) = (0, 0);
    for y in 0..pixels.height {
        for x in 0..pixels.width {
            let Some([_, _, _, alpha]) = pixels.pixel(x, y) else {
                continue;
            };
            if alpha == 0 {
                continue;
            }
            let (x, y) = (x as i32, y as i32);
            if x >= rect[0] && x < rect[2] && y >= rect[1] && y < rect[3] {
                inside += 1;
            } else {
                outside += 1;
            }
        }
    }
    (inside, outside)
}

// Label-ink pixels in the middle half of a device-pixel rectangle's height, where a placeholder sets its label.
fn label_ink(pixels: &Pixels, rect: [i32; 4]) -> usize {
    let height = rect[3] - rect[1];
    let (top, bottom) = (rect[1] + height / 4, rect[3] - height / 4);
    let mut ink = 0;
    for y in top.max(0)..bottom.max(0) {
        for x in rect[0].max(0)..rect[2].max(0) {
            if pixels
                .pixel(x as u32, y as u32)
                .is_some_and(|[red, green, blue, alpha]| {
                    alpha >= 0xe0
                        && green <= 0x20
                        && (0x40..=0x80).contains(&red)
                        && red.abs_diff(blue) <= 8
                })
            {
                ink += 1;
            }
        }
    }
    ink
}

// Asserts every rectangle carries a readable label in the software painter's pixels.
fn assert_labelled(pixels: &Pixels, rects: &[[i32; 4]]) {
    for rect in rects {
        let ink = label_ink(pixels, *rect);
        assert!(
            ink > 0,
            "{ink} label pixels inside the placeholder at {rect:?}; its label must be readable in the PNG"
        );
    }
}

// Layout losses as [chart, diagram, object, ink, picture, shape, unshaped, not read, approximated].
fn layout_vector(losses: &LayoutLosses) -> [usize; 9] {
    let counts = [
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Chart),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Diagram),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::EmbeddedObject),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Picture),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Shape),
        LayoutLossKind::TextMeasuredNotShaped,
        LayoutLossKind::DroppedByReader,
        LayoutLossKind::ValueApproximated,
    ]
    .map(|kind| losses.count(kind));
    assert_eq!(
        counts.iter().sum::<usize>(),
        losses.len(),
        "a layout loss of a kind this vector does not name"
    );
    counts
}

// Scene losses as [chart, colour, fill picture, text paint, paint approximated].
fn scene_vector(list: &DisplayList) -> [usize; 5] {
    let losses = list.losses();
    let counts = [
        SceneLossKind::ChartNotResolved,
        SceneLossKind::ColourNotResolved,
        SceneLossKind::FillImageNotSupplied,
        SceneLossKind::TextPaintDefaulted,
        SceneLossKind::PaintApproximated,
    ]
    .map(|kind| losses.count(kind));
    assert_eq!(
        counts.iter().sum::<usize>(),
        losses
            .iter()
            .filter(|loss| matches!(loss.category, LossCategory::Scene(_)))
            .count(),
        "a scene loss of a kind this vector does not name"
    );
    counts
}

// The labels of the placeholders one lowering of the page draws, whichever stage lost what they stand for.
fn plan_labels(page: &mut Page, options: PlanOptions) -> Vec<&'static str> {
    let resources = Resources::new(&mut page.atlas, page.geometry.as_ref(), &page.images);
    let plan = plan_frame_from(
        &page.list,
        PlanSources::from_resources(&resources),
        &mut Tessellator::new(),
        options,
    )
    .expect("no loss fails a plan");
    plan.layers()
        .iter()
        .flat_map(|layer| &layer.ops)
        .filter_map(|op| match op {
            DrawOp::Placeholder { label, .. } => Some(*label),
            _ => None,
        })
        .collect()
}

// Painter losses as [image pixels, glyph run, effect, line end, outline].
fn painter_vector(drawn: &DrawReport) -> [usize; 5] {
    let counts = [
        PainterLossKind::ImageWithNoPixels,
        PainterLossKind::GlyphRunNotEmbedded,
        PainterLossKind::EffectUnsupported,
        PainterLossKind::LineEndNotDrawn,
        PainterLossKind::OutlineUnresolved,
    ]
    .map(|kind| drawn.losses.count(kind));
    assert_eq!(
        counts.iter().sum::<usize>(),
        drawn.losses.len(),
        "a painter loss of a kind this vector does not name"
    );
    counts
}

// Every placeholder in the list as (pixel rectangle, label, category, source path).
fn placeholders(list: &DisplayList) -> Vec<([i32; 4], String, LossCategory, Vec<u32>)> {
    list.placeholders()
        .into_iter()
        .map(|placeholder| {
            (
                rounded(placeholder.rect),
                placeholder.label,
                placeholder.category,
                placeholder.source.path().segments().to_vec(),
            )
        })
        .collect()
}

// One expected placeholder row.
fn placeholder(
    rect: [i32; 4],
    label: &str,
    category: LossCategory,
    path: &[u32],
) -> ([i32; 4], String, LossCategory, Vec<u32>) {
    (rect, label.to_owned(), category, path.to_vec())
}

// The text pdftotext reads out of a PDF.
fn pdf_text(path: &Path) -> String {
    let result = Process::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .expect("pdftotext is installed");
    assert!(
        result.status.success(),
        "pdftotext refused {}",
        path.display()
    );
    String::from_utf8_lossy(&result.stdout).into_owned()
}

// The inputs, authored with the workspace's own writers and today's API.
mod inputs {
    use mjx_opc::{Relationship, TargetMode};
    use mjx_pptx::{ChartData, ChartKind, Package, PartName, Presentation, ShapeBounds, SlideSize};
    use mjx_xlsx::drawing_geometry::{CellMarker, ResizingBehavior};
    use mjx_xlsx::Workbook;

    const NAMESPACES: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main""#;

    // A valid one-pixel red PNG.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    const SHEET_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#;

    // One extra part on slide 1: relationship id, part name, content type, relationship type, bytes.
    type SlidePart = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Vec<u8>,
    );

    // A one-slide widescreen deck whose shape tree is exactly `shapes`.
    fn deck(shapes: &str, parts: Vec<SlidePart>) -> Vec<u8> {
        let mut blank = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
        blank.add_slide_from_layout(0).expect("one slide");
        let mut package =
            Package::open(&blank.save().expect("the blank deck saves")).expect("it reopens");
        let slide = PartName::new("/ppt/slides/slide1.xml").expect("a part name");
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {NAMESPACES}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
        );
        package
            .replace_part_bytes(&slide, xml.into_bytes())
            .expect("the slide is replaceable");
        for (id, name, content_type, rel_type, bytes) in parts {
            let part = PartName::new(name).expect("a part name");
            package
                .insert_part(&part, content_type, bytes)
                .expect("the part inserts");
            package
                .add_relationship(
                    Some(&slide),
                    Relationship {
                        id: id.to_owned(),
                        rel_type: rel_type.to_owned(),
                        target: format!("..{}", &name[4..]),
                        mode: TargetMode::Internal,
                    },
                )
                .expect("the relationship adds");
        }
        package.save().expect("the deck saves")
    }

    // An autoshape at (x, y) EMU of 3 by 2 inches.
    fn shape(id: u32, x: i64, y: i64, properties: &str, body: &str) -> String {
        format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Shape {id}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="2743200" cy="1828800"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{properties}</p:spPr>{body}</p:sp>"#
        )
    }

    // A text body of one run.
    fn body(run_properties: &str, text: &str) -> String {
        format!(
            r#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="2800">{run_properties}</a:rPr><a:t>{text}</a:t></a:r></a:p></p:txBody>"#
        )
    }

    const ARROW: &str = r#"<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="2" name="Arrow 1"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="914400" y="1828800"/><a:ext cx="5486400" cy="0"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom><a:ln w="38100"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:headEnd type="triangle"/><a:tailEnd type="arrow"/></a:ln></p:spPr></p:cxnSp>"#;

    // Adds a picture and then deletes its media part, leaving the relationship dangling.
    fn with_absent_picture(deck_bytes: &[u8], bounds: ShapeBounds) -> Vec<u8> {
        let mut presentation = Presentation::open(deck_bytes).expect("the deck opens");
        presentation
            .add_picture(0, PNG, bounds)
            .expect("the picture adds");
        let mut package =
            Package::open(&presentation.save().expect("the deck saves")).expect("it reopens");
        package
            .remove_part(&PartName::new("/ppt/media/image1.png").expect("a part name"))
            .expect("the media part removes");
        package.save_unchecked().expect("the deck saves")
    }

    // 01: a rectangle filled with a picture fill and no outline.
    pub(crate) fn blip_filled_shape() -> Vec<u8> {
        deck(
            &shape(
                2,
                914400,
                914400,
                r#"<a:blipFill><a:blip r:embed="rId9"/><a:stretch><a:fillRect/></a:stretch></a:blipFill><a:ln><a:noFill/></a:ln>"#,
                "",
            ),
            vec![(
                "rId9",
                "/ppt/media/image9.png",
                "image/png",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
                PNG.to_vec(),
            )],
        )
    }

    // 02: one run of red text in an unfilled box.
    pub(crate) fn red_run() -> Vec<u8> {
        deck(
            &shape(
                2,
                914400,
                914400,
                "",
                &body(
                    r#"<a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>"#,
                    "Red text",
                ),
            ),
            vec![],
        )
    }

    // 03: two rectangles filled with a placeholder colour outside any style, one transformed.
    pub(crate) fn placeholder_colours() -> Vec<u8> {
        let filled = |id: u32, x: i64, colour: &str| {
            shape(
                id,
                x,
                914400,
                &format!(r#"<a:solidFill>{colour}</a:solidFill><a:ln><a:noFill/></a:ln>"#),
                "",
            )
        };
        deck(
            &format!(
                "{}{}",
                filled(2, 914400, r#"<a:schemeClr val="phClr"/>"#),
                filled(
                    3,
                    4572000,
                    r#"<a:schemeClr val="phClr"><a:lumMod val="75000"/></a:schemeClr>"#
                )
            ),
            vec![],
        )
    }

    // 04: a picture whose media part is absent from the package.
    pub(crate) fn absent_picture() -> Vec<u8> {
        with_absent_picture(
            &deck("", vec![]),
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
    }

    // 05: one run of black text.
    pub(crate) fn hello_text() -> Vec<u8> {
        deck(
            &shape(
                2,
                914400,
                914400,
                "",
                &body(
                    r#"<a:solidFill><a:srgbClr val="000000"/></a:solidFill>"#,
                    "Hello",
                ),
            ),
            vec![],
        )
    }

    // 06: a blue rectangle with an inner shadow and an orange one with a reflection.
    pub(crate) fn inner_shadow_and_reflection() -> Vec<u8> {
        deck(
            &format!(
                "{}{}",
                shape(
                    2,
                    914400,
                    914400,
                    r#"<a:solidFill><a:srgbClr val="4472C4"/></a:solidFill><a:effectLst><a:innerShdw blurRad="63500" dist="50800" dir="2700000"><a:srgbClr val="000000"/></a:innerShdw></a:effectLst>"#,
                    ""
                ),
                shape(
                    3,
                    4572000,
                    914400,
                    r#"<a:solidFill><a:srgbClr val="ED7D31"/></a:solidFill><a:effectLst><a:reflection blurRad="6350" stA="50000" endA="300" endPos="55000" dist="0" dir="5400000" sy="-100000" algn="bl" rotWithShape="0"/></a:effectLst>"#,
                    ""
                )
            ),
            vec![],
        )
    }

    // 07: a horizontal connector with a triangle head and an arrow tail.
    pub(crate) fn arrow_connector() -> Vec<u8> {
        deck(ARROW, vec![])
    }

    // 08: a bar chart of three categories on a slide.
    pub(crate) fn slide_chart() -> Vec<u8> {
        let mut presentation = Presentation::open(&deck("", vec![])).expect("the deck opens");
        presentation
            .add_chart(0, &chart(), ShapeBounds::from_inches(1.0, 1.0, 6.0, 4.0))
            .expect("the chart adds");
        presentation.save().expect("the deck saves")
    }

    // The chart both chart inputs carry.
    fn chart() -> ChartData {
        ChartData::new(ChartKind::Bar)
            .categories(["Q1", "Q2", "Q3"])
            .series("Revenue", [10.0, 20.0, 15.0])
    }

    // 09 and 19: the same bar chart anchored from B2 to G13 on a blank sheet.
    pub(crate) fn sheet_chart() -> Vec<u8> {
        let mut book = Workbook::blank().expect("a blank workbook");
        book.add_chart(
            0,
            &chart(),
            CellMarker::new(1, 0, 1, 0),
            CellMarker::new(6, 0, 12, 0),
            "Revenue",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("the chart adds");
        book.save().expect("the workbook saves")
    }

    // 10: a SmartArt graphic frame with an empty data model.
    pub(crate) fn diagram_frame() -> Vec<u8> {
        deck(
            r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="Diagram 1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="914400" y="914400"/><a:ext cx="5486400" cy="3657600"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rId7" r:lo="rId7" r:qs="rId7" r:cs="rId7"/></a:graphicData></a:graphic></p:graphicFrame>"#,
            vec![(
                "rId7",
                "/ppt/diagrams/data1.xml",
                "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData",
                br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:ptLst/></dgm:dataModel>"#.to_vec(),
            )],
        )
    }

    // 11: an embedded OLE object frame with no fallback picture.
    pub(crate) fn ole_frame() -> Vec<u8> {
        deck(
            r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="Object 1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="914400" y="914400"/><a:ext cx="3657600" cy="2743200"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole"><p:oleObj name="Packager" r:id="rId7" imgW="3657600" imgH="2743200" progId="Package"><p:embed/></p:oleObj></a:graphicData></a:graphic></p:graphicFrame>"#,
            vec![(
                "rId7",
                "/ppt/embeddings/oleObject1.bin",
                "application/vnd.openxmlformats-officedocument.oleObject",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject",
                b"not a compound file".to_vec(),
            )],
        )
    }

    // 12: an ink content part of 3 by 2 inches.
    pub(crate) fn ink_part() -> Vec<u8> {
        deck(
            r#"<mc:AlternateContent><mc:Choice Requires="p14"><p:contentPart p14:bwMode="auto" r:id="rId7"><p14:nvContentPartPr><p14:cNvPr id="2" name="Ink 1"/><p14:cNvContentPartPr/><p14:nvPr/></p14:nvContentPartPr><p14:xfrm><a:off x="914400" y="914400"/><a:ext cx="2743200" cy="1828800"/></p14:xfrm></p:contentPart></mc:Choice><mc:Fallback/></mc:AlternateContent>"#,
            vec![(
                "rId7",
                "/ppt/ink/ink1.xml",
                "application/inkml+xml",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
                br#"<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0, 1000 1000, 2000 0</inkml:trace></inkml:ink>"#.to_vec(),
            )],
        )
    }

    // A blank workbook whose sheet and, optionally, styles are exactly the given markup.
    fn book(sheet: &str, styles: Option<&str>) -> Vec<u8> {
        let blank = Workbook::blank()
            .expect("a blank workbook")
            .save()
            .expect("it saves");
        let mut package = mjx_xlsx::Package::open(&blank).expect("it reopens");
        package
            .replace_part_bytes(
                &PartName::new("/xl/worksheets/sheet1.xml").expect("a part name"),
                format!("{SHEET_HEAD}{sheet}</worksheet>").into_bytes(),
            )
            .expect("the sheet is replaceable");
        if let Some(styles) = styles {
            package
                .replace_part_bytes(
                    &PartName::new("/xl/styles.xml").expect("a part name"),
                    styles.as_bytes().to_vec(),
                )
                .expect("the styles are replaceable");
        }
        package.save().expect("the workbook saves")
    }

    // 13: 10, 20 and 30 in A1:A3 under a three-arrow icon set.
    pub(crate) fn icon_sheet() -> Vec<u8> {
        book(
            r#"<sheetData><row r="1"><c r="A1"><v>10</v></c></row><row r="2"><c r="A2"><v>20</v></c></row><row r="3"><c r="A3"><v>30</v></c></row></sheetData><conditionalFormatting sqref="A1:A3"><cfRule type="iconSet" priority="1"><iconSet iconSet="3Arrows"><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule></conditionalFormatting>"#,
            None,
        )
    }

    // 14: an empty B2 whose only border is a thin top-left to bottom-right diagonal.
    pub(crate) fn diagonal_sheet() -> Vec<u8> {
        book(
            r#"<sheetData><row r="2"><c r="B2" s="1"/></row></sheetData>"#,
            Some(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="1"><font><sz val="11"/><name val="Liberation Sans"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border diagonalDown="1"><left/><right/><top/><bottom/><diagonal style="thin"><color rgb="FF000000"/></diagonal></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/><xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="1"/></cellXfs></styleSheet>"#,
            ),
        )
    }

    // 15: A1 holds an inline string of a bold red run and a plain run.
    pub(crate) fn rich_text_sheet() -> Vec<u8> {
        book(
            r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><r><rPr><b/><color rgb="FFFF0000"/><sz val="11"/><rFont val="Liberation Sans"/></rPr><t>Bold</t></r><r><t xml:space="preserve"> plain</t></r></is></c></row></sheetData>"#,
            None,
        )
    }

    // 16: the arrow connector above a picture whose media part is absent.
    pub(crate) fn picture_and_arrow() -> Vec<u8> {
        with_absent_picture(
            &deck(ARROW, vec![]),
            ShapeBounds::from_inches(1.0, 3.0, 3.0, 2.0),
        )
    }
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_01_a_shape_whose_fill_cannot_be_answered_is_a_labelled_placeholder() {
    const NAME: &str = "01-undecorated-shape";
    let bytes = input(NAME, "input.pptx", inputs::blip_filled_shape());
    let mut page = deck_page(&bytes);
    let shape = px(914400, 914400, 3657600, 2743200);
    assert_eq!(
        layout_vector(&page.losses),
        [0; 9],
        "the shape lays out whole"
    );
    assert_eq!(
        scene_vector(&page.list),
        [0, 0, 1, 0, 0],
        "one picture fill the resolver cannot supply"
    );
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            shape,
            LABEL_FILL_IMAGE,
            LossCategory::Scene(SceneLossKind::FillImageNotSupplied),
            &[0, 0]
        )]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(
        drawn.loss_placeholders, 1,
        "the painter draws the placeholder"
    );
    let (inside, outside) = ink_split(&pixels, shape);
    assert!(
        inside > 0 && outside == 0,
        "{inside} inked pixels inside the shape and {outside} outside; today the shape draws nothing"
    );
    assert_labelled(&pixels, &[shape]);
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_02_a_run_whose_paint_is_defaulted_is_counted() {
    const NAME: &str = "02-text-colour-default";
    let bytes = input(NAME, "input.pptx", inputs::red_run());
    let mut page = deck_page(&bytes);
    assert_eq!(layout_vector(&page.losses), [0; 9]);
    assert_eq!(
        scene_vector(&page.list),
        [0, 0, 0, 1, 0],
        "the one red run is drawn in the default colour"
    );
    let run = page
        .tree
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .map(|(_, node)| node.source().clone())
        .expect("the run lays out");
    let loss = page.list.losses().iter().next().cloned().expect("one loss");
    assert_eq!(
        (loss.source, loss.category.label()),
        (run, LABEL_TEXT_PAINT),
        "the loss names the run it approximated"
    );
    assert_eq!(
        placeholders(&page.list),
        vec![],
        "an approximation draws no placeholder"
    );
    let (drawn, _) = software(NAME, &mut page);
    assert_eq!(
        (drawn.glyphs, drawn.loss_placeholders),
        (7, 0),
        "seven inked glyphs of \"Red text\" and no placeholder"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_03_an_unresolved_colour_is_a_labelled_placeholder() {
    const NAME: &str = "03-unresolved-colour";
    let bytes = input(NAME, "input.pptx", inputs::placeholder_colours());
    let mut page = deck_page(&bytes);
    assert_eq!(layout_vector(&page.losses), [0; 9]);
    assert_eq!(
        scene_vector(&page.list),
        [0, 2, 0, 0, 0],
        "a bare scheme colour and a transformed one"
    );
    assert_eq!(
        placeholders(&page.list),
        vec![
            placeholder(
                px(914400, 914400, 3657600, 2743200),
                LABEL_COLOUR,
                LossCategory::Scene(SceneLossKind::ColourNotResolved),
                &[0, 0]
            ),
            placeholder(
                px(4572000, 914400, 7315200, 2743200),
                LABEL_COLOUR,
                LossCategory::Scene(SceneLossKind::ColourNotResolved),
                &[0, 1]
            ),
        ]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 2);
    assert_labelled(
        &pixels,
        &[
            px(914400, 914400, 3657600, 2743200),
            px(4572000, 914400, 7315200, 2743200),
        ],
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_04_a_picture_with_no_pixels_is_a_labelled_placeholder() {
    const NAME: &str = "04-missing-image";
    let bytes = input(NAME, "input.pptx", inputs::absent_picture());
    let mut page = deck_page(&bytes);
    let picture = px(914400, 914400, 3657600, 2743200);
    assert_eq!(layout_vector(&page.losses), [0; 9]);
    assert_eq!(
        scene_vector(&page.list),
        [0; 5],
        "the scene cannot know the part is absent"
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(painter_vector(&drawn), [1, 0, 0, 0, 0]);
    assert_eq!(
        (drawn.images, drawn.loss_placeholders),
        (0, 1),
        "no picture drawn and one placeholder; today the report claims one picture"
    );
    assert_eq!(
        plan_labels(&mut page, PlanOptions::for_raster()),
        [LABEL_IMAGE_PIXELS],
        "the painter's one placeholder reads the loss it stands for"
    );
    let (inside, outside) = ink_split(&pixels, picture);
    assert!(
        inside > 0 && outside == 0,
        "{inside} inked pixels inside the picture's box and {outside} outside"
    );
    assert_labelled(&pixels, &[picture]);
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_05_a_glyph_run_the_pdf_cannot_embed_is_a_labelled_placeholder() {
    const NAME: &str = "05-pdf-glyph-run";
    let bytes = input(NAME, "input.pptx", inputs::hello_text());
    let mut page = deck_page(&bytes);
    assert_eq!(scene_vector(&page.list), [0, 0, 0, 1, 0]);
    let mut pdf = PdfPainter::new();
    let (drawn, _) = paint(&mut pdf, &mut page, &FaceLibrary::new());
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0, 1, 0, 0, 0], 1),
        "the exporter has no face for the one run"
    );
    let path = output(NAME, "export.pdf", pdf.document().expect("a finished PDF"));
    assert_eq!(
        pdf_text(&path).trim(),
        LABEL_TEXT_NOT_EMBEDDED,
        "the PDF's only text is the placeholder's label"
    );
    let mut page = deck_page(&bytes);
    let (drawn, _) = software(NAME, &mut page);
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0; 5], 0),
        "a rasteriser draws the run from the atlas"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_06_an_effect_the_svg_cannot_express_is_counted_and_labelled() {
    const NAME: &str = "06-svg-effects";
    let bytes = input(NAME, "input.pptx", inputs::inner_shadow_and_reflection());
    let mut page = deck_page(&bytes);
    let mut svg = SvgPainter::new();
    let (drawn, _) = paint(&mut svg, &mut page, &FaceLibrary::new());
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0, 0, 2, 0, 0], 2),
        "an inner shadow and a reflection"
    );
    let document = svg.document().expect("a finished SVG").to_owned();
    output(NAME, "export.svg", document.as_bytes());
    assert_eq!(
        document
            .matches(&format!("data-mjx-loss=\"{LABEL_EFFECT}\""))
            .count(),
        2
    );
    let mut page = deck_page(&bytes);
    let (drawn, _) = software(NAME, &mut page);
    assert_eq!(
        (painter_vector(&drawn), drawn.layers),
        ([0; 5], 2),
        "the rasteriser draws both effects in their own layers"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_07_a_line_end_that_is_not_drawn_is_counted() {
    const NAME: &str = "07-line-ends";
    let bytes = input(NAME, "input.pptx", inputs::arrow_connector());
    let mut page = deck_page(&bytes);
    assert_eq!(scene_vector(&page.list), [0; 5]);
    let (drawn, _) = software(NAME, &mut page);
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0, 0, 0, 2, 0], 0),
        "a head and a tail; a zero-height connector has no room for a placeholder"
    );
    let labels = plan_labels(&mut page, PlanOptions::for_raster());
    assert!(
        labels.is_empty() && !labels.contains(&LABEL_LINE_END),
        "an arrowhead is counted and draws no placeholder, and the plan drew {labels:?}"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_08_a_slide_chart_is_one_labelled_placeholder() {
    const NAME: &str = "08-slide-chart";
    let bytes = input(NAME, "input.pptx", inputs::slide_chart());
    let mut page = deck_page(&bytes);
    assert_eq!(
        layout_vector(&page.losses),
        [0, 0, 0, 0, 0, 0, 1, 0, 0],
        "the chart engine lays the chart out and measures its text with nominal metrics, once"
    );
    assert_eq!(
        scene_vector(&page.list),
        [1, 0, 0, 0, 0],
        "one chart, however many handles"
    );
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            px(914400, 914400, 6400800, 4572000),
            LABEL_CHART,
            LossCategory::Scene(SceneLossKind::ChartNotResolved),
            &[0, 0]
        )]
    );
    let hit = page
        .list
        .placeholder_at(384.0, 288.0)
        .map(|found| (found.category, found.source.path().segments().to_vec()));
    assert_eq!(
        hit,
        Some((
            LossCategory::Scene(SceneLossKind::ChartNotResolved),
            vec![0, 0]
        ))
    );
    assert_eq!(
        page.list
            .placeholder_at(10.0, 10.0)
            .map(|found| found.label),
        None
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 1);
    assert_labelled(&pixels, &[px(914400, 914400, 6400800, 4572000)]);
    assert_eq!(
        drawn.page_losses(&page.list).vector(),
        vec![
            (
                LossCategory::Layout(LayoutLossKind::TextMeasuredNotShaped),
                1
            ),
            (LossCategory::Scene(SceneLossKind::ChartNotResolved), 1),
        ],
        "the page's whole loss vector"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_09_a_sheet_chart_is_one_labelled_placeholder() {
    const NAME: &str = "09-sheet-chart";
    let bytes = input(NAME, "input.xlsx", inputs::sheet_chart());
    let mut page = sheet_page(&bytes, 6.0, 4.0);
    assert_eq!(layout_vector(&page.losses), [0, 0, 0, 0, 0, 0, 1, 0, 0]);
    assert_eq!(scene_vector(&page.list), [1, 0, 0, 0, 0]);
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            px(581025, 190500, 3486150, 2286000),
            LABEL_CHART,
            LossCategory::Scene(SceneLossKind::ChartNotResolved),
            &[u32::MAX, 0]
        )]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 1);
    assert_labelled(&pixels, &[px(581025, 190500, 3486150, 2286000)]);
    assert_eq!(
        drawn.page_losses(&page.list).vector(),
        vec![
            (
                LossCategory::Layout(LayoutLossKind::TextMeasuredNotShaped),
                1
            ),
            (LossCategory::Scene(SceneLossKind::ChartNotResolved), 1),
        ],
        "the page's whole loss vector"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_10_a_diagram_frame_is_one_labelled_placeholder() {
    const NAME: &str = "10-diagram-frame";
    let bytes = input(NAME, "input.pptx", inputs::diagram_frame());
    let mut page = deck_page(&bytes);
    let category = LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(
        FrameContent::Diagram,
    ));
    assert_eq!(layout_vector(&page.losses), [0, 1, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(scene_vector(&page.list), [0; 5]);
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            px(914400, 914400, 6400800, 4572000),
            LABEL_DIAGRAM,
            category,
            &[0, 0]
        )]
    );
    assert_eq!(
        page.list
            .placeholder_at(384.0, 288.0)
            .map(|found| found.category),
        Some(category)
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 1);
    assert_labelled(&pixels, &[px(914400, 914400, 6400800, 4572000)]);
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_11_an_embedded_object_frame_is_one_labelled_placeholder() {
    const NAME: &str = "11-ole-frame";
    let bytes = input(NAME, "input.pptx", inputs::ole_frame());
    let mut page = deck_page(&bytes);
    assert_eq!(layout_vector(&page.losses), [0, 0, 1, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            px(914400, 914400, 4572000, 3657600),
            LABEL_OBJECT,
            LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(
                FrameContent::EmbeddedObject
            )),
            &[0, 0]
        )]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 1);
    assert_labelled(&pixels, &[px(914400, 914400, 4572000, 3657600)]);
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_12_an_ink_part_is_one_labelled_placeholder() {
    const NAME: &str = "12-ink-frame";
    let bytes = input(NAME, "input.pptx", inputs::ink_part());
    let mut page = deck_page(&bytes);
    assert_eq!(layout_vector(&page.losses), [0, 0, 0, 1, 0, 0, 0, 0, 0]);
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            px(914400, 914400, 3657600, 2743200),
            LABEL_INK,
            LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink)),
            &[0, u32::MAX, 2]
        )],
        "the wrapped part is addressed as the shape tree's third child element, outside the shape index space"
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 1);
    assert_labelled(&pixels, &[px(914400, 914400, 3657600, 2743200)]);
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_13_every_chosen_icon_is_a_labelled_placeholder() {
    const NAME: &str = "13-excel-icons";
    let bytes = input(NAME, "input.xlsx", inputs::icon_sheet());
    let mut page = sheet_page(&bytes, 6.0, 4.0);
    let category = LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(
        FrameContent::Picture,
    ));
    assert_eq!(layout_vector(&page.losses), [0, 0, 0, 0, 3, 0, 0, 0, 0]);
    assert_eq!(
        placeholders(&page.list),
        vec![
            placeholder(px(0, 0, 581025, 190500), LABEL_PICTURE, category, &[0, 0]),
            placeholder(
                px(0, 190500, 581025, 381000),
                LABEL_PICTURE,
                category,
                &[1, 0]
            ),
            placeholder(
                px(0, 381000, 581025, 571500),
                LABEL_PICTURE,
                category,
                &[2, 0]
            ),
        ]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 3);
    assert_labelled(
        &pixels,
        &[
            px(0, 0, 581025, 190500),
            px(0, 190500, 581025, 381000),
            px(0, 381000, 581025, 571500),
        ],
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_14_a_dropped_diagonal_border_is_a_labelled_placeholder() {
    const NAME: &str = "14-excel-diagonal";
    let bytes = input(NAME, "input.xlsx", inputs::diagonal_sheet());
    let mut page = sheet_page(&bytes, 6.0, 4.0);
    let cell = px(657225, 190500, 1314450, 381000);
    assert_eq!(layout_vector(&page.losses), [0, 0, 0, 0, 0, 0, 0, 1, 0]);
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            cell,
            LABEL_NOT_READ,
            LossCategory::Layout(LayoutLossKind::DroppedByReader),
            &[1, 1]
        )]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(drawn.loss_placeholders, 1);
    let (inside, outside) = ink_split(&pixels, cell);
    assert!(
        inside > 0 && outside == 0,
        "{inside} inked pixels inside B2 and {outside} outside; today the sheet draws nothing"
    );
    assert_labelled(&pixels, &[cell]);
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_15_flattened_rich_text_is_counted_as_approximated() {
    const NAME: &str = "15-excel-rich-text";
    let bytes = input(NAME, "input.xlsx", inputs::rich_text_sheet());
    let mut page = sheet_page(&bytes, 6.0, 4.0);
    assert_eq!(layout_vector(&page.losses), [0, 0, 0, 0, 0, 0, 0, 0, 1]);
    let loss = page.losses.iter().next().cloned().expect("one loss");
    assert_eq!(
        (loss.source.path().segments().to_vec(), loss.kind.label()),
        (vec![0, 0], LABEL_APPROXIMATED),
        "the loss names A1"
    );
    assert_eq!(
        placeholders(&page.list),
        vec![],
        "an approximation draws no placeholder"
    );
    let (drawn, _) = software(NAME, &mut page);
    assert_eq!(
        (drawn.glyphs, drawn.loss_placeholders),
        (9, 0),
        "nine inked glyphs of \"Bold plain\" and no placeholder"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_16_the_four_painters_agree_on_the_loss_counts() {
    const NAME: &str = "16-four-painters";
    let bytes = input(NAME, "input.pptx", inputs::picture_and_arrow());
    let fonts = FaceLibrary::new();
    let mut reports = Vec::new();

    let mut tiny_skia = SoftwarePainter::new();
    let (drawn, pixels) = paint(&mut tiny_skia, &mut deck_page(&bytes), &fonts);
    let pixels = pixels.expect("the software painter reads back");
    output(
        NAME,
        "software.png",
        &mjx_paint::export::png(pixels.width, pixels.height, &pixels.rgba),
    );
    assert_labelled(&pixels, &[px(914400, 2743200, 3657600, 4572000)]);
    reports.push((
        tiny_skia.name(),
        painter_vector(&drawn),
        drawn.loss_placeholders,
    ));

    let mut pdf = PdfPainter::new();
    let (drawn, _) = paint(&mut pdf, &mut deck_page(&bytes), &fonts);
    output(NAME, "export.pdf", pdf.document().expect("a finished PDF"));
    reports.push((pdf.name(), painter_vector(&drawn), drawn.loss_placeholders));

    let mut svg = SvgPainter::new();
    let (drawn, _) = paint(&mut svg, &mut deck_page(&bytes), &fonts);
    output(
        NAME,
        "export.svg",
        svg.document().expect("a finished SVG").as_bytes(),
    );
    reports.push((svg.name(), painter_vector(&drawn), drawn.loss_placeholders));

    match WgpuPainter::offscreen() {
        Ok(mut gpu) => {
            let (drawn, _) = paint(&mut gpu, &mut deck_page(&bytes), &fonts);
            reports.push((gpu.name(), painter_vector(&drawn), drawn.loss_placeholders));
        }
        Err(PaintError::NoAdapter { .. } | PaintError::Device(_)) => {
            println!("{NAME}: wgpu skipped, no graphics adapter");
        }
        Err(other) => panic!("the wgpu painter could not be built: {other}"),
    }

    let expected = [1, 0, 0, 2, 0];
    for (painter, vector, drawn_placeholders) in &reports {
        assert_eq!(
            (*vector, *drawn_placeholders),
            (expected, 1),
            "{painter}: one absent picture, two line ends, one placeholder"
        );
    }
    assert!(reports.len() >= 3, "tiny-skia, PDF and SVG are required");
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_17_the_real_deck_journey_has_an_exact_loss_vector() {
    const NAME: &str = "17-real-deck";
    let bytes = input(
        NAME,
        "input.pptx",
        mjx_fixtures::fixture("text_levels.pptx"),
    );
    let mut page = deck_page(&bytes);
    assert_eq!(layout_vector(&page.losses), [0; 9]);
    let runs = page
        .list
        .commands()
        .filter(|command| matches!(command, Command::DrawGlyphs { .. }))
        .count();
    assert_eq!(runs, 14, "nine text runs and five bullet markers");
    assert_eq!(
        scene_vector(&page.list),
        [0, 0, 0, 14, 0],
        "every run's paint is defaulted"
    );
    assert_eq!(placeholders(&page.list), vec![]);
    let (drawn, _) = software(NAME, &mut page);
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0; 5], 0)
    );
    assert_eq!(
        drawn.page_losses(&page.list).vector(),
        vec![(LossCategory::Scene(SceneLossKind::TextPaintDefaulted), 14)],
        "the page's whole loss vector"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_18_the_real_worksheet_journey_has_an_exact_loss_vector() {
    const NAME: &str = "18-real-worksheet";
    let bytes = input(
        NAME,
        "input.xlsx",
        mjx_fixtures::fixture("style_resources.xlsx"),
    );
    let mut page = sheet_page(&bytes, 8.0, 5.0);
    assert_eq!(
        layout_vector(&page.losses),
        [0, 0, 0, 0, 0, 0, 0, 1, 1],
        "A1's up diagonal is dropped and its dash-dot-dot right edge is drawn solid"
    );
    assert_eq!(scene_vector(&page.list), [0; 5]);
    assert_eq!(
        placeholders(&page.list),
        vec![placeholder(
            px(0, 0, 581025, 190500),
            LABEL_NOT_READ,
            LossCategory::Layout(LayoutLossKind::DroppedByReader),
            &[0, 0]
        )]
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0; 5], 1)
    );
    assert_labelled(&pixels, &[px(0, 0, 581025, 190500)]);
    assert_eq!(
        drawn.page_losses(&page.list).vector(),
        vec![
            (LossCategory::Layout(LayoutLossKind::DroppedByReader), 1),
            (LossCategory::Layout(LayoutLossKind::ValueApproximated), 1),
        ],
        "the page's whole loss vector"
    );
}

#[test]
#[ignore = "render test: run with --ignored (MJXOFF-299)"]
fn rc02_19_an_unresolved_worksheet_outline_is_a_placeholder_not_a_failure() {
    const NAME: &str = "19-unresolved-outline";
    let bytes = input(NAME, "input.xlsx", inputs::sheet_chart());
    let mut page = sheet_page(&bytes, 6.0, 4.0);
    let (handle, rect) = page
        .tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Shape(shape) => Some((shape.geometry.number(), node.rect())),
            _ => None,
        })
        .expect("the chart's first axis is a shape fragment");
    let within = SceneRect::new(
        (rect.left.emu() as f64 / EMU_PER_PIXEL) as f32,
        (rect.top.emu() as f64 / EMU_PER_PIXEL) as f32,
        (rect.right.emu() as f64 / EMU_PER_PIXEL) as f32,
        (rect.bottom.emu() as f64 / EMU_PER_PIXEL) as f32,
    );
    let answer = SheetGeometry::new()
        .outline(handle, within)
        .map(|outline| (outline.provenance, outline.label));
    assert_eq!(
        answer.ok(),
        Some((OutlineProvenance::Placeholder, LABEL_OUTLINE.to_owned())),
        "the worksheet's geometry answers a stand-in instead of refusing handle {handle}"
    );
    let (drawn, pixels) = software(NAME, &mut page);
    assert_eq!(
        (painter_vector(&drawn), drawn.loss_placeholders),
        ([0; 5], 1),
        "the chart is one placeholder and no outline request fails the frame"
    );
    assert_labelled(&pixels, &[px(581025, 190500, 3486150, 2286000)]);
    assert_eq!(
        page.list
            .placeholder_at(200.0, 120.0)
            .map(|found| found.label),
        Some(LABEL_CHART.to_owned())
    );
}
