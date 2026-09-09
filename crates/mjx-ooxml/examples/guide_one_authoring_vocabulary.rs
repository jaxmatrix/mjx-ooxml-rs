//! The guide's **The authoring vocabulary is one vocabulary** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md`][guide] shows this code in three
//! languages, and every one of the three blocks is a *copy* of a file a test runner executes — this
//! one, plus `bindings/mjx-python/tests/guide_examples/one_authoring_vocabulary.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/one_authoring_vocabulary.mjs`. `cargo run -p xtask
//! -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # Why the workbook is the package it saves
//!
//! One [`mjx_ooxml::FillSpec`] value goes onto a shape in a deck and onto a chart series in a
//! workbook, which is two packages, and a guide example offers one for the two binding harnesses to
//! compare. It offers the **workbook**, deliberately: `set_shape_fill` is already authored and
//! compared byte for byte by `crates/mjx-ooxml/examples/build_a_deck.rs`, and
//! `set_chart_series_fill` is authored by no walkthrough at all — so the workbook is the half of
//! this example whose bytes nothing else in the repository checks.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_one_authoring_vocabulary -- out.xlsx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{ChartData, ChartKind, ColorSpec, Deck, FillSpec};
    use mjx_ooxml::{PresetShapeType, ResizingBehavior, ShapeBounds, SlideSize, Surface, Workbook};

    let navy = FillSpec::solid(ColorSpec::Srgb("1F3864".into()));

    // On a shape in a deck.
    let mut deck = Deck::blank(SlideSize::widescreen())?;
    let slide = Surface::Slide(deck.add_slide_from_layout(0)?);
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0);
    let shape = deck.add_shape(slide, PresetShapeType::Rectangle, bounds)?;
    deck.set_shape_fill(slide, shape.into(), &navy)?;
    assert!(deck.shape_fill(slide, shape.into())?.is_some());

    // The same value, on a chart series in a workbook.
    let mut workbook = Workbook::blank()?;
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1"])
        .series("North", [12.5]);
    let resizing = ResizingBehavior::MoveAndResizeWithAnchorCells;
    let anchor = workbook.add_chart(0, &chart, 1, 1, 7, 16, "Revenue", resizing)?;
    workbook.set_chart_series_fill(0, anchor, 0, &navy)?;
    assert!(workbook.chart_series_fill(0, anchor, 0)?.is_some());

    let saved = workbook.save()?;
    // guide-example:end

    write_output(&saved)
}

/// Where this example writes: its first argument, or `target/examples/` by default.
fn write_output(saved: &[u8]) -> Result<(), Box<dyn Error>> {
    let path = match std::env::args().nth(1) {
        Some(argument) => PathBuf::from(argument),
        None => {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            std::fs::create_dir_all(&directory)?;
            directory.join("facade_guide_one_authoring_vocabulary.xlsx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
