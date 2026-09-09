//! The guide's **`Workbook`: a tab index and A1 text** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/addressing.md`][guide] shows this code in three languages, and
//! every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/addressing_a_workbook.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/addressing_a_workbook.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # The three differ in shape, and the page says why
//!
//! A cell's value is an [`mjx_ooxml::CellInput`] here, passed to [`mjx_ooxml::CellWrite::new`]. An
//! enumeration carrying a payload has no projection in either binding, so both give
//! [`mjx_ooxml::CellWrite`] one **static constructor per kind** instead —
//! `CellWrite.number("B1", 12.5)` — which is the same information with the variant folded into the
//! function name. The set of kinds is the same on all three surfaces.
//!
//! This example saves, so both binding harnesses compare their package against this one part by
//! part: the same cells written through two spellings of the same vocabulary must produce
//! byte-identical parts.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_addressing_a_workbook -- out.xlsx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{CellInput, CellWrite, Workbook};

    let mut workbook = Workbook::blank()?;
    workbook.write_cells(
        0,
        &[
            CellWrite::new("A1", CellInput::SharedText("Region".into())),
            CellWrite::new("B1", CellInput::Number(12.5)),
            // The anchoring is data, not address: `$B$2` and `B2` spell one cell.
            CellWrite::new("$B$2", CellInput::Number(18.0)),
        ],
    )?;

    // A block is row-major over the whole requested rectangle, blanks included, and its two
    // arguments are offsets *into the block* rather than sheet coordinates.
    let block = workbook.read_range(0, "A1:B2")?;
    assert_eq!(block.first_row(), 0, "A1 is row 0, column 0");
    assert_eq!(block.first_column(), 0);
    assert_eq!(block.value(0, 0)?.text(), Some("Region"));
    assert_eq!(block.value(1, 1)?.number(), Some(18.0));
    assert_eq!(block.range().as_deref(), Some("A1:B2"));
    assert_eq!(workbook.used_range(0)?.as_deref(), Some("A1:B2"));

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
            directory.join("facade_guide_addressing_a_workbook.xlsx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
