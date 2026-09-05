//! Writing SpreadsheetML back out — the parts, and the whole package.
//!
//! **Filled by MJXOFF-112 (D10), and consumed by MJXOFF-99 (E1).** Nothing here yet — this child
//! (MJXOFF-132) creates the crate and the tree, and models nothing.
//!
//! What belongs here: the serializers that turn this crate's models into part bytes, and the
//! whole-package writer that replaces
//! [`mjx_chart::EmbeddedWorkbook`](https://docs.rs/mjx-chart) — the workspace's one deliberate
//! duplicate, which exists only because this crate did not. MJXOFF-112 writes it and `Workbook::blank`;
//! MJXOFF-99 then retires `mjx-chart`'s copy and routes `to_package_bytes` here, legal precisely
//! because `mjx-chart` (rank 2.2) may point at `mjx-sml` (rank 2.1) and never could at `mjx-xlsx`
//! (rank 3.0).
//!
//! Every child written from here goes through [`mjx_ooxml_types::child_order`], never by hand.
