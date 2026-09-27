//! RC04 render tests: a stated opacity reaches the pixels, through every painter (MJXOFF-243).
//!
//! # The gate is not "alpha round-trips"
//!
//! A colour whose alpha is always `0xff` is reached constantly at one value, so a suite that
//! supplied one opacity could not tell a carried channel from a constant — and an earlier child
//! shipped exactly that shape elsewhere in this programme. So every case below supplies **several
//! distinct opacities** and asserts the concrete pixel each one produces over a known background:
//! black at 25 %, 50 % and 75 % over white is `191`, `127` and `64`, which are three different
//! numbers no stuck implementation produces.
//!
//! # Premultiplication is a real hazard and is pinned here
//!
//! `mjx_paint::Pixels::rgba` is **premultiplied** — its own documentation says so, and says it was
//! documented the other way round until MJXOFF-164 — while `mjx_scene::Color` is a straight colour
//! with a separate alpha. Nothing but a test holds the two conventions apart, and getting them
//! confused darkens every translucent thing on the page by exactly the factor nobody notices.
//! [`rc04_02_the_display_list_is_straight_and_the_readback_is_premultiplied`] is that test.
//!
//! **Its red byte is `0x10` and was written `0x0F`** (MJXOFF-243, at implementation). Premultiplying
//! `0x1F` by `0x80` is `31 * 128/255 = 15.57`, and every float-to-byte conversion in `mjx-paint`
//! rounds — `gradient.rs`, `software/compose.rs` and `software/mod.rs` all spell it
//! `(channel.clamp(0.0, 1.0) * 255.0).round()`. `0x0F` is what `31 / 2` gives, which is the
//! arithmetic of the sentence rather than of the painter, and it is the one channel of the four
//! where rounding and truncation differ. The assertion is unweakened: it is still an exact
//! four-byte equality, and a **straight** readback would answer `0x1F` and fail it.
//!
//! # Every case owns its folder
//!
//! `tests/render/RC04-alpha/<NN>-<name>/` holds the input, a README saying what the case is, the
//! LibreOffice reference for **this** run and an empty `reference/windows.png` until the sitting.
//! Our own renders go to `output/`, which is git-ignored. LibreOffice is a change detector and not
//! parity (`docs/validation/07-the-reference-pack.md`).

use std::path::{Path, PathBuf};

use mjx_dml::Size;
use mjx_layout::{BoxModel, PageIndex};
use mjx_layout_pptx::{constraints_for, PageCatalogue, SlideBoxModel, SlideDeck};
use mjx_paint::{
    DrawReport, EncodedImages, FaceLibrary, OffscreenSurface, PaintError, Painter, PdfPainter,
    Pixels, Resources, SoftwarePainter, SvgPainter, Viewport, WgpuPainter,
};
use mjx_pptx::{Package, PartName, Presentation, SlideSize, Surface};
use mjx_reference_pack::outlines::shape_outline;
use mjx_scene::{build_page, Command, DisplayList, Paint, SceneOptions};
use mjx_scene_pptx::{SlideGeometry, SlideResources};
use mjx_text::{FontResolver, GlyphAtlas};

/// Unzoomed device pixels are 96 per inch, so one inch is 914400 EMU and one pixel is 9525.
const EMU_PER_INCH: i64 = 914400;
/// The pixels per inch every rectangle below is placed and then sampled at.
const PIXELS_PER_INCH: f64 = 96.0;

const NAMESPACES: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

/// The opacities the bars state, as (per-mille-of-a-percent wire value, the byte it becomes, the
/// grey black-over-white produces at it).
///
/// `0x40` is `round(0.25 * 255)` and `0xBF` is `round(0.75 * 255)`; the grey is
/// `round(255 * (1 - alpha/255))`, which is exact for all three because the painter composites
/// `dst = src + dst * (1 - a)` over an opaque background.
const BARS: [(&str, u8, u8); 5] = [
    ("0", 0x00, 0xFF),
    ("25000", 0x40, 0xBF),
    ("50000", 0x80, 0x7F),
    ("75000", 0xBF, 0x40),
    ("100000", 0xFF, 0x00),
];

