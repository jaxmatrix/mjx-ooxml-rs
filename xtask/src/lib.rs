//! `xtask` as a library, so its own integration tests can reach the tables they are written against.
//!
//! # Why this target exists at all
//!
//! `xtask` is a host-only developer binary and everything in it used to be private to `main.rs`. The
//! validation harness (MJXOFF-122) changed that: `xtask/tests/validation_index.rs` compares the area
//! catalogue against `docs/validation/01-index.md`, and an integration test compiles into its own
//! crate — it cannot see a binary's modules. The alternative was to have the test parse the
//! binary's `--list` output, which would make a text format the contract instead of a type.
//!
//! Only [`validation`] lives here. `codegen`, `fuzz` and `corpus` stay private to the binary,
//! because nothing outside it needs them and moving them would move the fuzz campaign's
//! `#[global_allocator]` into every `xtask` test binary along with them.
//!
//! Nothing depends on this crate — `xtask/tests/layering.rs` asserts it — and it is excluded from
//! the cross-build matrix, so a library target here widens nothing.

pub mod validation;
