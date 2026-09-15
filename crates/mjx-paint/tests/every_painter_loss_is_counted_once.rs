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
    let mut host = OffscreenSurface::new(240, 160, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut glyphs = common::ChequeredAtlas::new();
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

// A page that meets every painter loss: a picture with no pixels, a line with two arrowheads, an unresolved outline, a run of glyphs and an inner shadow.
fn every_painter_loss() -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 240.0, 160.0);
    let image = builder
        .add_image(Image::stretched(common::PICTURE_HANDLE))
        .expect("a picture");
    builder
        .push(Command::DrawImage {
            image,
            destination: SceneRect::new(10.0, 10.0, 110.0, 70.0),
        })
        .expect("a picture draw");
    let ink = builder
        .add_paint(Paint::Solid(common::rgb(0, 0, 0)))
        .expect("an ink");
    let line = builder
        .add_geometry(&Geometry::path(
            vec![
                PathCommand::MoveTo(ScenePoint::new(10.0, 150.0)),
                PathCommand::LineTo(ScenePoint::new(110.0, 150.0)),
            ],
            FillRule::NonZero,
        ))
        .expect("a line");
    let arrowed = builder
        .add_stroke(Stroke {
            paint: ink,
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
        .push(Command::StrokePath {
            geometry: line,
            stroke: arrowed,
        })
        .expect("a stroke draw");
    let outline = builder
        .add_geometry(&Geometry::Unresolved {
            outline: 7,
            bounds: SceneRect::new(130.0, 10.0, 230.0, 70.0),
        })
        .expect("an unresolved shape");
    builder
        .push(Command::FillPath {
            geometry: outline,
            paint: ink,
        })
        .expect("a fill");
    let shadowed = builder
        .add_geometry(&common::box_path(SceneRect::new(130.0, 90.0, 230.0, 150.0)))
        .expect("a box");
    let shadow = builder
        .add_effect(mjx_scene::Effect::new(EffectKind::InnerShadow))
        .expect("an effect");
    builder
        .push(Command::PushEffect(shadow))
        .expect("an effect group");
    builder
        .push(Command::FillPath {
            geometry: shadowed,
            paint: ink,
        })
        .expect("a fill");
    builder.push(Command::Pop).expect("the effect closes");
    let run = builder
        .add_glyph_run(&common::stand_in_run(120.0))
        .expect("a run");
    builder
        .push(Command::DrawGlyphs { run, paint: ink })
        .expect("a run draw");
    builder.finish().expect("the page is well formed")
}

#[test]
fn every_painter_reports_the_painter_neutral_losses_alike() {
    // Painter-specific by design: a glyph run is not embedded only by PDF and SVG, which write text from faces, and an effect is unsupported only by SVG.
    const NEUTRAL: [PainterLossKind; 3] = [
        PainterLossKind::ImageWithNoPixels,
        PainterLossKind::LineEndNotDrawn,
        PainterLossKind::OutlineUnresolved,
    ];
    let list = every_painter_loss();
    let mut reports = vec![
        drawn(&mut SoftwarePainter::new(), &list),
        drawn(&mut PdfPainter::new(), &list),
        drawn(&mut SvgPainter::new(), &list),
    ];
    let mut names = vec![
        mjx_paint::SOFTWARE_PAINTER,
        mjx_paint::PDF_EXPORTER,
        mjx_paint::SVG_EXPORTER,
    ];
    match common::painter() {
        Ok(mut gpu) => {
            common::announce(
                "every_painter_reports_the_painter_neutral_losses_alike",
                &gpu,
            );
            reports.push(drawn(&mut gpu, &list));
            names.push(gpu.name());
        }
        Err(why) => common::skip(
            "every_painter_reports_the_painter_neutral_losses_alike",
            &why,
        ),
    }
    for (name, report) in names.iter().zip(&reports) {
        assert_eq!(
            NEUTRAL.map(|kind| report.losses.count(kind)),
            [1, 2, 1],
            "{name}: one picture with no pixels, two arrowheads and one unresolved outline"
        );
        let specific = (
            report.losses.count(PainterLossKind::GlyphRunNotEmbedded),
            report.losses.count(PainterLossKind::EffectUnsupported),
            report.loss_placeholders,
        );
        let expected = match *name {
            name if name == mjx_paint::PDF_EXPORTER => (1, 0, 3),
            name if name == mjx_paint::SVG_EXPORTER => (1, 1, 4),
            _ => (0, 0, 2),
        };
        assert_eq!(
            specific, expected,
            "{name}: the painter-specific losses and every placeholder drawn"
        );
        assert_eq!(
            report.page_losses(&list).vector().len(),
            NEUTRAL.len() + usize::from(expected.0 > 0) + usize::from(expected.1 > 0),
            "{name}: the page's whole vector is the painter's, since the list carries no loss of its own"
        );
    }
}

// A page of `width` by `height` whose one draw is a picture over `rect` with no pixels behind it.
fn one_missing_picture(width: f32, height: f32, rect: SceneRect) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let image = builder
        .add_image(Image::stretched(common::PICTURE_HANDLE))
        .expect("a picture");
    builder
        .push(Command::DrawImage {
            image,
            destination: rect,
        })
        .expect("a picture draw");
    builder.finish().expect("the page is well formed")
}