// The folder a test reads its input from and writes its output into.
fn folder(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/render/RC04-alpha")
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
fn output(name: &str, file: &str, bytes: &[u8]) {
    let directory = folder(name).join("output");
    std::fs::create_dir_all(&directory).expect("the output folder is creatable");
    std::fs::write(directory.join(file), bytes).expect("the output is writable");
}

// The bundled faces only, so a render does not depend on the machine.
fn resolver() -> FontResolver {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&fonts)
        .expect("the committed faces index")
        .build()
}

// A one-slide widescreen deck whose shape tree is exactly `shapes`.
fn deck(shapes: &str) -> Vec<u8> {
    let mut blank = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    blank.add_slide_from_layout(0).expect("one slide");
    let mut package = Package::open(&blank.save().expect("it saves")).expect("it reopens");
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {NAMESPACES}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    );
    package
        .replace_part_bytes(
            &PartName::new("/ppt/slides/slide1.xml").expect("a part name"),
            slide.into_bytes(),
        )
        .expect("the slide is replaceable");
    package.save().expect("the deck saves")
}

// One unoutlined rectangle at (x, y) inches, `width` by `height` inches, stating `fill`.
fn rectangle(
    id: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: &str,
    effects: &str,
) -> String {
    let emu = |inches: f64| (inches * EMU_PER_INCH as f64).round() as i64;
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Bar {id}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{fill}<a:ln><a:noFill/></a:ln>{effects}</p:spPr></p:sp>"#,
        emu(x),
        emu(y),
        emu(width),
        emu(height)
    )
}

