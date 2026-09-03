//! Baseline timings for the SpreadsheetML corpus file (MJXOFF-147) — the workbook MJXOFF-95 reads
//! this baseline for.
//!
//! `mjx-xlsx` has no model yet, so every operation here is at the layer that exists —
//! `mjx_opc::Package` — exactly what MJXOFF-132 will be measured against. Run with:
//!
//! ```sh
//! cargo run --release -p xtask -- corpus   # once, to generate target/corpus/workbook_large.xlsx
//! cargo bench -p mjx-xlsx
//! ```
//!
//! Six groups, each stating which copy-on-write path it is on (the trap MJXOFF-147 names: a "save"
//! benchmark that never edits anything measures `memcpy`, not the real path):
//!
//! - `open` — `Package::open`, no part parsed.
//! - `first_mutation_materialisation` — one `part_tree_mut` on a freshly opened package: the first
//!   parse of `xl/worksheets/sheet1.xml` (300,000 cells, ~610,000 elements) into a `RawDocument`.
//!   `docs/BENCHMARKS.md` also records this scenario's **peak resident set** — the figure MJXOFF-95
//!   halts without — measured separately with `cargo run --release -p xtask -- corpus --mem xlsx`,
//!   since criterion does not report memory.
//! - `edit_after_materialised` — one more attribute edit on a part that is *already* materialised
//!   (parse cost already paid), isolating the marginal cost of a mutation from the cost of parsing.
//! - `save_untouched` — the part was never read as a tree; `save` re-emits its stored bytes verbatim.
//! - `save_lightly_edited` — one attribute changed; every untouched sibling still copies from the
//!   source buffer (MJX-248's span-preserving path).
//! - `save_fully_materialized` — the source buffer is released before saving (the pre-MJX-248
//!   behaviour): every element serializes from the model.
//!
//! # Every routine below returns what it consumed — this is not idle tidiness
//!
//! `criterion::Bencher::iter_batched` only excludes a routine's **return value**'s `Drop` from the
//! timed region (its whole reason to exist over a hand-rolled loop: it batches return values into a
//! side buffer and drops that buffer *after* stopping the clock). A routine that takes `Package` by
//! value and lets it fall out of scope unreturned — `black_box(package);` as a trailing statement,
//! say — drops it **inside** the timed call. `RawElement`'s `Drop` is compiler-derived and
//! recursive (the same property MJXOFF-146 traced a stack overflow to), so for a ~610,000-element
//! tree that recursive deallocation is not free: an early draft of this file measured
//! `edit_after_materialised` in the milliseconds — it should cost microseconds, since the parse is
//! already paid for in `iter_batched`'s untimed setup — because the materialised tree's drop was
//! riding along inside the timed routine. Every routine here ends with the value(s) it must not
//! have dropped early as its tail expression.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use mjx_opc::{Package, PartName};

mod support;

const CORPUS_FILE: &str = "workbook_large.xlsx";
const WORKSHEET_PART: &str = "/xl/worksheets/sheet1.xml";

fn worksheet_part() -> PartName {
    PartName::new(WORKSHEET_PART).expect("a valid part name")
}

fn criterion_benchmark(c: &mut Criterion) {
    let bytes = support::load_corpus(CORPUS_FILE);
    let target = worksheet_part();
    // Computed once, read-only, outside every timed closure below — see `benches/support/mod.rs`
    // for why folding this search into a timed routine would measure something else entirely.
    let edit_path = {
        let mut package = Package::open(&bytes).expect("open");
        let tree = package.part_tree_mut(&target).expect("materialise");
        support::representative_path(tree)
    };

    // `open` never materialises a tree (parts stay `PartBody::Raw`), so dropping the opened
    // `Package` at the end of a plain `b.iter` closure is cheap and does not need `iter_batched`'s
    // deferred-drop treatment.
    c.bench_function("xlsx/open", |b| {
        b.iter(|| Package::open(std::hint::black_box(&bytes)).expect("open"));
    });

    c.bench_function("xlsx/first_mutation_materialisation", |b| {
        b.iter_batched(
            || Package::open(&bytes).expect("open"),
            |mut package| {
                package.part_tree_mut(&target).expect("materialise");
                package
            },
            BatchSize::LargeInput,
        );
    });

    c.bench_function("xlsx/edit_after_materialised", |b| {
        b.iter_batched(
            || {
                let mut package = Package::open(&bytes).expect("open");
                package.part_tree_mut(&target).expect("materialise");
                package
            },
            |mut package| {
                let tree = package
                    .part_tree_mut(&target)
                    .expect("already materialised");
                support::mutate_at(tree, &edit_path);
                package
            },
            BatchSize::LargeInput,
        );
    });

    c.bench_function("xlsx/save_untouched", |b| {
        b.iter_batched(
            || Package::open(&bytes).expect("open"),
            |package| {
                let saved = package.save().expect("save");
                (package, saved)
            },
            BatchSize::LargeInput,
        );
    });

    c.bench_function("xlsx/save_lightly_edited", |b| {
        b.iter_batched(
            || {
                let mut package = Package::open(&bytes).expect("open");
                let tree = package.part_tree_mut(&target).expect("materialise");
                support::mutate_at(tree, &edit_path);
                package
            },
            |package| {
                let saved = package.save().expect("save");
                (package, saved)
            },
            BatchSize::LargeInput,
        );
    });

    c.bench_function("xlsx/save_fully_materialized", |b| {
        b.iter_batched(
            || {
                let mut package = Package::open(&bytes).expect("open");
                let tree = package.part_tree_mut(&target).expect("materialise");
                support::mutate_at(tree, &edit_path);
                tree.release_source();
                package
            },
            |package| {
                let saved = package.save().expect("save");
                (package, saved)
            },
            BatchSize::LargeInput,
        );
    });
}

criterion_group! {
    name = benches;
    // ~610,000 elements costs real milliseconds per iteration; a smaller sample keeps a full
    // `cargo bench -p mjx-xlsx` run in the tens of seconds rather than minutes, which is what lets
    // this baseline actually be re-run by a later child rather than only trusted from a doc.
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark
}
criterion_main!(benches);
