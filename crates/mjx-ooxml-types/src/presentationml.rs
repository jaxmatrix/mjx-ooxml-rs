//! PresentationML identity types: what a placeholder holds, how a slide layout arranges content,
//! and what a slide's size is optimized for.
//!
//! These are **generated** from `pml.xsd` (regenerate with `cargo run -p xtask -- codegen`) and
//! re-exported here item by item, so the crate's public surface is curated rather than whatever the
//! generator happens to emit. Each item documents its original `ST_*` symbol and exact wire token.
//!
//! ```
//! use mjx_ooxml_types::presentationml::{PlaceholderType, SlideLayoutKind};
//!
//! assert_eq!(PlaceholderType::from_wire("ctrTitle"), Some(PlaceholderType::CenteredTitle));
//! assert_eq!(SlideLayoutKind::TitleAndObject.to_wire(), "obj");
//! ```

pub use crate::generated::presentationml::{
    Orientation, PlaceholderSize, PlaceholderType, SlideLayoutKind, SlideSizeKind,
};
