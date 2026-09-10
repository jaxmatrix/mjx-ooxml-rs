//! `xtask` — developer automation for mjx-ooxml-rs.
//!
//! **This crate has no guide of its own**: it is a host-only developer binary, never
//! published, and nothing may depend on it. Every prose page in this repository is listed
//! from `docs/api/README.md`, and `CONTRIBUTING.md` is where a contributor starts.
//!
//! Commands:
//! - `codegen` — regenerate `mjx-ooxml-types` from the local `References/` XSD schemas.
//!   `codegen --check` writes nothing and reports whether the committed output is what the
//!   generator produces today (MJXOFF-224).
//! - `guide-examples` — copy each guide example's sentinel-delimited region out of the three files
//!   a test runner executes and into the code blocks the guide commits (MJXOFF-254).
//!   `guide-examples --check` writes nothing and reports whether the committed blocks are current.
//! - `docs-site` — write the Docusaurus content tree the user guide's site renders from
//!   (MJXOFF-281). The output is git-ignored; `xtask/tests/docs_site.rs` is the gate.
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

mod corpus;
mod fuzz;

use anyhow::{bail, Result};

// Two commands live in this package's *library* target rather than in modules here, because an
// integration test cannot see a binary's modules and both have suites written against their tables:
// `xtask/tests/validation_index.rs` against the area catalogue, and `xtask/tests/codegen_drift.rs`
// against the generator's own artefacts. See `src/lib.rs`.
use xtask::{codegen, docs_site, guide_examples, validation};

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("codegen") => match arguments.get(1).map(String::as_str) {
            None => codegen::run(),
            Some("--check") => codegen::check(),
            Some(other) => bail!("unknown codegen argument {other:?}. Available: --check"),
        },
        Some("guide-examples") => match arguments.get(1).map(String::as_str) {
            None => guide_examples::run(),
            Some("--check") => guide_examples::check(),
            Some(other) => bail!("unknown guide-examples argument {other:?}. Available: --check"),
        },
        Some("docs-site") => match arguments.get(1).map(String::as_str) {
            None => docs_site::run(),
            Some(other) => bail!("unknown docs-site argument {other:?}. It takes none."),
        },
        Some("fuzz") => fuzz::run(&arguments[1..]),
        Some("corpus") => corpus::run(&arguments[1..]),
        Some("validation-artefacts") => validation::run(&arguments[1..]),
        Some(other) => bail!(
            "unknown command {other:?}. Available: codegen, guide-examples, docs-site, fuzz, \
             corpus, validation-artefacts"
        ),
        None => {
            println!(
                "xtask — developer automation\n\nCommands:\n  \
                 codegen   regenerate mjx-ooxml-types from References/\n            \
                 --check  write nothing; report whether the committed output is current\n  \
                 guide-examples\n            \
                 copy each guide example's region into the blocks the guide commits\n            \
                 --check  write nothing; report whether the committed blocks are current\n  \
                 docs-site\n            \
                 write the content tree the user guide's site renders from\n  \
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
