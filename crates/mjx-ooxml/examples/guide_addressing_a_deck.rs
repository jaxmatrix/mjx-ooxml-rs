//! The guide's **`Deck`: a surface and a path** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/addressing.md`][guide] shows this code in three languages, and
//! every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/addressing_a_deck.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/addressing_a_deck.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! What it demonstrates is the two halves of a deck address: a [`mjx_ooxml::Surface`] says which
//! shape-bearing part, and a [`mjx_ooxml::ShapePath`] says which shape on it — including a shape
//! inside a group, which is the case the flat index space cannot express.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_addressing_a_deck -- out.pptx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

// guide-example:packages saved
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{Deck, PresetShapeType, ShapeBounds, ShapePath, SlideSize, Surface};

    let mut deck = Deck::blank(SlideSize::widescreen())?;
    // `add_slide_from_layout` would copy the layout's placeholders too.
    let slide = Surface::Slide(deck.add_slide()?);
    let rectangle = ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0);
    let ellipse = ShapeBounds::from_inches(4.0, 1.0, 2.0, 1.0);
    deck.add_shape(slide, PresetShapeType::Rectangle, rectangle)?;
    deck.add_shape(slide, PresetShapeType::Ellipse, ellipse)?;
    assert_eq!(deck.shape_count(slide)?, 2);

    // The group itself is one entry on the surface's index space.
    let group: ShapePath = deck.group_shapes(slide, &[0.into(), 1.into()])?;
    assert!(group.is_top_level());

    // Member 1 of that group, one step deeper.
    let member = group.child(1);
    assert_eq!(member.depth(), 2);
    assert_eq!(member.indices().len(), 2);
    assert!(!member.is_top_level());
    assert_eq!(member.parent(), Some(group.clone()));

    let saved = deck.save()?;
    // guide-example:end

    write_output(&saved)
}

/// Where this example writes: its first argument, or `target/examples/` by default.
///
/// The two binding harnesses run this program with an explicit path and compare what lands there,
/// part by part, against what they produced themselves.
fn write_output(saved: &[u8]) -> Result<(), Box<dyn Error>> {
    let path = match std::env::args().nth(1) {
        Some(argument) => PathBuf::from(argument),
        None => {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            std::fs::create_dir_all(&directory)?;
            directory.join("facade_guide_addressing_a_deck.pptx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
