//! `xtask` — developer automation for mjx-ooxml-rs.
//!
//! Commands:
//! - `codegen` — regenerate `mjx-ooxml-types` from the local `References/` XSD schemas.
//! - `fuzz` — run the campaign against the untrusted-input entry points (MJXOFF-146).
//!
//! This is a host-only dev tool; it is excluded from the shipped cross-compile matrix and never
//! part of the runtime dependency graph. It parses the schemas with our own `mjx-xml` (the schemas
//! are plain XML), applies the naming engine, and writes deterministic, committed Rust source.

// `xtask` is a binary: its module items have no external crate consumers, so `unreachable_pub`
// (a library-oriented lint) does not apply here.
#![allow(unreachable_pub)]

mod codegen;
mod fuzz;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("codegen") => codegen::run(),
        Some("fuzz") => fuzz::run(&arguments[1..]),
        Some(other) => bail!("unknown command {other:?}. Available: codegen, fuzz"),
        None => {
            println!(
                "xtask — developer automation\n\nCommands:\n  \
                 codegen   regenerate mjx-ooxml-types from References/\n  \
                 fuzz      campaign against the untrusted-input entry points (--list for targets)"
            );
            Ok(())
        }
    }
}