// A solid fill of `hex`, at `alpha` thousandths of a per cent, or opaque when `alpha` is `None`.
fn solid(hex: &str, alpha: Option<&str>) -> String {
    match alpha {
        Some(alpha) => format!(
            r#"<a:solidFill><a:srgbClr val="{hex}"><a:alpha val="{alpha}"/></a:srgbClr></a:solidFill>"#
        ),
        None => format!(r#"<a:solidFill><a:srgbClr val="{hex}"/></a:solidFill>"#),
    }
}

// A page on its way to a painter, with everything a painter borrows.
struct Page {
    list: DisplayList,
    atlas: GlyphAtlas,
    geometry: SlideGeometry,
    images: EncodedImages,
    width: u32,
    height: u32,
}

// Slide 0 of a deck, laid out and built into a display list, with the production outline reader.
fn deck_page(bytes: &[u8]) -> Page {
    let mut presentation = Presentation::open(bytes).expect("the input opens");
    let read = SlideDeck::read(&mut presentation).expect("the deck reads");
    let constraints = constraints_for(&read);
    let mut model = SlideBoxModel::new(resolver());
    let page = model
        .layout_page(&read, PageIndex::new(0), &constraints, None)
        .expect("the slide lays out");
    let mut geometry = SlideGeometry::new();
    geometry.register_all(model.catalogue(), |request| {
        shape_outline(
            &mut presentation,
            Surface::Slide(request.surface_index as usize),
            &request.shape,
            Size::from_emu(request.rect.width().emu(), request.rect.height().emu()),
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
    .expect("the fragment tree becomes a display list");
    let (width, height) = list.page_size();
    Page {
        list,
        atlas,
        geometry,
        images,
        width: width.ceil().max(1.0) as u32,
        height: height.ceil().max(1.0) as u32,
    }
}

// The encoded bytes of every picture the page asks for, read out of the package itself.
fn package_images(presentation: &mut Presentation, catalogue: &PageCatalogue) -> EncodedImages {
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
fn paint(painter: &mut dyn Painter, page: &mut Page) -> (DrawReport, Option<Pixels>) {
    let fonts = FaceLibrary::new();
    let mut host = OffscreenSurface::new(page.width, page.height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter
        .begin(&mut host, viewport)
        .expect("the frame begins");
    let drawn = {
        let mut resources =
            Resources::new(&mut page.atlas, &page.geometry, &page.images).with_fonts(&fonts);
        painter
            .draw(&frame, &page.list, &mut resources)
            .expect("no loss fails a frame")
    };
    painter.end(frame).expect("the frame ends");
    let pixels = painter.read_pixels().expect("readback answers");
    (drawn, pixels)
}

// Draws the page through the software painter and keeps the PNG in the output folder.
fn software(name: &str, page: &mut Page) -> Pixels {
    let mut painter = SoftwarePainter::new();
    let (_, pixels) = paint(&mut painter, page);
    let pixels = pixels.expect("the software painter reads back");
    output(
        name,
        "software.png",
        &mjx_paint::export::png(pixels.width, pixels.height, &pixels.rgba),
    );
    pixels
}

// Renders slide 0 of `bytes` through the software painter, writing the plate into `name`'s folder.
fn software_render(name: &str, bytes: &[u8]) -> Pixels {
    let mut page = deck_page(bytes);
    software(name, &mut page)
}

// The pixel at (x, y) inches from the page's top-left corner.
#[track_caller]
fn at(pixels: &Pixels, x: f64, y: f64) -> [u8; 4] {
    let (px, py) = (
        (x * PIXELS_PER_INCH).round() as u32,
        (y * PIXELS_PER_INCH).round() as u32,
    );
    pixels
        .pixel(px, py)
        .unwrap_or_else(|| panic!("({x} in, {y} in) is ({px}, {py}) px, outside the page"))
}

// The five-bar deck's shape tree: a white page with black bars at five opacities on it.
fn five_bars() -> String {
    let mut shapes = rectangle(2, 0.0, 0.0, 13.333, 7.5, &solid("FFFFFF", None), "");
    for (index, (wire, _, _)) in BARS.iter().enumerate() {
        let alpha = (*wire != "100000").then_some(*wire);
        shapes.push_str(&rectangle(
            10 + index as u32,
            0.5 + 2.0 * index as f64,
            1.0,
            1.5,
            2.0,
            &solid("000000", alpha),
            "",
        ));
    }
    shapes
}

// Where the centre of bar `index` is, in inches.
fn bar_centre(index: usize) -> (f64, f64) {
    (0.5 + 2.0 * index as f64 + 0.75, 2.0)
}

/// **Five opacities, five different greys.** Black over white at 0, 25, 50, 75 and 100 per cent.
#[test]
#[ignore = "render test: run with --ignored (MJXOFF-243)"]
fn rc04_01_five_opacities_paint_five_different_greys() {
    const NAME: &str = "01-mid-alpha-fill";
    let bytes = input(NAME, "input.pptx", deck(&five_bars()));
    let pixels = software_render(NAME, &bytes);

    let painted: Vec<u8> = BARS
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let (x, y) = bar_centre(index);
            at(&pixels, x, y)[0]
        })
        .collect();
    let expected: Vec<u8> = BARS.iter().map(|(_, _, grey)| *grey).collect();
    assert_eq!(
        painted, expected,
        "black over white at 0 %, 25 %, 50 %, 75 % and 100 % must paint {expected:?}; a build that \
         carried one opacity everywhere paints one value five times"
    );

    // The mid values are what a stuck-at-one-value implementation cannot produce, so they are
    // asserted to differ from each other as well as from the two ends.
    assert!(
        painted[1] > painted[2] && painted[2] > painted[3],
        "25 % must be lighter than 50 %, which must be lighter than 75 %; the three came back as \
         {:?}",
        &painted[1..4]
    );
    assert_eq!(
        at(&pixels, 0.2, 0.2),
        [0xFF, 0xFF, 0xFF, 0xFF],
        "the page behind the bars is opaque white, which is what every blend above is measured \
         against"
    );
}

/// **The display list carries a straight colour; the readback is premultiplied.**
///
/// Two conventions meet here and nothing but this test holds them apart. `mjx_scene::Color` is a
/// colour and a separate alpha — `1F3864` at 50 % is `1F 38 64 80`, with the channels untouched.
/// `mjx_paint::Pixels::rgba` is premultiplied, so the same colour drawn on a transparent page reads
/// back as `10 1C 32 80`: each channel scaled by the alpha. An implementation that confused the two
/// would darken every translucent thing on the page by exactly the factor nobody notices.
#[test]
#[ignore = "render test: run with --ignored (MJXOFF-243)"]
fn rc04_02_the_display_list_is_straight_and_the_readback_is_premultiplied() {
    const NAME: &str = "02-premultiplication";
    // No white background: the page stays transparent, so the readback is the source alone.
    let bytes = input(
        NAME,
        "input.pptx",
        deck(&rectangle(
            2,
            1.0,
            1.0,
            4.0,
            3.0,
            &solid("1F3864", Some("50000")),
            "",
        )),
    );
    let mut page = deck_page(&bytes);

    let solids: Vec<mjx_scene::Color> = page
        .list
        .commands()
        .filter_map(|command| match command {
            Command::FillPath { paint, .. } => page.list.paint(paint),
            _ => None,
        })
        .filter_map(|paint| match paint {
            Paint::Solid(color) => Some(color),
            _ => None,
        })
        .collect();
    assert_eq!(
        solids,
        vec![mjx_scene::Color {
            red: 0x1F,
            green: 0x38,
            blue: 0x64,
            alpha: 0x80,
        }],
        "the display list must carry the colour **straight** — the channels as the document states \
         them, with the opacity beside them. A premultiplied `10 1C 32 80` here would be the \
         painter's convention leaking one stage down."
    );

    let pixels = software(NAME, &mut page);
    assert_eq!(
        at(&pixels, 3.0, 2.5),
        [0x10, 0x1C, 0x32, 0x80],
        "`Pixels::rgba` is premultiplied, so a half-opaque `1F3864` on a transparent page reads \
         back with each channel scaled by the alpha: `31 * 128/255` rounds to `0x10`, `56 * 128/255` \
         to `0x1C` and `100 * 128/255` to `0x32`. A straight `1F 38 64 80` here means the painter \
         is writing the other convention into a buffer everything else reads as premultiplied."
    );
    assert_eq!(
        at(&pixels, 0.2, 0.2),
        [0x00, 0x00, 0x00, 0x00],
        "the page outside the rectangle is untouched"
    );
}

/// **Every painter agrees about the opacity**, in the vocabulary each of them has for it.
///
/// The two rasterisers answer in pixels; the two exporters answer in the attribute the format uses —
/// `fill-opacity` in SVG and an `ExtGState`'s `/ca` in PDF — because a screenshot of a PDF would be
/// a test of whatever rasterised it. `number` writes three decimals with the trailing zeros
/// trimmed, so `0x40` is `0.251`, `0x80` is `0.502` and `0xBF` is `0.749`.
#[test]
#[ignore = "render test: run with --ignored (MJXOFF-243)"]
fn rc04_03_four_painters_carry_the_same_opacities() {
    const NAME: &str = "03-four-painters";
    let bytes = input(NAME, "input.pptx", deck(&five_bars()));
    let expected: Vec<u8> = BARS.iter().map(|(_, _, grey)| *grey).collect();

    let greys = |pixels: &Pixels| -> Vec<u8> {
        BARS.iter()
            .enumerate()
            .map(|(index, _)| {
                let (x, y) = bar_centre(index);
                at(pixels, x, y)[0]
            })
            .collect()
    };

    let mut page = deck_page(&bytes);
    let pixels = software(NAME, &mut page);
    assert_eq!(greys(&pixels), expected, "tiny-skia");

    let mut svg = SvgPainter::new();
    let (_, _) = paint(&mut svg, &mut deck_page(&bytes));
    let document = svg.document().expect("a finished SVG").to_owned();
    output(NAME, "export.svg", document.as_bytes());
    for opacity in ["0.251", "0.502", "0.749"] {
        assert!(
            document.contains(&format!("fill-opacity=\"{opacity}\"")),
            "the SVG export states no `fill-opacity=\"{opacity}\"`; the three translucent bars must \
             each carry their own, and an export that wrote `1` everywhere draws five identical \
             black bars"
        );
    }

    let mut pdf = PdfPainter::new();
    let (_, _) = paint(&mut pdf, &mut deck_page(&bytes));
    let finished = pdf.document().expect("a finished PDF").to_vec();
    output(NAME, "export.pdf", &finished);
    let text = String::from_utf8_lossy(&finished).into_owned();
    for opacity in ["0.251", "0.502", "0.749"] {
        assert!(
            text.contains(&format!("/ca {opacity}")),
            "the PDF export states no `/ca {opacity}`; a translucent fill is an `ExtGState` with \
             `/ca` and `/CA`, and without it the bar prints solid"
        );
    }

    match WgpuPainter::offscreen() {
        Ok(mut gpu) => {
            let (_, pixels) = paint(&mut gpu, &mut deck_page(&bytes));
            let pixels = pixels.expect("the wgpu painter reads back");
            assert_eq!(greys(&pixels), expected, "wgpu on {}", gpu.backend());
        }
        Err(PaintError::NoAdapter { .. } | PaintError::Device(_)) => {
            println!(
                "SKIPPED {NAME} (wgpu): no graphics adapter on this machine. The other three \
                 painters ran; set MJX_REQUIRE_GPU=1 to make this a failure instead."
            );
            assert!(
                std::env::var("MJX_REQUIRE_GPU").is_err(),
                "MJX_REQUIRE_GPU is set, so a missing graphics device is a failure and not a skip"
            );
        }
        Err(other) => panic!("the wgpu painter could not be built: {other}"),
    }
}

/// **A theme shadow renders translucent**, which is the loss this whole ticket was named for.
///
/// The standard Office theme puts `<a:alpha val="63000"/>` on the shadow of every styled shape, so
/// a shadow drawn without the channel is a solid slab. The deck below states the same 63 % shadow
/// beside a 100 % one, over an opaque white page: the two must differ by most of the range, and the
/// translucent one must land on the blend — `round(255 * (1 - 0.63))` is `94`.
#[test]
#[ignore = "render test: run with --ignored (MJXOFF-243)"]
fn rc04_04_a_sixty_three_per_cent_shadow_is_not_a_solid_one() {
    const NAME: &str = "04-theme-shadow";
    // A shadow with no blur, so its interior is a flat area a pixel can be sampled from: the blur
    // radius is what makes a shadow's darkest pixel a question rather than a value.
    let shadow = |alpha: Option<&str>| {
        let colour = match alpha {
            Some(alpha) => {
                format!(r#"<a:srgbClr val="000000"><a:alpha val="{alpha}"/></a:srgbClr>"#)
            }
            None => r#"<a:srgbClr val="000000"/>"#.to_owned(),
        };
        format!(
            r#"<a:effectLst><a:outerShdw blurRad="0" dist="457200" dir="2700000" rotWithShape="0">{colour}</a:outerShdw></a:effectLst>"#
        )
    };
    let shapes = format!(
        "{}{}{}",
        rectangle(2, 0.0, 0.0, 13.333, 7.5, &solid("FFFFFF", None), ""),
        rectangle(
            10,
            1.0,
            1.0,
            3.0,
            2.0,
            &solid("FFFFFF", None),
            &shadow(Some("63000"))
        ),
        rectangle(
            11,
            7.0,
            1.0,
            3.0,
            2.0,
            &solid("FFFFFF", None),
            &shadow(None)
        ),
    );
    let bytes = input(NAME, "input.pptx", deck(&shapes));
    let pixels = software_render(NAME, &bytes);

    // The shadow is offset half an inch down and to the right, so a point just past the shape's
    // bottom-right corner is over the shadow and not over the shape.
    let translucent = at(&pixels, 4.25, 3.25);
    let opaque = at(&pixels, 10.25, 3.25);
    assert_eq!(
        opaque,
        [0x00, 0x00, 0x00, 0xFF],
        "the 100 % shadow must be solid black; it came back as {opaque:?}, so this case is not \
         sampling a shadow at all and the comparison below would mean nothing"
    );
    assert_eq!(
        translucent,
        [0x5E, 0x5E, 0x5E, 0xFF],
        "a 63 % black shadow over white is `round(255 * (1 - 161/255))` = 0x5E. Solid black here is \
         the defect RC04 exists to fix: a slab under every theme-styled shape instead of a soft \
         shadow."
    );
    assert!(
        translucent[0] as i32 - opaque[0] as i32 >= 80,
        "the translucent shadow and the solid one differ by {} levels; a difference this small \
         means the opacity barely reached the painter",
        translucent[0] as i32 - opaque[0] as i32
    );
}

/// **The corporate deck's overlay band lets the row beneath it through.**
///
/// RC03 pinned this as a loss: the band is `1F3864` at 35 % and painted opaque over the table's
/// third row, so the row was in the display list and not in the picture. The assertion is the blend
/// rather than "something changed": a pixel inside a third-row cell must be the band's colour mixed
/// 35 % over whatever that cell paints, which is measured from the same cell just below the band.
#[test]
#[ignore = "render test: run with --ignored (MJXOFF-243)"]
fn rc04_05_the_corporate_overlay_lets_the_row_beneath_it_through() {
    const NAME: &str = "05-corporate-overlay";
    let bytes = input(NAME, "input.pptx", mjx_fixtures::fixture("corporate.pptx"));
    let pixels = software_render(NAME, &bytes);

    // The band spans 0.6 in to 12.7 in horizontally and 3.25 in to 3.85 in vertically; the table
    // stands at 9.4 in to 12.8 in, its third row from about 3.23 in to 3.9 in. So a point at 3.5 in
    // is inside both, and one at 3.87 in is inside the row and below the band.
    let under_band = at(&pixels, 10.0, 3.5);
    let beside_band = at(&pixels, 10.0, 3.87);

    let band = [0x1F, 0x38, 0x64];
    assert_ne!(
        [under_band[0], under_band[1], under_band[2]],
        band,
        "the pixel inside the third row is the band's own colour, so the band painted opaque over \
         the row — which is exactly the loss RC03 pinned and RC04 removes"
    );

    let blended: Vec<u8> = (0..3)
        .map(|channel| {
            let over = f64::from(band[channel]) * 0.349_019_6;
            let under = f64::from(beside_band[channel]) * (1.0 - 0.349_019_6);
            (over + under).round() as u8
        })
        .collect();
    for channel in 0..3 {
        let difference = i32::from(under_band[channel]) - i32::from(blended[channel]);
        assert!(
            difference.abs() <= 2,
            "channel {channel} under the band is {} and the 35 % blend of {band:?} over the cell's \
             own {beside_band:?} is {}; the two must agree to within a rounding step",
            under_band[channel],
            blended[channel]
        );
    }
    assert_ne!(
        [beside_band[0], beside_band[1], beside_band[2]],
        [under_band[0], under_band[1], under_band[2]],
        "the sample under the band and the one beside it are identical, so the band drew nothing \
         and the blend above compared a colour with itself"
    );
}
