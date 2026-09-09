//! The guide's **Row first, except where the file says otherwise** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/addressing.md`][guide] shows this code in three languages, and
//! every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/the_calls_that_take_the_column_first.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/the_calls_that_take_the_column_first.mjs`.
//! `cargo run -p xtask -- guide-examples` does the copying and `xtask/tests/guide_examples.rs`
//! proves it was done.
//!
//! Four methods on this facade take the column before the row, and this is one of them. The reason
//! is the markup: an `xdr` anchor marker is `<xdr:col><xdr:colOff><xdr:row><xdr:rowOff>`, so its two
//! offsets interleave with its two indices and taking the row first would put each offset beside the
//! wrong one. Nothing catches a caller who transposes them, because both arguments are numbers —
//! which is exactly why the same four numbers appear here in all three languages.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_the_calls_that_take_the_column_first -- out.xlsx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{ChartData, ChartKind, ResizingBehavior, Workbook};

    let mut workbook = Workbook::blank()?;
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("North", [12.5, 18.0]);

    // From column 1, row 1 (B2) to column 7, row 16 (H17) — column first, both times.
    let resizing = ResizingBehavior::MoveAndResizeWithAnchorCells;
    let anchor = workbook.add_chart(0, &chart, 1, 1, 7, 16, "Revenue", resizing)?;
    assert_eq!(workbook.chart_anchor_indices(0)?, vec![anchor]);

    let saved = workbook.save()?;
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
            directory.join("facade_guide_the_calls_that_take_the_column_first.xlsx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
