// A documentation-only module: the guide lives in `docs/effective_properties.md` so it reads as
// prose on a source host as well as on the rendered docs page. It declares no items — this is what
// makes its snippets *compiled*: without this module (see the pinned orchestrator comment on
// MJXOFF-106), the Markdown file would be prose no compiler ever reads. Mirrors
// `mjx_pptx::effective_properties`'s own shape exactly.
#![doc = include_str!("../docs/effective_properties.md")]

#[allow(unused_imports)] // Referenced only by the guide's intra-doc links.
use crate::{
    Document, EffectiveCharacterProperties, EffectiveColor, EffectiveParagraphProperties,
    ParagraphProperties, RunProperties, StyleIndex, StyleSheet,
};
