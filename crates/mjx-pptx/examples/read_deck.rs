//! Inspection only — open a deck and describe it, changing nothing.
//!
//! ```sh
//! cargo run -p mjx-pptx --example read_deck -- deck.pptx
//! ```
//!
//! Ends by proving the claim it is really making: **reading dirties nothing**. Every part of the
//! deck it saves is byte-identical to the deck it opened, despite having resolved formatting on
//! every run of every shape.

use std::collections::BTreeMap;

use anyhow::Result;
use mjx_opc::Package;
use mjx_pptx::Presentation;

mod support;

fn main() -> Result<()> {
    let bytes = match std::env::args().nth(1) {
        Some(path) => std::fs::read(path)?,
        None => support::template()?,
    };
    let mut deck = Presentation::open(&bytes)?;

    let size = deck.slide_size()?;
    println!(
        "{} slides, {} layouts, {} masters — {}×{} EMU",
        deck.slide_count(),
        deck.layout_count(),
        deck.master_count(),
        size.width_emu,
        size.height_emu
    );

    for slide in 0..deck.slide_count() {
        let layout = deck.slide_layout(slide)?;
        println!("\nslide {slide} (layout {layout:?})");

        for entry in deck.shapes(slide)? {
            let shape = entry.index;
            let role = match &entry.placeholder {
                Some(info) => format!("{:?} placeholder", info.kind),
                None => "not a placeholder".to_owned(),
            };
            println!("  shape {shape}: {:?}, {role}", entry.kind);

            // Where it renders — which for a placeholder that declares nothing lives on the layout.
            match deck.effective_shape_bounds(slide, shape)? {
                Some(bounds) => println!(
                    "    at ({}, {}) sized {}×{} EMU",
                    bounds.offset_x_emu, bounds.offset_y_emu, bounds.width_emu, bounds.height_emu
                ),
                None => println!("    no tier places this shape"),
            }

            // Text, and what its first run actually renders as.
            if let Ok(text) = deck.shape_text(slide, shape) {
                if !text.is_empty() {
                    println!("    text: {text:?}");
                    let effective = deck.effective_run_properties(slide, shape, 0, 0)?;
                    println!(
                        "    run 0 renders at {:?}pt, bold {:?}",
                        effective.size_points(),
                        effective.is_bold()
                    );
                }
            }
        }

        if let Some(notes) = deck.notes_text(slide)? {
            println!("  notes: {notes:?}");
        }
    }

    // The fidelity claim, checked.
    let before = byte_map(&Package::open(&bytes)?);
    let after = byte_map(&Package::open(&deck.save()?)?);
    for (name, original) in &before {
        anyhow::ensure!(
            after.get(name) == Some(original),
            "reading dirtied part {name}"
        );
    }
    println!("\nall {} parts byte-identical after reading", before.len());

    Ok(())
}

fn byte_map(package: &Package) -> BTreeMap<String, Vec<u8>> {
    package
        .entries()
        .iter()
        .filter_map(|entry| entry.bytes().map(|b| (entry.name.clone(), b.to_vec())))
        .collect()
}
