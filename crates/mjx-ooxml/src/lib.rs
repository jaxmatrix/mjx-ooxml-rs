//! `mjx-ooxml` — the umbrella facade and documentation hub for the mjx-ooxml-rs workspace.
//!
//! `mjx-ooxml-rs` is a **pure-Rust** library for parsing, editing, generating, and (later) rendering
//! Office Open XML documents — PowerPoint (`.pptx`), Word (`.docx`), and Excel (`.xlsx`). The goal is
//! to open *any* OOXML file, load it fully into RAM, edit it at runtime, and write it back **without
//! corrupting the parts you did not touch** — with a codebase that cross-compiles cleanly to desktop,
//! Android, iOS, and WebAssembly.
//!
//! This crate will grow into the high-level `open()`/`save()` API (Phase 4). Today it is the umbrella
//! that ties the workspace together and the natural entry point for reading the docs.
//!
//! # The layered workspace
//!
//! Dependencies point strictly downward; each layer builds on the ones below it.
//!
//! - **Foundations** — [`mjx_ooxml_core`] (string [interner](mjx_ooxml_core::Interner) + the raw
//!   [preservation tree](mjx_ooxml_core::RawDocument)) and [`mjx_xml`] (the byte-preserving
//!   [`fidelity`](mjx_xml::fidelity) reader/writer — the only place `quick-xml` is used).
//! - **Packaging & compatibility** — [`mjx_opc`] (the OPC ZIP container and part graph, e.g.
//!   [`Package`](mjx_opc::Package)), [`mjx_mce`] (Markup Compatibility [`resolve`](mjx_mce::resolve) /
//!   preserve), and [`mjx_ooxml_types`] (generated, comprehensively-named simple types + namespaces).
//! - **Formats** *(in progress)* — `mjx_dml` (DrawingML), then `mjx_pptx`, `mjx_docx`, `mjx_xlsx`.
//!
//! # Fidelity model
//!
//! A package is a graph of parts kept as raw bytes; a part is materialized into a typed model only on
//! demand, and unmodified parts serialize back **verbatim**. Editing one slide cannot disturb the
//! theme, masters, or vendor parts, because they were never deserialized. See [`mjx_opc`] and
//! [`mjx_xml::fidelity`] for the mechanics.
//!
//! # Status
//!
//! Pre-release (`v0.0.x`). The packaging + fidelity + compatibility layers and the schema-type
//! generator are implemented and tested; the format models are being built PowerPoint-first. See the
//! repository `PLAN.md` and `CHANGELOG.md` for the roadmap and version milestones (`v0.1` = PowerPoint,
//! `v0.2` = Word, `v0.3` = Excel).

#[cfg(test)]
mod tests {
    #[test]
    fn crate_scaffold_builds() {
        // Placeholder so the crate is born green (TDD: always-green increments).
        assert_eq!(2 + 2, 4);
    }
}
