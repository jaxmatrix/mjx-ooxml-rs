//! `xtask` — developer automation for mjx-ooxml-rs.
//!
//! Commands:
//! - `codegen` — regenerate `mjx-ooxml-types` from the local `References/` XSD schemas.
//!
//! This is a host-only dev tool; it is excluded from the shipped cross-compile matrix and never
//! part of the runtime dependency graph. It parses the schemas with our own `mjx-xml` (the schemas
//! are plain XML), applies the naming engine, and writes deterministic, committed Rust source.

// `xtask` is a binary: its module items have no external crate consumers, so `unreachable_pub`
// (a library-oriented lint) does not apply here.
#![allow(unreachable_pub)]

mod codegen;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("codegen") => codegen::run(),
        Some(other) => bail!("unknown command {other:?}. Available: codegen"),
        None => {
            println!("xtask — developer automation\n\nCommands:\n  codegen   regenerate mjx-ooxml-types from References/");
            Ok(())
        }
    }
}
