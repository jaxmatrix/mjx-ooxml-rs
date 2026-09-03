//! Baseline timings for the PresentationML corpus file (MJXOFF-147).
//!
//! Run with:
//!
//! ```sh
//! cargo run --release -p xtask -- corpus   # once, to generate target/corpus/deck_large.pptx
//! cargo bench -p mjx-pptx
//! ```
//!
//! Seven groups. The first six operate directly on `mjx_opc::Package`, exactly like the
//! `mjx-docx`/`mjx-xlsx` benchmarks, so the three formats are read on one scale; each states which
//! copy-on-write path it is on (the trap MJXOFF-147 names: a "save" benchmark that never edits
//! anything measures `memcpy`, not the real path):
//!
//! - `open` — `Package::open`, no part parsed.
//! - `first_mutation_materialisation` — one `part_tree_mut` on a freshly opened package: the first
//!   parse of the middle slide's part into a `RawDocument`.
//! - `edit_after_materialised` — one more attribute edit on a part that is *already* materialised,
//!   isolating the marginal cost of a mutation from the cost of parsing.
//! - `save_untouched` / `save_lightly_edited` / `save_fully_materialized` — the same three save
//!   paths as the other two formats.
//!
//! The seventh, `edit_via_presentation_api`, is `mjx-pptx`-only: it is the one format with a real
//! edit surface, and MJXOFF-147 asks for it to be exercised through that surface rather than only
//! through the lower `Package` layer — `Presentation::open` followed by one
//! `set_shape_text_content` call on the middle slide's title placeholder, the same call a caller of
//! this library would actually make.
//!
//! # Every routine below returns what it consumed — this is not idle tidiness
//!
//! `criterion::Bencher::iter_batched` only excludes a routine's **return value**'s `Drop` from the
//! timed region (its whole reason to exist over a hand-rolled loop: it batches return values into a
//! side buffer and drops that buffer *after* stopping the clock). A routine that takes `Package` (or
//! `Presentation`) by value and lets it fall out of scope unreturned — `black_box(package);` as a
//! trailing statement, say — drops it **inside** the timed call. `RawElement`'s `Drop` is
//! compiler-derived and recursive (the same property MJXOFF-146 traced a stack overflow to), so a
//! materialised tree's recursive deallocation is not free: an early draft of this file measured
//! `edit_after_materialised` in the milliseconds — it should cost microseconds, since the parse is
//! already paid for in `iter_batched`'s untimed setup — because the materialised tree's drop was
//! riding along inside the timed routine. Every routine here ends with the value(s) it must not
//! have dropped early as its tail expression.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use mjx_opc::{Package, PartName};
use mjx_pptx::Presentation;

mod support;

const CORPUS_FILE: &str = "deck_large.pptx";

fn criterion_benchmark(c: &mut Criterion) {
    let bytes = support::load_corpus(CORPUS_FILE);
    let target: PartName = support::representative_slide_part().expect("a valid part name");
    let slide_index = support::SLIDE_COUNT / 2;
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
    c.bench_function("pptx/open", |b| {
        b.iter(|| Package::open(std::hint::black_box(&bytes)).expect("open"));
    });

    c.bench_function("pptx/first_mutation_materialisation", |b| {
        b.iter_batched(
            || Package::open(&bytes).expect("open"),
            |mut package| {
                package.part_tree_mut(&target).expect("materialise");
                package
            },
            BatchSize::LargeInput,
        );
    });

    c.bench_function("pptx/edit_after_materialised", |b| {
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

    c.bench_function("pptx/save_untouched", |b| {
        b.iter_batched(
            || Package::open(&bytes).expect("open"),
            |package| {
                let saved = package.save().expect("save");
                (package, saved)
            },
            BatchSize::LargeInput,
        );
    });

    c.bench_function("pptx/save_lightly_edited", |b| {
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

    c.bench_function("pptx/save_fully_materialized", |b| {
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

    c.bench_function("pptx/edit_via_presentation_api", |b| {
        b.iter_batched(
            || Presentation::open(&bytes).expect("open"),
            |mut deck| {
                deck.set_shape_text_content(slide_index, 0, "Retitled by the benchmark")
                    .expect("set_shape_text_content");
                deck
            },
            BatchSize::LargeInput,
        );
    });
}

criterion_group! {
    name = benches;
    // A smaller sample keeps a full `cargo bench -p mjx-pptx` run in the tens of seconds, which is
    // what lets this baseline actually be re-run by a later child rather than only trusted from a
    // doc.
    config = Criterion::default().sample_size(20);
    targets = criterion_benchmark
}
criterion_main!(benches);
