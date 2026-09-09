//! `xtask` — developer automation for mjx-ooxml-rs.
//!
//! Commands:
//! - `codegen` — regenerate `mjx-ooxml-types` from the local `References/` XSD schemas.
//! - `tokens` — regenerate the three design-token artefacts from
//!   `docs/client-platform/data/tokens.json` (MJXOFF-156); `tokens --check` refuses instead of
//!   writing, which is how the committed artefacts are held to the source.
//! - `ledger` — regenerate `docs/client-platform/PARITY_LEDGER.md` from the workspace's own test
//!   suites (MJXOFF-179); `ledger --check` refuses instead of writing, which is how the committed
//!   artefact is held to the tree. It runs no renderer and judges nothing itself: a row's state is
//!   derived from what the suites covering it actually contain, and anything nothing tests is
//!   `not-started`.
//! - `fuzz` — run the campaign against the untrusted-input entry points (MJXOFF-146).
//! - `corpus` — (re)build the large-file benchmarking corpus; `corpus --mem <format>` runs its
//!   peak-RSS checkpoints (MJXOFF-147).
//! - `validation-artefacts` — write the files the human Microsoft Office pass reads, for every
//!   validation area of all three formats (MJXOFF-122). It marks nothing; see
//!   `docs/validation/00-method.md`. `--ingest <file>` runs it the other way: hand it something
//!   saved out of Office and it reports which entry the file answers, whether it round-trips,
//!   whether the package holds, whether its child order matches ours, whether it validates, and
//!   where it would be committed (MJXOFF-130).
//!
//! This is a host-only dev tool; it is excluded from the shipped cross-compile matrix and never
//! part of the runtime dependency graph. It parses the schemas with our own `mjx-xml` (the schemas
//! are plain XML), applies the naming engine, and writes deterministic, committed Rust source.

// `xtask` is a binary: its module items have no external crate consumers, so `unreachable_pub`
// (a library-oriented lint) does not apply here.
#![allow(unreachable_pub)]

mod codegen;
mod corpus;
mod fuzz;
mod json;
mod ledger;

use anyhow::{bail, Result};

// The fourth command lives in this package's *library* target rather than in a module here, because
// `xtask/tests/validation_index.rs` is written against its area catalogue and an integration test
// cannot see a binary's modules. See `src/lib.rs`.
use xtask::validation;

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("codegen") => codegen::run(),
        Some("tokens") => codegen::tokens::run(&arguments[1..]),
        Some("ledger") => ledger::run(&arguments[1..]),
        Some("fuzz") => fuzz::run(&arguments[1..]),
        Some("corpus") => corpus::run(&arguments[1..]),
        Some("validation-artefacts") => validation::run(&arguments[1..]),
        Some(other) => bail!(
            "unknown command {other:?}. \
             Available: codegen, tokens, ledger, fuzz, corpus, validation-artefacts"
        ),
        None => {
            println!(
                "xtask — developer automation\n\nCommands:\n  \
                 codegen   regenerate mjx-ooxml-types from References/\n  \
                 tokens    regenerate the design-token artefacts (--check to verify, not write)\n  \
                 ledger    regenerate the parity ledger from the suites (--check to verify)\n  \
                 fuzz      campaign against the untrusted-input entry points (--list for targets)\n  \
                 corpus    (re)build the large-file benchmarking corpus (--mem <pptx|docx|xlsx>)\n  \
                 validation-artefacts\n            \
                 write the artefacts the human Office pass reads\n            \
                 [--format pptx|docx|xlsx] [--area <id or number>] [--out <dir>] [--list]\n            \
                 --ingest <file>  report on a file saved out of Office"
            );
            Ok(())
        }
    }
}
