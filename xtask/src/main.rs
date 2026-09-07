//! `xtask` — developer automation for mjx-ooxml-rs.
//!
//! Commands:
//! - `codegen` — regenerate `mjx-ooxml-types` from the local `References/` XSD schemas.
//! - `fuzz` — run the campaign against the untrusted-input entry points (MJXOFF-146).
//! - `corpus` — (re)build the large-file benchmarking corpus; `corpus --mem <format>` runs its
//!   peak-RSS checkpoints (MJXOFF-147).
//! - `validation-artefacts` — write the files the human Microsoft Office pass reads, for every
//!   validation area of all three formats (MJXOFF-122). It marks nothing; see
//!   `docs/validation/00-method.md`.
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

use anyhow::{bail, Result};

// The fourth command lives in this package's *library* target rather than in a module here, because
// `xtask/tests/validation_index.rs` is written against its area catalogue and an integration test
// cannot see a binary's modules. See `src/lib.rs`.
use xtask::validation;

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("codegen") => codegen::run(),
        Some("fuzz") => fuzz::run(&arguments[1..]),
        Some("corpus") => corpus::run(&arguments[1..]),
        Some("validation-artefacts") => validation::run(&arguments[1..]),
        Some(other) => bail!(
            "unknown command {other:?}. Available: codegen, fuzz, corpus, validation-artefacts"
        ),
        None => {
            println!(
                "xtask — developer automation\n\nCommands:\n  \
                 codegen   regenerate mjx-ooxml-types from References/\n  \
                 fuzz      campaign against the untrusted-input entry points (--list for targets)\n  \
                 corpus    (re)build the large-file benchmarking corpus (--mem <pptx|docx|xlsx>)\n  \
                 validation-artefacts\n            \
                 write the artefacts the human Office pass reads\n            \
                 [--format pptx|docx|xlsx] [--area <id or number>] [--out <dir>] [--list]"
            );
            Ok(())
        }
    }
}
