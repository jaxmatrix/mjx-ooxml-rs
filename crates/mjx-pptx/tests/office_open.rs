//! The Phase-2 exit gate: a `.pptx` we *construct* must actually open in a real Office
//! implementation. We drive LibreOffice headless to convert the deck to PDF and assert a valid PDF
//! came out — soffice's exit code is unreliable, so the produced PDF is the real signal that the
//! document parsed and rendered.
//!
//! When no `soffice`/`libreoffice` binary is found the test **skips** (prints a notice and passes),
//! so the suite stays green on machines without LibreOffice. CI sets `MJX_REQUIRE_SOFFICE=1`, which
//! turns a missing binary into a hard failure so coverage can never silently disappear.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use mjx_dml::{
    Angle, BlipFillMode, ColorSpec, EffectListSpec, Emu, FillSpec, Fraction, GlowEffect,
    GradientStopSpec, LineCap, LineDash, LineJoin, LineSpec, LineWidth, OuterShadowEffect,
    PatternType, PresetLineDash, RectangleAlignment, SchemeColor, ShapeGeometry,
};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_pptx::{Presentation, ShapeBounds, Surface};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// Locates a LibreOffice command, searching `PATH` then a few well-known install locations.
fn find_soffice() -> Option<PathBuf> {
    let names = ["soffice", "libreoffice"];
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let known = [
        "/usr/bin/soffice",
        "/usr/bin/libreoffice",
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        "/opt/libreoffice/program/soffice",
    ];
    known.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// Result of an attempted conversion.
enum Outcome {
    /// The deck converted to a valid PDF.
    Opened,
    /// No LibreOffice was available and the environment did not require it.
    Skipped,
}

/// A private working directory under the system temp dir, removed on drop.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("mjx_office_{tag}_{}", std::process::id()));
        // Fresh: clear any leftovers from a previous crashed run.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create work dir");
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Converts `pptx` to PDF with LibreOffice and asserts a valid PDF resulted. `tag` names the
/// temporary working directory. Skips (returning [`Outcome::Skipped`]) when LibreOffice is absent
/// unless `MJX_REQUIRE_SOFFICE` is set, in which case a missing binary panics.
fn convert_opens(pptx: &[u8], tag: &str) -> Outcome {
    let Some(soffice) = find_soffice() else {
        if std::env::var_os("MJX_REQUIRE_SOFFICE").is_some() {
            panic!("MJX_REQUIRE_SOFFICE is set but no soffice/libreoffice binary was found");
        }
        eprintln!("skipping office-open test `{tag}`: no soffice/libreoffice on this machine");
        return Outcome::Skipped;
    };

    let work = WorkDir::new(tag);
    let input = work.path().join("input.pptx");
    std::fs::write(&input, pptx).expect("write input pptx");
    let profile = work.path().join("profile");
    let user_installation = format!("-env:UserInstallation=file://{}", profile.display());

    let mut child = Command::new(&soffice)
        .arg("--headless")
        .arg("--norestore")
        .arg(&user_installation)
        .arg("--convert-to")
        .arg("pdf:impress_pdf_Export")
        .arg("--outdir")
        .arg(work.path())
        .arg(&input)
        .spawn()
        .unwrap_or_else(|e| panic!("spawning {}: {e}", soffice.display()));

    // soffice may fork/detach; wait for the PDF to appear (or the child to exit) with a hard cap.
    let output_pdf = work.path().join("input.pdf");
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if output_pdf.is_file() {
            break;
        }
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Child exited; give the filesystem a moment, then stop waiting.
                if output_pdf.is_file() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
                break;
            }
            Ok(None) => {}
            Err(e) => panic!("waiting on soffice: {e}"),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "soffice did not produce {} within the timeout",
                output_pdf.display()
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.wait();

    let pdf = std::fs::read(&output_pdf)
        .unwrap_or_else(|e| panic!("reading produced pdf {}: {e}", output_pdf.display()));
    assert!(
        pdf.len() > 100,
        "produced pdf is implausibly small ({} bytes)",
        pdf.len()
    );
    assert!(
        pdf.starts_with(b"%PDF"),
        "produced file does not start with the %PDF signature"
    );
    Outcome::Opened
}

