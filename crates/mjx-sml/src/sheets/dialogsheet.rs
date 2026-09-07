//! `xl/dialogsheets/sheetN.xml` — `CT_Dialogsheet`, a legacy Excel 5.0 dialog box.
//!
//! `CT_Dialogsheet` (`sml.xsd:2150`) is a **16-slot `xsd:sequence`**, every member `minOccurs="0"`,
//! so `<dialogsheet/>` is valid markup. ECMA-376 Part 1 §12.3.7 names the part; Excel has not
//! offered to *create* one since version 5.0 and still opens files that carry them.
//!
//! # A dialogsheet has no cells either, and for a different reason
//!
//! Like [`ChartSheetPart`](crate::ChartSheetPart), the type has no `sheetData` and no `dimension`;
//! unlike it, it *does* have `sheetFormatPr` and a `sheetViews`. A dialog sheet is a canvas for form
//! controls — the `controls` slot at rank 14 — laid out over a coordinate space that is not a grid
//! of values. So [`DialogSheetPart`] has no cell accessor, for the reason
//! [`crate::sheets::chartsheet`] states in full: the absence is in the type.
//!
//! # Sixteen slots, ten modelled, six held — and none of the ten is modelled here
//!
//! Every one comes from somewhere else in this crate — [`SheetProperties`],
//! [`SheetViews`], [`SheetFormatProperties`] and
//! [`SheetProtection`] from MJXOFF-102/117's worksheet spine;
//! [`CustomSheetViews`] and the four-element print block from this child;
//! [`SheetDrawing`] from the chartsheet beside it. This file is a
//! content enum and a set of accessors, which is all a sheet kind whose markup is entirely shared
//! *should* be.
//!
//! The six held verbatim are `legacyDrawing` (rank 10), `legacyDrawingHF` (11), `drawingHF` (12),
//! `oleObjects` (13) and `controls` (14) — the drawing family and the two object slots — plus
//! `extLst` (15). `oleObjects` and `controls` are **MJXOFF-107's (E3)**, which is also the child
//! that will make a dialogsheet's controls readable; `legacyDrawing` is **MJXOFF-114's (E5)**. All
//! six round-trip byte-for-byte in position regardless, which is what
//! [`DialogSheetContent::Raw`] is for.
//!
//! Both figures are derived rather than stated: `sheets/frame.rs`'s
//! `every_slot_of_every_sheet_kind_is_accounted_for` reads a dialogsheet holding one of every slot
//! and holds the heading above to what the reader actually typed. Until MJXOFF-220 wrote it, this
//! header claimed that eleven of the sixteen were modelled where ten are, and called six of them
//! *five* in the sentence directly above the list of them.

use mjx_ooxml_core::{FromXml, Interner, RawElement, RawNode};
use mjx_ooxml_types::child_order::{ChildOrder, DIALOGSHEET};

use crate::error::SmlError;
use crate::features::custom_views::CustomSheetViews;
use crate::features::print::{HeaderFooter, PageMargins, PageSetup, PrintOptions};
use crate::sheets::chartsheet::SheetDrawing;
use crate::sheets::frame::{sheet_part_surface, sheet_slot, SheetContent, SheetFrame};
use crate::worksheet::{SheetFormatProperties, SheetProperties, SheetProtection, SheetViews};

/// One child of [`DialogSheetPart`]: ten modelled slots, and everything else.
#[derive(Debug)]
pub enum DialogSheetContent {
    /// `x:sheetPr` (rank 0) — the worksheet `CT_SheetPr`, not a chartsheet's.
    Properties(SheetProperties),
    /// `x:sheetViews` (rank 1).
    SheetViews(SheetViews),
    /// `x:sheetFormatPr` (rank 2).
    FormatProperties(SheetFormatProperties),
    /// `x:sheetProtection` (rank 3) — advisory locks and a preserved hash, never security.
    Protection(SheetProtection),
    /// `x:customSheetViews` (rank 4) — the *worksheet* `CT_CustomSheetViews`.
    CustomSheetViews(CustomSheetViews),
    /// `x:printOptions` (rank 5).
    PrintOptions(PrintOptions),
    /// `x:pageMargins` (rank 6).
    PageMargins(PageMargins),
    /// `x:pageSetup` (rank 7) — the full `CT_PageSetup`.
    PageSetup(PageSetup),
    /// `x:headerFooter` (rank 8).
    HeaderFooter(HeaderFooter),
    /// `x:drawing` (rank 9) — the relationship to a drawings part.
    Drawing(SheetDrawing),
    /// Everything this type does not model: `legacyDrawing` (10), `legacyDrawingHF` (11),
    /// `drawingHF` (12), `oleObjects` (13), `controls` (14), `extLst` (15), any foreign element, any
    /// `mc:AlternateContent`, and the text, comments and processing instructions between siblings.
    Raw(RawNode),
}

impl SheetContent for DialogSheetContent {
    const ORDER: &'static ChildOrder = DIALOGSHEET;

    fn read(element: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        Ok(Some(match interner.resolve(element.name.local) {
            "sheetPr" => Self::Properties(SheetProperties::from_xml(element, interner)?),
            "sheetViews" => Self::SheetViews(SheetViews::from_xml(element, interner)?),
            "sheetFormatPr" => {
                Self::FormatProperties(SheetFormatProperties::from_xml(element, interner)?)
            }
            "sheetProtection" => Self::Protection(SheetProtection::from_xml(element, interner)?),
            "customSheetViews" => {
                Self::CustomSheetViews(CustomSheetViews::from_xml(element, interner)?)
            }
            "printOptions" => Self::PrintOptions(PrintOptions::from_xml(element, interner)?),
            "pageMargins" => Self::PageMargins(PageMargins::from_xml(element, interner)?),
            "pageSetup" => Self::PageSetup(PageSetup::from_xml(element, interner)?),
            "headerFooter" => Self::HeaderFooter(HeaderFooter::from_xml(element, interner)?),
            "drawing" => Self::Drawing(SheetDrawing::from_xml(element, interner)?),
            _ => return Ok(None),
        }))
    }