// The software painter's pixels for `list`, with no pictures and no faces.
fn software_pixels(list: &DisplayList, width: u32, height: u32) -> mjx_paint::Pixels {
    let mut painter = SoftwarePainter::new();
    let mut host = OffscreenSurface::new(width, height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut glyphs = NoGlyphs;
    let geometry = PlaceholderGeometry::new();
    {
        let mut resources = Resources::new(&mut glyphs, &geometry, &NoImages);
        painter
            .draw(&frame, list, &mut resources)
            .expect("no loss fails a frame");
    }
    painter.end(frame).expect("the frame ends");
    painter
        .read_pixels()
        .expect("readback answers")
        .expect("the software painter reads back")
}

// Label-ink pixels, opaque dark magenta, in the middle half of `rect`'s height, which is where a label is set.
fn label_ink(pixels: &mjx_paint::Pixels, rect: SceneRect) -> usize {
    let band_top = rect.top + rect.height() * 0.25;
    let band_bottom = rect.bottom - rect.height() * 0.25;
    let mut ink = 0;
    for y in band_top.ceil() as u32..band_bottom.floor() as u32 {
        for x in rect.left.ceil() as u32..rect.right.floor() as u32 {
            if let Some([red, green, blue, alpha]) = pixels.pixel(x, y) {
                if alpha >= 0xe0
                    && green <= 0x20
                    && (0x40..=0x80).contains(&red)
                    && red.abs_diff(blue) <= 8
                {
                    ink += 1;
                }
            }
        }
    }
    ink
}

#[test]
fn a_placeholder_label_is_ink_inside_its_box_in_the_software_painters_pixels() {
    let rect = SceneRect::new(20.0, 20.0, 180.0, 100.0);
    let pixels = software_pixels(&one_missing_picture(200.0, 120.0, rect), 200, 120);
    let ink = label_ink(&pixels, rect);
    assert!(
        ink >= 40,
        "{ink} label pixels in the middle band of the placeholder; its label must be readable in a PNG"
    );
    let outside = label_ink(&pixels, SceneRect::new(0.0, 0.0, 200.0, 20.0));
    assert_eq!(outside, 0, "the label stays inside its box");
}

// One unresolved outline, filled and then stroked, as a shape with both is drawn.
fn one_unresolved_shape_filled_and_stroked(width: f32, height: f32) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&Geometry::Unresolved {
            outline: 7,
            bounds: SceneRect::new(width * 0.15, height * 0.15, width * 0.85, height * 0.85),
        })
        .expect("an unresolved shape");
    let paint = builder
        .add_paint(Paint::Solid(common::rgb(0x00, 0x80, 0x00)))
        .expect("a paint");
    let stroke = builder
        .add_stroke_style(&mjx_scene::StrokeStyle::solid(3.0, common::rgb(0, 0, 0)))
        .expect("a stroke interns")
        .expect("a visible stroke");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder
        .push(Command::StrokePath { geometry, stroke })
        .expect("a stroke");
    builder.finish().expect("the scene is well formed")
}

#[test]
fn a_shape_filled_and_stroked_over_one_unresolved_outline_is_one_loss_and_one_placeholder() {
    let report = lowered(
        &one_unresolved_shape_filled_and_stroked(100.0, 100.0),
        &NoImages,
        &NoFonts,
        PlanOptions::for_raster(),
    );
    assert_eq!(
        (
            report.losses.count(PainterLossKind::OutlineUnresolved),
            report.loss_placeholders,
            report.placeholders
        ),
        (1, 1, 2),
        "one element, one loss and one placeholder, though both its draws use the stand-in"
    );
}

// An effect wrapping an opacity group wrapping one filled box.
fn an_effect_over_an_opacity_group(kind: EffectKind) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 100.0, 100.0);
    let shape = builder
        .add_geometry(&common::box_path(SceneRect::new(30.0, 30.0, 70.0, 60.0)))
        .expect("a shape");
    let ink = builder
        .add_paint(Paint::Solid(common::rgb(0x20, 0x80, 0x40)))
        .expect("an ink");
    let effect = builder
        .add_effect(mjx_scene::Effect::new(kind))
        .expect("an effect");
    builder
        .push(Command::PushEffect(effect))
        .expect("an effect group");
    builder
        .push(Command::PushOpacity(0.5))
        .expect("an opacity group");
    builder
        .push(Command::FillPath {
            geometry: shape,
            paint: ink,
        })
        .expect("a fill");
    builder.push(Command::Pop).expect("the opacity closes");
    builder.push(Command::Pop).expect("the effect closes");
    builder.finish().expect("the scene is well formed")
}

#[test]
fn an_effect_the_painter_cannot_express_over_nested_layers_is_still_drawn_as_a_placeholder() {
    let svg = SVG_UNEXPRESSED_EFFECTS
        .into_iter()
        .fold(PlanOptions::for_vector(), PlanOptions::not_expressing);
    let report = lowered(
        &an_effect_over_an_opacity_group(EffectKind::InnerShadow),
        &NoImages,
        &NoFonts,
        svg,
    );
    assert_eq!(
        (
            report.losses.count(PainterLossKind::EffectUnsupported),
            report.loss_placeholders
        ),
        (1, 1),
        "the effect's only content is a composited opacity layer, and it still covers the box"
    );
}