#[test]
fn unmodified_fixture_opens() {
    // Baseline: isolates "our edit broke it" from "fixture or soffice is broken".
    let _ = convert_opens(&fixture("sample.pptx"), "baseline");
}

#[test]
fn effects_theme_fixture_opens() {
    // The hand-authored effects_theme.pptx (rich theme effectStyleLst + a shape with an effectRef)
    // must itself be a valid deck a real Office implementation opens.
    let _ = convert_opens(&fixture("effects_theme.pptx"), "effects_theme_baseline");
}

#[test]
fn deck_with_added_text_box_opens() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_text_box(
        0,
        "Canary\nLine two",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0),
    )
    .expect("add text box");
    let saved = pres.save().expect("save");
    // The constructed deck must open in LibreOffice.
    let _ = convert_opens(&saved, "added_text_box");
}

#[test]
fn deck_with_added_shape_opens() {
    // Constructs an autoshape via add_shape + set_shape_geometry (a rounded rectangle with a custom
    // corner radius) and checks the deck opens in LibreOffice — exercises the geometry write path
    // end-to-end through a real Office implementation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    pres.set_shape_geometry(
        0,
        idx,
        ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.3),
        },
    )
    .expect("set geometry");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "added_shape");
}

#[test]
fn deck_with_filled_shapes_opens() {
    // Adds autoshapes with a gradient and a preset pattern fill and checks the deck opens in
    // LibreOffice — exercises the fill write path end-to-end through a real Office implementation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");

    let gradient = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 1.5),
        )
        .expect("add gradient shape");
    pres.set_shape_fill(
        0,
        gradient,
        &FillSpec::linear_gradient(
            vec![
                GradientStopSpec {
                    position: Fraction::from_ratio(0.0),
                    color: ColorSpec::Srgb("FF0000".into()),
                },
                GradientStopSpec {
                    position: Fraction::from_ratio(1.0),
                    color: ColorSpec::Scheme(SchemeColor::Accent1),
                },
            ],
            Angle::from_degrees(45.0),
        ),
    )
    .expect("set gradient fill");

    let pattern = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 3.0, 3.0, 1.5),
        )
        .expect("add pattern shape");
    pres.set_shape_fill(
        0,
        pattern,
        &FillSpec::pattern(
            PatternType::Percent25,
            ColorSpec::Srgb("000000".into()),
            ColorSpec::Srgb("FFFFFF".into()),
        ),
    )
    .expect("set pattern fill");

    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "filled_shapes");
}

#[test]
fn deck_with_outlined_shape_opens() {
    // Adds an autoshape with both a solid fill and a dashed, round-capped outline and checks the deck
    // opens in LibreOffice — exercises the outline write path (and fill+outline coexistence in spPr)
    // end-to-end through a real Office implementation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 1.5),
        )
        .expect("add shape");
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("FFF2CC".into())))
        .expect("set fill");
    pres.set_shape_outline(
        0,
        idx,
        &LineSpec {
            width: Some(LineWidth::from_points(3.0)),
            cap: Some(LineCap::Round),
            fill: Some(FillSpec::Solid(ColorSpec::Scheme(SchemeColor::Accent1))),
            dash: Some(LineDash::Preset(PresetLineDash::Dash)),
            join: Some(LineJoin::Round),
            ..LineSpec::new()
        },
    )
    .expect("set outline");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "outlined_shape");
}

#[test]
fn deck_with_added_slide_opens() {
    // Exercises the whole add-slide construction (empty-slide template + the four package touches)
    // through a real Office implementation — the strongest check the new slide is valid.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_slide_with_text("Second slide", ShapeBounds::from_inches(1.0, 1.0, 5.0, 2.0))
        .expect("add slide with text");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "added_slide");
}

