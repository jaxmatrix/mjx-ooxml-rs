//! The guide's **Charts: the same fifty method names on all three** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md`][guide] shows this code in three
//! languages, and every one of the three blocks is a *copy* of a file a test runner executes — this
//! one, plus `bindings/mjx-python/tests/guide_examples/the_same_chart_on_all_three.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/the_same_chart_on_all_three.mjs`. `cargo run -p
//! xtask -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # It authors two packages, and both are compared
//!
//! A chart on a slide and the same chart in a Word document — because that is the claim: one
//! description, two owners, one vocabulary. Both are offered, under `saved` and `saved_document`,
//! and both binding harnesses compare both against this program part by part (MJXOFF-260).
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_the_same_chart_on_all_three -- out.pptx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

// guide-example:packages saved saved_document
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{ChartData, ChartKind, Deck, Document, PageSize};
    use mjx_ooxml::{ShapeBounds, SlideSize, Surface};

    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("North", [12.5, 18.0, 21.5])
        .title("Quarterly revenue".to_owned());

    // A slide is a canvas in EMU, so a chart on one is laid out inside bounds.
    let mut deck = Deck::blank(SlideSize::widescreen())?;
    deck.add_slide()?;
    let slide = Surface::Slide(0);
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 5.0, 3.0);
    let shape = deck.add_chart(slide, &chart, bounds)?;

    // A Word drawing is inline in a paragraph, so it takes a width and a height.
    let mut document = Document::blank(PageSize::a4())?;
    let drawing = document.add_chart(0.into(), &chart, 4_572_000, 2_743_200, "Revenue")?;

    // Everything after the address is identical: the same question, the same answer.
    let on_slide = deck.chart_title(slide, shape.into())?;
    let in_document = document.chart_title(drawing)?;
    assert_eq!(on_slide.as_deref(), Some("Quarterly revenue"));
    assert_eq!(on_slide, in_document);
    assert_eq!(deck.chart_series(slide, shape.into())?.len(), 1);
    assert_eq!(document.chart_series(drawing)?.len(), 1);

    let saved = deck.save()?;
    let saved_document = document.save()?;
    // guide-example:end

    write_output(&[
        ("saved", saved.as_slice()),
        ("saved_document", saved_document.as_slice()),
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
            directory.join("facade_guide_the_same_chart_on_all_three.pptx")
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
