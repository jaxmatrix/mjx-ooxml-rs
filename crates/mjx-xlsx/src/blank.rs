//! Authoring a workbook from nothing, and creating a part a workbook does not yet have.
//!
//! **Filled by MJXOFF-112 (D10).** Nothing here yet: MJXOFF-91 (D02) builds the package a model is
//! reached through and authors no part at all.
//!
//! What belongs here: `Workbook::blank` and `Workbook::blank_with_properties` — a workbook package
//! written from code rather than unpacked from a committed template, exactly as
//! `mjx_pptx::Presentation::blank` and `mjx_docx::Document::blank` are, so that the markup is markup
//! this project can explain and the same schema gate that validates an edited workbook validates
//! this one. MJXOFF-112 also removes `mjx_chart::EmbeddedWorkbook` and routes it through here, which
//! is the whole reason the `mjx-sml`/`mjx-xlsx` split exists.
//!
//! # The rule this module must be written to, before a line of it is written
//!
//! **When a part is authored on demand, write back a value that was READ from that part — never a
//! freshly constructed root written over a parsed one.**
//!
//! This is not hypothetical. In `mjx-docx`, `create_footnotes_part` parsed its template and then
//! wrote a fresh `Footnotes::blank()` over the root. A freshly built value has no ancestor to
//! inherit an `xmlns:w` declaration from, so the declaration was discarded, and every footnote
//! vanished the next time the document was opened. The gate was green throughout, because it
//! asserted on the model that had just been built rather than on the file that came back.
//!
//! The correct shape is:
//!
//! ```text
//! insert_part(part, content_type, minimal_bytes_carrying_the_namespace_declaration)
//! let root = package.part_tree_mut(&part)?;
//! let mut model = X::from_xml(root, interner)?;   // read what is there
//! model.mutate(...);                              // change it
//! model.write_back(root, interner);               // write back what was read
//! ```
//!
//! A freshly built *child* inserted into a value that *was* read from the root is fine; a freshly
//! built *root* is the bug. `mjx_docx::Document::create_style_sheet_part` is the shape to copy — it
//! writes a minimal `<x:styleSheet xmlns:x="…"/>` as **bytes** and then re-parses them through the
//! ordinary `part_tree_mut`/`FromXml` path, so the typed model only ever mutates a tree it read.
//!
//! **And assert on the reopened file, not on the model just built.** Where a namespace declaration
//! matters, include a raw-byte assertion on the reopened package, so that a "fix" which merely made
//! the reader more forgiving would not satisfy the test.
//!
//! A sweep of the whole workspace after that defect was found confirmed it exists nowhere else, so
//! `mjx-xlsx` starts clean. This note is here to keep it that way.