#[test]
fn deck_with_effect_shape_opens() {
    // Adds an autoshape with a solid fill plus an outer shadow and a glow, and checks the deck opens
    // in LibreOffice — exercises the effects write path (and fill+effect coexistence in spPr)
    // end-to-end through a real Office implementation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 1.5),
        )
        .expect("add shape");
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("FFF2CC".into())))
        .expect("set fill");
    pres.set_shape_effects(
        0,
        idx,
        &EffectListSpec {
            glow: Some(GlowEffect {
                color: ColorSpec::Scheme(SchemeColor::Accent1),
                radius: Some(Emu::from_points(5.0)),
            }),
            outer_shadow: Some(OuterShadowEffect {
                color: ColorSpec::Srgb("808080".into()),
                blur_radius: Some(Emu::from_points(4.0)),
                distance: Some(Emu::from_points(3.0)),
                direction: Some(Angle::from_degrees(45.0)),
                scale_x: None,
                scale_y: None,
                skew_x: None,
                skew_y: None,
                alignment: Some(RectangleAlignment::BottomRight),
                rotate_with_shape: Some(false),
            }),
            ..EffectListSpec::new()
        },
    )
    .expect("set effects");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "effect_shape");
}

/// A valid 2×2 truecolour PNG (76 bytes), inlined so no binary fixture is committed.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn deck_with_a_picture_filled_shape_opens() {
    // Adds an image part and fills a shape with it, and checks the deck opens in LibreOffice —
    // exercises the whole image path (media part, content type, slide relationship, a:blip@r:embed)
    // end-to-end through a real Office implementation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    let rel_id = pres.add_image(0, TINY_PNG).expect("add image");
    pres.set_shape_fill(
        0,
        idx,
        &FillSpec::Blip {
            rel_id,
            mode: BlipFillMode::Stretch,
        },
    )
    .expect("set picture fill");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "picture_filled_shape");
}

#[test]
fn deck_with_a_picture_shape_opens() {
    // Adds a real p:pic (not just a picture-filled autoshape) and gives it an outline through the
    // shared spPr surface, then checks the deck opens in LibreOffice — exercises the picture shape,
    // its blipFill relationship, and shape-kind-agnostic addressing end-to-end.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0))
        .expect("add picture");
    pres.set_shape_outline(
        0,
        picture,
        &LineSpec {
            fill: Some(FillSpec::solid(ColorSpec::Srgb("203864".into()))),
            width: Some(LineWidth::from_points(3.0)),
            ..LineSpec::new()
        },
    )
    .expect("outline the picture");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "picture_shape");
}

#[test]
fn layouts_fixture_opens() {
    // The hand-authored layouts.pptx (one master, three layouts, two slides on different layouts)
    // must itself be a valid deck a real Office implementation opens, before anything is asserted
    // about reading it.
    let _ = convert_opens(&fixture("layouts.pptx"), "layouts_baseline");
}

#[test]
fn deck_with_an_edited_layout_opens() {
    // Fills the *layout's* title placeholder and checks the deck opens — the slides built on that
    // layout inherit the fill, which is the point of addressing a layout at all.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.set_shape_fill(
        Surface::Layout(1),
        0,
        &FillSpec::solid(ColorSpec::Srgb("C00000".into())),
    )
    .expect("fill the layout's title");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "edited_layout");
}

#[test]
fn deck_with_a_slide_built_from_a_layout_opens() {
    // Builds a slide the way a caller is meant to: pick a layout, fill the placeholders it hands
    // over. Everything else — position, size, text style — inherits from the layout.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide"); // "Title and Content"
    pres.set_shape_text(slide, 0, 0, "Built from a layout")
        .expect("set the title");
    pres.set_shape_text(slide, 1, 0, "The placeholders came with the slide")
        .expect("set the body");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "slide_from_layout");
}
