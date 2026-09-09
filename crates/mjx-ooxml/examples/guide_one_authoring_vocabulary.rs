//! The guide's **The authoring vocabulary is one vocabulary** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md`][guide] shows this code in three
//! languages, and every one of the three blocks is a *copy* of a file a test runner executes — this
//! one, plus `bindings/mjx-python/tests/guide_examples/one_authoring_vocabulary.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/one_authoring_vocabulary.mjs`. `cargo run -p xtask
//! -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # It authors two packages, and both are compared
//!
//! One [`mjx_ooxml::FillSpec`] value goes onto a shape in a deck and onto a chart series in a
//! workbook. Both are offered, under `saved` and `saved_deck`, and both binding harnesses compare
//! both against this program part by part (MJXOFF-260).
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_one_authoring_vocabulary -- out.xlsx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

// guide-example:packages saved saved_deck
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
    let saved_deck = deck.save()?;
    // guide-example:end

    write_output(&[
        ("saved", saved.as_slice()),
        ("saved_deck", saved_deck.as_slice()),
    ])
}

/// Where this example writes: its first argument, or `target/examples/` by default.
///
/// The first package declared takes the path itself; a second has its binding's suffix inserted
/// before the extension, which is the rule `xtask::guide_examples::package_output_path` states once
/// and both binding harnesses apply when they look for the file to compare against.
fn write_output(packages: &[(&str, &[u8])]) -> Result<(), Box<dyn Error>> {
    let base = match std::env::args().nth(1) {
        Some(argument) => PathBuf::from(argument),
        None => {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            std::fs::create_dir_all(&directory)?;
            directory.join("facade_guide_one_authoring_vocabulary.xlsx")
        }
    };
    for (binding, bytes) in packages {
        let path = output_path_for(&base, binding);
        std::fs::write(&path, bytes)?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
    Ok(())
}

/// [`xtask::guide_examples::package_output_path`], restated here because a `cargo` example under
/// `mjx-ooxml` may not depend on `xtask` — the layering rule points downward only, and `xtask` is
/// outside the ranked graph entirely. `xtask/tests/guide_examples.rs` holds the two to the same
/// answer, so the restatement cannot drift.
fn output_path_for(base: &std::path::Path, binding: &str) -> PathBuf {
    let Some(suffix) = binding
        .strip_prefix("saved")
        .and_then(|rest| rest.strip_prefix('_'))
    else {
        return base.to_path_buf();
    };
    let stem = base
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let file = match base.extension() {
        Some(extension) => format!("{stem}.{suffix}.{}", extension.to_string_lossy()),
        None => format!("{stem}.{suffix}"),
    };
    base.with_file_name(file)
}
