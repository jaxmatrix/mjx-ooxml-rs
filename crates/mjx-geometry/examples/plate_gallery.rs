//! Draw all 186 presets onto one SVG sheet, for **a person** to look at.
//!
//! ```sh
//! cargo run -p mjx-geometry --example plate_gallery -- /tmp/presets.svg
//! ```
//!
//! # ⚠ This is not verification, and nothing gates on it
//!
//! MJXOFF-201 §6 is explicit and this file exists under that heading rather than in spite of it:
//! **rendering all 186 and finding them plausible is not evidence.** An agent that looks at a
//! picture and reports it correct has asserted nothing; the shapes it would call wrong are the ones
//! it already knows are wrong, and the ones it would call right include every shape whose guides are
//! wired to the wrong variable in a way that happens to look like a shape.
//!
//! The three checks that *are* evidence live in `tests/`, need no picture, and run on every commit:
//! `every_preset_is_structurally_sound.rs`, `the_third_route_is_the_parser.rs` and
//! `every_adjustment_moves_its_shape.rs`. **The authoritative visual check is Microsoft PowerPoint
//! on Windows, and it is the user's** (MJXOFF-155 §9 #5). This sheet is what makes that check
//! practical — 186 shapes on one page, each named, each drawn at its default adjustments beside its
//! text rectangle and its connection sites, so a person comparing against PowerPoint can find the
//! wrong one instead of opening 186 files.
//!
//! # What is drawn, and in what colour
//!
//! * the **outline**, filled pale and stroked — the shape itself;
//! * the shape's **box**, as a thin dashed frame, so a shape that does not fill its box (40 of them)
//!   or reaches outside it (18) is visible as such rather than merely as an odd shape;
//! * the **text rectangle** where the shape declares one (181 do), as a dotted rectangle — this is
//!   the surface a bounding-box fallback would make invisible, so it is drawn separately from the
//!   box it must not be confused with; and
//! * the **connection sites** (856 across 173 shapes) as small marks with a tick in the outgoing
//!   direction, because a site without its angle is a point and the angle is what an elbow connector
//!   reads.

use std::fmt::Write as _;

use mjx_geometry::{
    preset_connection_sites, preset_outline, preset_text_rectangle, seeded_shapes, Size,
    TextRectangle,
};
use mjx_scene::{PathCommand, SceneRect};

/// How wide one plate is, in SVG user units, and how tall.
const PLATE: (f32, f32) = (160.0, 150.0);
/// How much of a plate the shape's own box takes, leaving room for the caption.
const SHAPE_BOX: (f32, f32) = (120.0, 90.0);
/// How many plates to a row.
const COLUMNS: usize = 10;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "presets.svg".to_owned());

    let shapes = seeded_shapes();
    let rows = shapes.len().div_ceil(COLUMNS);
    let (width, height) = (PLATE.0 * COLUMNS as f32, PLATE.1 * rows as f32 + 40.0);

    let mut svg = String::new();
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    )?;
    svg.push_str(r##"<rect width="100%" height="100%" fill="#ffffff"/>"##);
    write!(
        svg,
        r##"<text x="12" y="26" font-family="system-ui, sans-serif" font-size="18" fill="#111111">{} preset shapes at their default adjustments — not a verification, see the module documentation</text>"##,
        shapes.len()
    )?;

    // One EMU per point, and a box whose aspect is not one: a square plate would hide an axis swap.
    let extents = Size::from_emu((SHAPE_BOX.0 as i64) * 12_700, (SHAPE_BOX.1 as i64) * 12_700);

    for (index, definition) in shapes.iter().enumerate() {
        let (column, row) = (index % COLUMNS, index / COLUMNS);
        let origin = (
            column as f32 * PLATE.0 + (PLATE.0 - SHAPE_BOX.0) / 2.0,
            row as f32 * PLATE.1 + 50.0,
        );
        let within = SceneRect::new(
            origin.0,
            origin.1,
            origin.0 + SHAPE_BOX.0,
            origin.1 + SHAPE_BOX.1,
        );

        // The box, dashed, so a shape that does not fill it says so.
        write!(
            svg,
            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="#c8cdd4" stroke-width="0.6" stroke-dasharray="3 3"/>"##,
            within.left,
            within.top,
            within.width(),
            within.height()
        )?;

        let token = definition.preset.to_wire();
        match preset_outline(definition.preset, extents, &[], within) {
            Ok(outline) => {
                write!(
                    svg,
                    r##"<path d="{}" fill="#dbe6f5" fill-rule="nonzero" stroke="#2b5aa8" stroke-width="1"/>"##,
                    path_data(&outline.commands)
                )?;
            }
            Err(error) => {
                write!(
                    svg,
                    r##"<text x="{}" y="{}" font-family="system-ui, sans-serif" font-size="8" fill="#b03030">{}</text>"##,
                    within.left + 4.0,
                    within.top + 14.0,
                    escaped(&format!("{error}"))
                )?;
            }
        }

        if let Ok(TextRectangle::Declared(rectangle)) =
            preset_text_rectangle(definition.preset, extents, &[], within)
        {
            write!(
                svg,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="#c07a2a" stroke-width="0.6" stroke-dasharray="1 2"/>"##,
                rectangle.left,
                rectangle.top,
                rectangle.width(),
                rectangle.height()
            )?;
        }

        if let Ok(sites) = preset_connection_sites(definition.preset, extents, &[], within) {
            for site in sites {
                let radians = site.angle.radians();
                write!(
                    svg,
                    r##"<circle cx="{}" cy="{}" r="1.3" fill="#1c8a4a"/><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#1c8a4a" stroke-width="0.6"/>"##,
                    site.position.x,
                    site.position.y,
                    site.position.x,
                    site.position.y,
                    site.position.x + 5.0 * radians.cos() as f32,
                    site.position.y + 5.0 * radians.sin() as f32
                )?;
            }
        }

        write!(
            svg,
            r##"<text x="{}" y="{}" font-family="ui-monospace, monospace" font-size="8" fill="#333333" text-anchor="middle">{}</text>"##,
            within.left + within.width() / 2.0,
            within.bottom + 14.0,
            escaped(token)
        )?;
    }

    svg.push_str("</svg>");
    std::fs::write(&destination, svg)?;
    println!(
        "{} plates written to {destination} — a sheet for a person to compare against PowerPoint, \
         and not a check anything gates on",
        shapes.len()
    );
    Ok(())
}

/// A command list as an SVG `d` attribute.
fn path_data(commands: &[PathCommand]) -> String {
    let mut data = String::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo(at) => {
                let _ = write!(data, "M{} {} ", at.x, at.y);
            }
            PathCommand::LineTo(at) => {
                let _ = write!(data, "L{} {} ", at.x, at.y);
            }
            PathCommand::QuadraticTo { control, end } => {
                let _ = write!(data, "Q{} {} {} {} ", control.x, control.y, end.x, end.y);
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                let _ = write!(
                    data,
                    "C{} {} {} {} {} {} ",
                    first_control.x,
                    first_control.y,
                    second_control.x,
                    second_control.y,
                    end.x,
                    end.y
                );
            }
            PathCommand::Close => data.push_str("Z "),
        }
    }
    data
}

/// The five characters XML text may not carry verbatim.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
