//! Every loss a painter meets is counted once, by the one lowering all four painters share (MJXOFF-299).
//!
//! MJX-STAND-IN: the placeholder geometry here answers the one unresolved outline the suite counts, and no document geometry.

mod common;

use mjx_paint::export::svg::SVG_UNEXPRESSED_EFFECTS;
use mjx_paint::{
    plan_frame_from, DrawReport, EncodedImages, FontSource, ImageSource, NoFonts, NoGlyphs,
    NoImages, OffscreenSurface, Painter, PainterLossKind, PdfPainter, PlanOptions, PlanSources,
    Resources, SoftwarePainter, SvgPainter, Viewport,
};
use mjx_scene::{
    Command, CompoundStroke, DashPattern, DeviceScale, DisplayList, EffectKind, FillRule, Geometry,
    Image, LineCap, LineEnd, LineEndShape, LineJoin, Paint, PathCommand, PlaceholderGeometry,
    SceneBuilder, ScenePoint, SceneRect, Stroke, StrokeAlignment, Tessellator,
};

// The report one lowering of `list` makes, handed these sources and options.
fn lowered(
    list: &DisplayList,
    images: &dyn ImageSource,
    fonts: &dyn FontSource,
    options: PlanOptions,
) -> DrawReport {
    let mut glyphs = NoGlyphs;
    let geometry = PlaceholderGeometry::new();
    let resources = Resources::new(&mut glyphs, &geometry, images).with_fonts(fonts);
    plan_frame_from(
        list,
        PlanSources::from_resources(&resources),
        &mut Tessellator::new(),
        options,
    )
    .expect("the list lowers")
    .report()
}

// A page holding one picture and, when asked, one line with an arrowhead at each end.
fn picture_and_arrow(arrow: bool) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 120.0, 80.0);
    let image = builder
        .add_image(Image::stretched(common::PICTURE_HANDLE))
        .expect("a picture");
    builder
        .push(Command::DrawImage {
            image,
            destination: SceneRect::new(10.0, 10.0, 50.0, 40.0),
        })
        .expect("a picture draw");
    if arrow {
        let paint = builder
            .add_paint(Paint::Solid(common::rgb(0, 0, 0)))
            .expect("an ink");
        let geometry = builder
            .add_geometry(&Geometry::path(
                vec![
                    PathCommand::MoveTo(ScenePoint::new(10.0, 60.0)),
                    PathCommand::LineTo(ScenePoint::new(110.0, 60.0)),
                ],
                FillRule::NonZero,
            ))
            .expect("a line");
        let stroke = builder
            .add_stroke(Stroke {
                paint,
                width: 3.0,
                cap: LineCap::Flat,
                join: LineJoin::Round,
                dash: DashPattern::Solid,
                alignment: StrokeAlignment::Centered,
                compound: CompoundStroke::Single,
                head: LineEnd {
                    shape: LineEndShape::Triangle,
                    ..LineEnd::default()
                },
                tail: LineEnd {
                    shape: LineEndShape::Arrow,
                    ..LineEnd::default()
                },
            })
            .expect("a stroke");
        builder
            .push(Command::StrokePath { geometry, stroke })
            .expect("a stroke draw");
    }
    builder.finish().expect("the page is well formed")
}

#[test]
fn a_picture_with_no_pixels_is_counted_under_a_placeholder_and_one_with_pixels_is_drawn() {
    let list = picture_and_arrow(false);
    let missing = lowered(&list, &NoImages, &NoFonts, PlanOptions::for_raster());
    assert_eq!(
        (
            missing.losses.count(PainterLossKind::ImageWithNoPixels),
            missing.loss_placeholders,
            missing.images
        ),
        (1, 1, 0)
    );

    let mut encoded = EncodedImages::new();
    encoded.insert(common::PICTURE_HANDLE, vec![0x89, 0x50, 0x4e, 0x47]);
    assert_eq!(
        encoded.bytes(common::PICTURE_HANDLE).map(<[u8]>::len),
        Some(4)
    );
    let undecoded = lowered(&list, &encoded, &NoFonts, PlanOptions::for_raster());
    assert_eq!(
        undecoded.losses.count(PainterLossKind::ImageWithNoPixels),
        1,
        "bytes nothing decodes are a picture with no pixels"
    );

    let supplied = lowered(
        &list,
        &common::OnePicture::new(),
        &NoFonts,
        PlanOptions::for_raster(),
    );
    assert_eq!(
        (
            supplied.losses.len(),
            supplied.loss_placeholders,
            supplied.images
        ),
        (0, 0, 1)
    );
}

