//! `xtask` as a library, so its own integration tests can reach the tables they are written against.
//!
//! **This crate has no guide of its own**: it is a host-only developer binary, never published,
//! and nothing may depend on it. Every prose page in this repository is listed from
//! `docs/api/README.md`, and `CONTRIBUTING.md` is where a contributor starts.
//!
//! # Why this target exists at all
//!
//! `xtask` is a host-only developer binary and everything in it used to be private to `main.rs`. The
//! validation harness (MJXOFF-122) changed that: `xtask/tests/validation_index.rs` compares the area
//! catalogue against `docs/validation/01-index.md`, and an integration test compiles into its own
//! crate — it cannot see a binary's modules. The alternative was to have the test parse the
//! binary's `--list` output, which would make a text format the contract instead of a type.
//!
//! [`codegen`] joined it with MJXOFF-224, for the same reason and no other:
//! `xtask/tests/codegen_drift.rs` asks whether the committed `mjx-ooxml-types` source is what the
//! generator produces today, and it is written against [`codegen::artefacts`],
//! [`codegen::SIMPLE_TYPE_MODULES`] and [`codegen::UNCOVERED_SCHEMAS`] — the tables themselves,
//! not a text rendering of them. Nothing else re-derives that crate, so without a test there is no
//! moment at which a generator defect stops being invisible.
//!
//! `fuzz` and `corpus` stay private to the binary. `fuzz` must: moving it would move the campaign's
//! `#[global_allocator]` into every `xtask` test binary along with it.
//!
//! Nothing depends on this crate — `xtask/tests/layering.rs` asserts it — and it is excluded from
//! the cross-build matrix, so a library target here widens nothing.

pub mod codegen;
pub mod validation;
