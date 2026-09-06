//! **The Excel boundary, measured (MJXOFF-137).** What a range read and a batched write cost
//! through the facade on a real 300,000-cell workbook, against the per-cell shape this facade
//! deliberately does not offer.
//!
//! ```sh
//! cargo run --release -p xtask -- corpus     # once; writes target/corpus/
//! cargo bench -p mjx-ooxml
//! ```
//!
//! # Why this exists at all
//!
//! MJXOFF-135 measured the figures [*Large workbooks*](mjx_xlsx::guide::large_workbooks) states —
//! 387 ms to read one cell of that sheet, a 16,427× ratio on a 4,000-cell write loop — and **left no
//! committed instrument that reproduces them**. `docs/BENCHMARKS.md`'s own harness measures
//! `Package::part_tree_mut`, which `mjx-xlsx` never calls, so it cannot see this cost at all: it is
//! a *different* code path measuring a *different* materialisation. MJXOFF-137 needed those figures
//! to be true before designing a public API around them, so it built the thing that checks.
//!
//! # What each case measures
//!
//! | case | what it is |
//! |---|---|
//! | `open` | `Workbook::open` — the package, the workbook part, the sheet list. No worksheet. |
//! | `read_range_one_cell` | the whole cost of reaching one cell: one worksheet parse. |
//! | `read_range_full_sheet` | the same parse, answering 300,000 cells instead of one. |
//! | `write_cells_batched` | one parse, N edits, one serialize, for N = 4,000. |
//! | `batched_hundred` / `per_cell_hundred` | the same **hundred** cells, batched and one at a time. |
//! | `save_untouched` | the ZIP floor: nothing was edited. |
//!
//! **`per_cell_hundred` is deliberately a hundred cells and not four thousand.** A per-cell write
//! into this sheet is a whole 300,000-cell parse each time, so four thousand of them is half an hour
//! per criterion iteration. That is the finding, not a measurement problem.
//!
//! # The methodology trap this file inherits
//!
//! `criterion::Bencher::iter_batched` only excludes a *returned* value's `Drop` from the timed
//! region, so every routine below returns the values it must not drop early as its tail expression.
//! `docs/BENCHMARKS.md` records the run where getting that wrong moved a figure by 99.98%.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use mjx_ooxml::{CellInput, CellWrite, Workbook};

/// The corpus file `cargo run --release -p xtask -- corpus` writes: 5,000 rows × 60 columns,
/// 300,000 populated cells, an 8.60 MiB worksheet part in a 1,235 KiB package.
const CORPUS: &str = "workbook_large.xlsx";

/// How many cells the batched write applies — a realistic batch, and a figure that shows the batch
/// size does **not** move the cost, because it is one parse either way.
const WRITE_CELLS: u32 = 4_000;

/// How many cells the like-for-like comparison below uses.
///
/// A hundred rather than four thousand, because a per-cell write into *this* sheet is a whole
/// 300,000-cell parse each time — around half a second — so four thousand of them is half an hour
/// per criterion iteration. A hundred is enough to establish the ratio and short enough to finish;
/// the shape of the result is the point and the shape does not change with N.
const COMPARED_CELLS: u32 = 100;

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/corpus")
        .join(CORPUS)
}

fn load_corpus() -> Vec<u8> {
    let path = corpus_path();
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "reading {}: {e}\n\nGenerate the corpus first:\n  \
             cargo run --release -p xtask -- corpus",
            path.display()
        )
    })
}

/// A batch of `count` writes into a region the corpus does not populate.
fn writes_of(count: u32) -> Vec<CellWrite> {
    (0..count)
        .map(|n| {
            CellWrite::new(
                format!("BA{}", n + 1),
                CellInput::Number(f64::from(n) + 0.5),
            )
        })
        .collect()
}

fn boundary(criterion: &mut Criterion) {
    let bytes = load_corpus();

    let mut group = criterion.benchmark_group("xlsx_facade");
    group.sample_size(10);

    group.bench_function("open", |bencher| {
        // `iter` drops the returned value outside the timed region, so the workbook's own teardown
        // is not folded into the open figure.
        bencher.iter(|| Workbook::open(black_box(&bytes)).expect("the corpus workbook"));
    });

    group.bench_function("read_range_one_cell", |bencher| {
        bencher.iter_batched(
            || Workbook::open(&bytes).expect("the corpus workbook"),
            |workbook| {
                let block = workbook.read_range(0, "A1").expect("one cell");
                (workbook, block)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("read_range_full_sheet", |bencher| {
        bencher.iter_batched(
            || Workbook::open(&bytes).expect("the corpus workbook"),
            |workbook| {
                let block = workbook.read_sheet(0).expect("the whole sheet");
                (workbook, block)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("write_cells_batched", |bencher| {
        let batch = writes_of(WRITE_CELLS);
        bencher.iter_batched(
            || Workbook::open(&bytes).expect("the corpus workbook"),
            |mut workbook| {
                workbook.write_cells(0, &batch).expect("the batched write");
                workbook
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("save_untouched", |bencher| {
        bencher.iter_batched(
            || Workbook::open(&bytes).expect("the corpus workbook"),
            |workbook| {
                let saved = workbook.save().expect("saving");
                (workbook, saved)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();

    // The like-for-like pair: the same hundred cells, batched and one at a time. The second is in
    // its own group with the smallest sample criterion allows, because every iteration of it is a
    // hundred whole-worksheet parses — and that *is* the result.
    let mut group = criterion.benchmark_group("xlsx_compared");
    group.sample_size(10);
    group.bench_function("batched_hundred", |bencher| {
        let batch = writes_of(COMPARED_CELLS);
        bencher.iter_batched(
            || Workbook::open(&bytes).expect("the corpus workbook"),
            |mut workbook| {
                workbook.write_cells(0, &batch).expect("the batched write");
                workbook
            },
            BatchSize::LargeInput,
        );
    });
    group.bench_function("per_cell_hundred", |bencher| {
        let batch = writes_of(COMPARED_CELLS);
        let references: Vec<mjx_ooxml::CellReference> = batch
            .iter()
            .map(|write| mjx_ooxml::CellReference::parse(&write.reference).expect("an address"))
            .collect();
        bencher.iter_batched(
            || Workbook::open(&bytes).expect("the corpus workbook"),
            |mut workbook| {
                for (write, reference) in batch.iter().zip(&references) {
                    let CellInput::Number(number) = write.value else {
                        panic!("the measured writes are numbers");
                    };
                    workbook
                        .workbook_mut()
                        .set_cell_value(0, *reference, mjx_sml::CellValue::Number(number))
                        .expect("one cell");
                }
                workbook
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(benches, boundary);
criterion_main!(benches);