#[test]
fn a_run_an_exporter_has_no_face_for_is_counted_and_a_rasteriser_loses_nothing() {
    let list = common::text_page(240.0, 120.0);
    let exporter = PlanOptions::for_vector().writing_text(true);
    let faceless = lowered(&list, &NoImages, &NoFonts, exporter);
    assert_eq!(
        (
            faceless.losses.count(PainterLossKind::GlyphRunNotEmbedded),
            faceless.loss_placeholders
        ),
        (1, 1)
    );
    let library = common::liberation_library();
    let faced = lowered(&list, &NoImages, &library, exporter);
    assert_eq!((faced.losses.len(), faced.loss_placeholders), (0, 0));
    let raster = lowered(&list, &NoImages, &NoFonts, PlanOptions::for_raster());
    assert_eq!((raster.losses.len(), raster.loss_placeholders), (0, 0));
}

#[test]
fn an_effect_the_painter_cannot_express_is_counted_under_a_placeholder() {
    let list = common::one_shape_under(100.0, 100.0, EffectKind::InnerShadow);
    let svg = SVG_UNEXPRESSED_EFFECTS
        .into_iter()
        .fold(PlanOptions::for_vector(), PlanOptions::not_expressing);
    let unexpressed = lowered(&list, &NoImages, &NoFonts, svg);
    assert_eq!(
        (
            unexpressed.losses.count(PainterLossKind::EffectUnsupported),
            unexpressed.loss_placeholders
        ),
        (1, 1)
    );
    let expressed = lowered(&list, &NoImages, &NoFonts, PlanOptions::for_raster());
    assert_eq!(
        (expressed.losses.len(), expressed.loss_placeholders),
        (0, 0)
    );
}

#[test]
fn every_arrowhead_is_counted_and_none_draws_a_placeholder() {
    let report = lowered(
        &picture_and_arrow(true),
        &common::OnePicture::new(),
        &NoFonts,
        PlanOptions::for_raster(),
    );
    assert_eq!(
        (
            report.losses.count(PainterLossKind::LineEndNotDrawn),
            report.losses.len(),
            report.loss_placeholders
        ),
        (2, 2, 0)
    );
}

#[test]
fn an_unresolved_outline_is_counted_as_the_stand_in_it_draws() {
    let report = lowered(
        &common::one_unresolved_shape(100.0, 100.0),
        &NoImages,
        &NoFonts,
        PlanOptions::for_raster(),
    );
    assert_eq!(
        (
            report.losses.count(PainterLossKind::OutlineUnresolved),
            report.placeholders,
            report.loss_placeholders
        ),
        (1, 1, 1)
    );
}

// What one painter reports drawing `list` with no pictures and no faces.
fn drawn(painter: &mut dyn Painter, list: &DisplayList) -> DrawReport {
    let mut host = OffscreenSurface::new(120, 80, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut glyphs = NoGlyphs;
    let geometry = PlaceholderGeometry::new();
    let report = {
        let mut resources = Resources::new(&mut glyphs, &geometry, &NoImages);
        painter
            .draw(&frame, list, &mut resources)
            .expect("no loss fails a frame")
    };
    painter.end(frame).expect("the frame ends");
    report
}

#[test]
fn the_software_pdf_and_svg_painters_report_the_same_losses_for_one_list() {
    let list = picture_and_arrow(true);
    let reports = [
        ("tiny-skia", drawn(&mut SoftwarePainter::new(), &list)),
        ("pdf", drawn(&mut PdfPainter::new(), &list)),
        ("svg", drawn(&mut SvgPainter::new(), &list)),
    ];
    for (painter, report) in &reports {
        assert_eq!(
            (
                report.losses.count(PainterLossKind::ImageWithNoPixels),
                report.losses.count(PainterLossKind::LineEndNotDrawn),
                report.losses.len(),
                report.loss_placeholders
            ),
            (1, 2, 3, 1),
            "{painter}: one picture with no pixels, two arrowheads, one placeholder"
        );
    }
}
