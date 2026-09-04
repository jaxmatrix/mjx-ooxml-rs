// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors
// `mjx_pptx::guide`'s own shape — see that module's doc comment for why: no page declares an item,
// and each module imports the crate's public vocabulary so the guide's intra-doc links resolve.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            BlockPath, Document, DocxError, PageOrientation, PageSize, Paragraph, Run, RunPath,
        };
    };
}

guide_vocabulary!();

/// The whole story once, end to end.
pub mod building_a_document {
    #![doc = include_str!("../docs/guide/building_a_document.md")]
    guide_vocabulary!();
}