    fn raw(node: RawNode) -> Self {
        Self::Raw(node)
    }

    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Properties(_) => "sheetPr",
            Self::SheetViews(_) => "sheetViews",
            Self::FormatProperties(_) => "sheetFormatPr",
            Self::Protection(_) => "sheetProtection",
            Self::CustomSheetViews(_) => "customSheetViews",
            Self::PrintOptions(_) => "printOptions",
            Self::PageMargins(_) => "pageMargins",
            Self::PageSetup(_) => "pageSetup",
            Self::HeaderFooter(_) => "headerFooter",
            Self::Drawing(_) => "drawing",
            Self::Raw(_) => return None,
        })
    }

    fn raw_node(&self) -> Option<&RawNode> {
        match self {
            Self::Raw(node) => Some(node),
            _ => None,
        }
    }

    fn as_raw_element(&self) -> Option<RawElement> {
        Some(match self {
            Self::Properties(value) => value.as_raw_element(),
            Self::SheetViews(value) => value.as_raw_element(),
            Self::FormatProperties(value) => value.as_raw_element(),
            Self::Protection(value) => value.as_raw_element(),
            Self::CustomSheetViews(value) => value.as_raw_element(),
            Self::PrintOptions(value) => value.as_raw_element(),
            Self::PageMargins(value) => value.as_raw_element(),
            Self::PageSetup(value) => value.as_raw_element(),
            Self::HeaderFooter(value) => value.as_raw_element(),
            Self::Drawing(value) => value.as_raw_element(),
            Self::Raw(_) => return None,
        })
    }
}

/// `x:dialogsheet` (`CT_Dialogsheet`, `sml.xsd:2150`) — the whole dialogsheet part.
///
/// See the [module documentation](crate::sheets::dialogsheet) for the sixteen slots and for why a
/// dialogsheet has no cell accessor.
#[derive(Debug)]
pub struct DialogSheetPart {
    frame: SheetFrame<DialogSheetContent>,
}

sheet_part_surface!(
    DialogSheetPart,
    DialogSheetContent,
    "dialogsheet",
    "dialogsheet"
);

impl DialogSheetPart {
    sheet_slot!(
        DialogSheetContent,
        properties,
        properties_mut,
        set_properties,
        Properties,
        SheetProperties,
        "sheetPr",
        "`x:sheetPr` (rank 0) — the *worksheet* `CT_SheetPr`, the same type MJXOFF-102 modelled."
    );
    sheet_slot!(
        DialogSheetContent,
        sheet_views,
        sheet_views_mut,
        set_sheet_views,
        SheetViews,
        SheetViews,
        "sheetViews",
        "`x:sheetViews` (rank 1) — the *worksheet* `CT_SheetViews`, panes and selections included."
    );
    sheet_slot!(
        DialogSheetContent,
        format_properties,
        format_properties_mut,
        set_format_properties,
        FormatProperties,
        SheetFormatProperties,
        "sheetFormatPr",
        "`x:sheetFormatPr` (rank 2) — default row height and column width."
    );
    sheet_slot!(
        DialogSheetContent,
        protection,
        protection_mut,
        set_protection,
        Protection,
        SheetProtection,
        "sheetProtection",
        "`x:sheetProtection` (rank 3) — advisory locks and a preserved hash. **Never security.**"
    );
    sheet_slot!(
        DialogSheetContent,
        custom_sheet_views,
        custom_sheet_views_mut,
        set_custom_sheet_views,
        CustomSheetViews,
        CustomSheetViews,
        "customSheetViews",
        "`x:customSheetViews` (rank 4) — the *worksheet* `CT_CustomSheetViews`, not a chartsheet's."
    );
    sheet_slot!(
        DialogSheetContent,
        print_options,
        print_options_mut,
        set_print_options,
        PrintOptions,
        PrintOptions,
        "printOptions",
        "`x:printOptions` (rank 5) — what a printed dialog sheet shows beside its controls."
    );
    sheet_slot!(
        DialogSheetContent,
        page_margins,
        page_margins_mut,
        set_page_margins,
        PageMargins,
        PageMargins,
        "pageMargins",
        "`x:pageMargins` (rank 6) — the six margins of a printed page, in inches."
    );
    sheet_slot!(
        DialogSheetContent,
        page_setup,
        page_setup_mut,
        set_page_setup,
        PageSetup,
        PageSetup,
        "pageSetup",
        "`x:pageSetup` (rank 7) — the **full** `CT_PageSetup`, unlike a chartsheet's."
    );
    sheet_slot!(
        DialogSheetContent,
        header_footer,
        header_footer_mut,
        set_header_footer,
        HeaderFooter,
        HeaderFooter,
        "headerFooter",
        "`x:headerFooter` (rank 8) — the six opaque code strings, never re-serialised."
    );
    sheet_slot!(
        DialogSheetContent,
        drawing,
        drawing_mut,
        set_drawing,
        Drawing,
        SheetDrawing,
        "drawing",
        "`x:drawing` (rank 9) — the relationship to a drawings part. Unlike a chartsheet's, this \
         slot is `minOccurs=\"0\"`."
    );
}
