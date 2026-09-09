//! The guide's **The contract** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/the_round_trip_contract.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/the_round_trip_contract.mjs`. `cargo run -p xtask
//! -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # Why a workbook, and not the deck this block used to open
//!
//! Until MJXOFF-254 finished the guide, this block edited a chart on a deck and compared the two
//! packages through `mjx_opc::Package` — one layer *below* the facade the page is about, and the
//! layer [The curated surface](https://docs.rs/mjx-ooxml) seals deliberately. That made it a block
//! no binding could ever show, for no reason but its choice of surface.
//!
//! [`mjx_ooxml::Workbook`] is the one surface with a part door in all three languages —
//! `part_names` and `part_bytes` — so the project's central contract is stated here in all three
//! rather than exempted from two. The claim is the same claim: one edit, and every part it did not
//! touch comes back byte for byte.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_the_round_trip_contract -- out.xlsx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let original = mjx_fixtures::fixture("sample.xlsx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::Workbook;

    // One edit: the first tab's name, which `xl/workbook.xml` states and no other part does.
    let mut workbook = Workbook::open(&original)?;
    workbook.rename_sheet(0, "Revised")?;
    let saved = workbook.save()?;

    let before = Workbook::open(&original)?;
    let after = Workbook::open(&saved)?;
    assert_eq!(before.part_names(), after.part_names(), "no part appeared");

    let mut changed = Vec::new();
    for part in before.part_names() {
        if before.part_bytes(&part)? != after.part_bytes(&part)? {
            changed.push(part);
        }
    }
    // Everything else came back byte for byte — the whole of the contract.
    assert_eq!(changed, ["/xl/workbook.xml"]);
    // guide-example:end

    println!("one edit touched {changed:?}");
    write_output(&saved)
}

/// Where this example writes: its first argument, or `target/examples/` by default.
///
/// The two binding harnesses run this program with an explicit path and compare what lands there,
/// part by part, against what they produced themselves from the same fixture.
fn write_output(saved: &[u8]) -> Result<(), Box<dyn Error>> {
    let path = match std::env::args().nth(1) {
        Some(argument) => PathBuf::from(argument),
        None => {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            std::fs::create_dir_all(&directory)?;
            directory.join("facade_guide_the_round_trip_contract.xlsx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
