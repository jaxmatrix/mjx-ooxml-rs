//! `CT_Macrosheet` — an XLM macro sheet, the fourth sheet kind and the one ECMA-376 declares no
//! global element for.
//!
//! # The schema declares the type and no element, and that is not an oversight this crate fixes
//!
//! `sml.xsd` declares three global elements — `worksheet` (`:2115`), `chartsheet` (`:2116`) and
//! `dialogsheet` (`:2117`) — and **none for `macrosheet`**. `CT_Macrosheet` (`:2118`) is a complex
//! type nothing in ECMA-376 Part 1 makes into a part: there is no content type for one in §12.3 and
//! no relationship type for one in §15.2. The strings real files use —
//! `application/vnd.ms-excel.macrosheet+xml` and
//! `http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet` — are Microsoft's
//! extensions, and `crates/mjx-xlsx/src/parts.rs` has a written rule against declaring
//! `vnd.ms-excel` names this project cannot quote a clause for.
//!
//! So a macrosheet is reached the way `mjx-xlsx` reaches a `.xlsm`'s workbook part: **by its root
//! element**, never by its content type. `mjx_xlsx::Workbook::sheet_markup` parses the part and
//! dispatches on the root local name, so a macrosheet reports as one while
//! `mjx_xlsx::SheetKind` — which is defined by content type and by ECMA-376's three — stays at
//! three. `crates/mjx-xlsx/tests/worksheet_part.rs` pins both halves.
//!
//! # `sheetData` is preserved, not stored
//!
//! `CT_Macrosheet` declares `sheetData` `minOccurs="1"`, and its cells hold the XLM macro language
//! rather than the formula language [`CellFormula`](crate::CellFormula) models. This type holds that
//! slot **verbatim**, as [`MacroSheetContent::Raw`], and offers no cell accessor:
//!
//! * MJXOFF-95's packed [`SheetData`](crate::SheetData) is `CT_Worksheet`'s store, built around the
//!   part's own shared buffer and three levels of copy-on-write below the slot. Reaching it from a
//!   second frame is a design question, not a line of plumbing;
//! * and nothing in this workspace interprets XLM, so a modelled macrosheet cell would be a
//!   `CellValue` nobody could act on.
//!
//! The ticket asks for a macrosheet *"modelled at least to the frame level, so a workbook containing
//! one round-trips and reports it"*, and that is exactly what this is: twenty of the twenty-seven
//! slots typed, seven held, every one of the twenty-seven byte-identical in position.

use mjx_ooxml_core::{FromXml, Interner, RawElement, RawNode};
use mjx_ooxml_types::child_order::{ChildOrder, MACROSHEET};

use crate::error::SmlError;
use crate::features::custom_views::CustomSheetViews;
use crate::features::print::{HeaderFooter, PageMargins, PageSetup, PrintOptions};
use crate::features::{
    AutoFilter, ConditionalFormatting, CustomProperties, DataConsolidation, SortState,
};
use crate::sheets::chartsheet::SheetDrawing;
use crate::sheets::frame::{sheet_part_surface, sheet_slot, SheetContent, SheetFrame};
use crate::worksheet::{
    ColumnBlock, PageBreaks, SheetDimension, SheetFormatProperties, SheetProperties,
    SheetProtection, SheetViews,
};

use crate::features::print::SheetBackgroundPicture;

/// One child of [`MacroSheetPart`]: twenty modelled slots, and everything else.
#[derive(Debug)]
pub enum MacroSheetContent {
    /// `x:sheetPr` (rank 0).
    Properties(SheetProperties),
    /// `x:dimension` (rank 1) — a cached bounding box.
    Dimension(SheetDimension),
    /// `x:sheetViews` (rank 2).
    SheetViews(SheetViews),
    /// `x:sheetFormatPr` (rank 3).
    FormatProperties(SheetFormatProperties),
    /// `x:cols` (rank 4) — one block. The slot is `maxOccurs="unbounded"`, so several may stand in a
    /// row and merging them would change the file.
    Columns(ColumnBlock),
    /// `x:sheetProtection` (rank 6).
    Protection(SheetProtection),
    /// `x:autoFilter` (rank 7) — recorded, never applied.
    AutoFilter(AutoFilter),
    /// `x:sortState` (rank 8) — a record of a sort, never a sort.
    SortState(SortState),
    /// `x:dataConsolidate` (rank 9).
    DataConsolidation(DataConsolidation),
    /// `x:customSheetViews` (rank 10).
    CustomSheetViews(CustomSheetViews),
    /// `x:conditionalFormatting` (rank 12) — one block; the slot is `maxOccurs="unbounded"`.
    ConditionalFormatting(ConditionalFormatting),
    /// `x:printOptions` (rank 13).
    PrintOptions(PrintOptions),
    /// `x:pageMargins` (rank 14).
    PageMargins(PageMargins),
    /// `x:pageSetup` (rank 15) — the full `CT_PageSetup`.
    PageSetup(PageSetup),
    /// `x:headerFooter` (rank 16).
    HeaderFooter(HeaderFooter),
    /// `x:rowBreaks` (rank 17).
    RowBreaks(PageBreaks),
    /// `x:colBreaks` (rank 18) — the same complex type, the other axis.
    ColumnBreaks(PageBreaks),
    /// `x:customProperties` (rank 19).
    CustomProperties(CustomProperties),
    /// `x:drawing` (rank 20).
    Drawing(SheetDrawing),
    /// `x:picture` (rank 24) — the image drawn behind the sheet.
    BackgroundPicture(SheetBackgroundPicture),
    /// Everything this type does not model: **`sheetData` (rank 5)** above all — see the
    /// [module documentation](crate::sheets::macrosheet) — plus `phoneticPr` (11), `legacyDrawing`
    /// (21), `legacyDrawingHF` (22), `drawingHF` (23), `oleObjects` (25) and `extLst` (26), any
    /// foreign element, any `mc:AlternateContent`, and the text between siblings.
    Raw(RawNode),
}

impl SheetContent for MacroSheetContent {
    const ORDER: &'static ChildOrder = MACROSHEET;

    fn read(element: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        Ok(Some(match interner.resolve(element.name.local) {
            "sheetPr" => Self::Properties(SheetProperties::from_xml(element, interner)?),
            "dimension" => Self::Dimension(SheetDimension::from_xml(element, interner)?),
            "sheetViews" => Self::SheetViews(SheetViews::from_xml(element, interner)?),
            "sheetFormatPr" => {
                Self::FormatProperties(SheetFormatProperties::from_xml(element, interner)?)
            }
            "cols" => Self::Columns(ColumnBlock::from_xml(element, interner)?),
            "sheetProtection" => Self::Protection(SheetProtection::from_xml(element, interner)?),
            "autoFilter" => Self::AutoFilter(AutoFilter::from_xml(element, interner)?),
            "sortState" => Self::SortState(SortState::from_xml(element, interner)?),
            "dataConsolidate" => {
                Self::DataConsolidation(DataConsolidation::from_xml(element, interner)?)
            }
            "customSheetViews" => {
                Self::CustomSheetViews(CustomSheetViews::from_xml(element, interner)?)
            }
            "conditionalFormatting" => {
                Self::ConditionalFormatting(ConditionalFormatting::from_xml(element, interner)?)
            }
            "printOptions" => Self::PrintOptions(PrintOptions::from_xml(element, interner)?),
            "pageMargins" => Self::PageMargins(PageMargins::from_xml(element, interner)?),
            "pageSetup" => Self::PageSetup(PageSetup::from_xml(element, interner)?),
            "headerFooter" => Self::HeaderFooter(HeaderFooter::from_xml(element, interner)?),
            "rowBreaks" => Self::RowBreaks(PageBreaks::from_xml(element, interner)?),
            "colBreaks" => Self::ColumnBreaks(PageBreaks::from_xml(element, interner)?),
            "customProperties" => {
                Self::CustomProperties(CustomProperties::from_xml(element, interner)?)
            }
            "drawing" => Self::Drawing(SheetDrawing::from_xml(element, interner)?),
            "picture" => {
                Self::BackgroundPicture(SheetBackgroundPicture::from_xml(element, interner)?)
            }
            _ => return Ok(None),
        }))
    }

    fn raw(node: RawNode) -> Self {
        Self::Raw(node)
    }

    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Properties(_) => "sheetPr",
            Self::Dimension(_) => "dimension",
            Self::SheetViews(_) => "sheetViews",
            Self::FormatProperties(_) => "sheetFormatPr",
            Self::Columns(_) => "cols",
            Self::Protection(_) => "sheetProtection",
            Self::AutoFilter(_) => "autoFilter",
            Self::SortState(_) => "sortState",
            Self::DataConsolidation(_) => "dataConsolidate",
            Self::CustomSheetViews(_) => "customSheetViews",
            Self::ConditionalFormatting(_) => "conditionalFormatting",
            Self::PrintOptions(_) => "printOptions",
            Self::PageMargins(_) => "pageMargins",
            Self::PageSetup(_) => "pageSetup",
            Self::HeaderFooter(_) => "headerFooter",
            Self::RowBreaks(_) => "rowBreaks",
            Self::ColumnBreaks(_) => "colBreaks",
            Self::CustomProperties(_) => "customProperties",
            Self::Drawing(_) => "drawing",
            Self::BackgroundPicture(_) => "picture",
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
            Self::Dimension(value) => value.as_raw_element(),
            Self::SheetViews(value) => value.as_raw_element(),
            Self::FormatProperties(value) => value.as_raw_element(),
            Self::Columns(value) => value.as_raw_element(),
            Self::Protection(value) => value.as_raw_element(),
            Self::AutoFilter(value) => value.as_raw_element(),
            Self::SortState(value) => value.as_raw_element(),
            Self::DataConsolidation(value) => value.as_raw_element(),
            Self::CustomSheetViews(value) => value.as_raw_element(),
            Self::ConditionalFormatting(value) => value.as_raw_element(),
            Self::PrintOptions(value) => value.as_raw_element(),
            Self::PageMargins(value) => value.as_raw_element(),
            Self::PageSetup(value) => value.as_raw_element(),
            Self::HeaderFooter(value) => value.as_raw_element(),
            Self::RowBreaks(value) | Self::ColumnBreaks(value) => value.as_raw_element(),
            Self::CustomProperties(value) => value.as_raw_element(),
            Self::Drawing(value) => value.as_raw_element(),
            Self::BackgroundPicture(value) => value.as_raw_element(),
            Self::Raw(_) => return None,
        })
    }
}

/// `CT_Macrosheet` (`sml.xsd:2118`) — the whole macrosheet part.
///
/// The root element's name is whatever the file wrote, because ECMA-376 declares no global element
/// for this type at all; every producer writes `macrosheet`, which is the name
/// [`read_document`](Self::read_document) matches. See the
/// [module documentation](crate::sheets::macrosheet) for what that means for content types and
/// relationship types, and for why `sheetData` is held rather than stored.
#[derive(Debug)]
pub struct MacroSheetPart {
    frame: SheetFrame<MacroSheetContent>,
}

sheet_part_surface!(
    MacroSheetPart,
    MacroSheetContent,
    "macrosheet",
    "macrosheet"
);

impl MacroSheetPart {
    sheet_slot!(
        MacroSheetContent,
        properties,
        properties_mut,
        set_properties,
        Properties,
        SheetProperties,
        "sheetPr",
        "`x:sheetPr` (rank 0) — the tab's colour, outline behaviour and page-setup flags."
    );
    sheet_slot!(
        MacroSheetContent,
        dimension,
        dimension_mut,
        set_dimension,
        Dimension,
        SheetDimension,
        "dimension",
        "`x:dimension` (rank 1) — a cached bounding box, reported and never recomputed: this type \
         does not read the `sheetData` it would have to recompute it from."
    );
    sheet_slot!(
        MacroSheetContent,
        sheet_views,
        sheet_views_mut,
        set_sheet_views,
        SheetViews,
        SheetViews,
        "sheetViews",
        "`x:sheetViews` (rank 2)."
    );
    sheet_slot!(
        MacroSheetContent,
        format_properties,
        format_properties_mut,
        set_format_properties,
        FormatProperties,
        SheetFormatProperties,
        "sheetFormatPr",
        "`x:sheetFormatPr` (rank 3) — default row height and column width."
    );
    sheet_slot!(
        MacroSheetContent,
        protection,
        protection_mut,
        set_protection,
        Protection,
        SheetProtection,
        "sheetProtection",
        "`x:sheetProtection` (rank 6) — advisory locks and a preserved hash. **Never security.**"
    );
    sheet_slot!(
        MacroSheetContent,
        auto_filter,
        auto_filter_mut,
        set_auto_filter,
        AutoFilter,
        AutoFilter,
        "autoFilter",
        "`x:autoFilter` (rank 7) — recorded, never applied."
    );
    sheet_slot!(
        MacroSheetContent,
        sort_state,
        sort_state_mut,
        set_sort_state,
        SortState,
        SortState,
        "sortState",
        "`x:sortState` (rank 8) — a record of a sort, never a sort."
    );
    sheet_slot!(
        MacroSheetContent,
        data_consolidation,
        data_consolidation_mut,
        set_data_consolidation,
        DataConsolidation,
        DataConsolidation,
        "dataConsolidate",
        "`x:dataConsolidate` (rank 9) — recorded, never performed."
    );
    sheet_slot!(
        MacroSheetContent,
        custom_sheet_views,
        custom_sheet_views_mut,
        set_custom_sheet_views,
        CustomSheetViews,
        CustomSheetViews,
        "customSheetViews",
        "`x:customSheetViews` (rank 10) — the *worksheet* `CT_CustomSheetViews`."
    );
    sheet_slot!(
        MacroSheetContent,
        print_options,
        print_options_mut,
        set_print_options,
        PrintOptions,
        PrintOptions,
        "printOptions",
        "`x:printOptions` (rank 13)."
    );
    sheet_slot!(
        MacroSheetContent,
        page_margins,
        page_margins_mut,
        set_page_margins,
        PageMargins,
        PageMargins,
        "pageMargins",
        "`x:pageMargins` (rank 14) — the six margins of a printed page, in inches."
    );
    sheet_slot!(
        MacroSheetContent,
        page_setup,
        page_setup_mut,
        set_page_setup,
        PageSetup,
        PageSetup,
        "pageSetup",
        "`x:pageSetup` (rank 15) — the **full** `CT_PageSetup`."
    );
    sheet_slot!(
        MacroSheetContent,
        header_footer,
        header_footer_mut,
        set_header_footer,
        HeaderFooter,
        HeaderFooter,
        "headerFooter",
        "`x:headerFooter` (rank 16) — the six opaque code strings, never re-serialised."
    );
    sheet_slot!(
        MacroSheetContent,
        row_breaks,
        row_breaks_mut,
        set_row_breaks,
        RowBreaks,
        PageBreaks,
        "rowBreaks",
        "`x:rowBreaks` (rank 17) — `CT_PageBreak` in the row axis."
    );
    sheet_slot!(
        MacroSheetContent,
        column_breaks,
        column_breaks_mut,
        set_column_breaks,
        ColumnBreaks,
        PageBreaks,
        "colBreaks",
        "`x:colBreaks` (rank 18) — the same complex type in the column axis."
    );
    sheet_slot!(
        MacroSheetContent,
        custom_properties,
        custom_properties_mut,
        set_custom_properties,
        CustomProperties,
        CustomProperties,
        "customProperties",
        "`x:customProperties` (rank 19) — the custom-property parts hung off this sheet."
    );
    sheet_slot!(
        MacroSheetContent,
        drawing,
        drawing_mut,
        set_drawing,
        Drawing,
        SheetDrawing,
        "drawing",
        "`x:drawing` (rank 20) — the relationship to a drawings part."
    );
    sheet_slot!(
        MacroSheetContent,
        background_picture,
        background_picture_mut,
        set_background_picture,
        BackgroundPicture,
        SheetBackgroundPicture,
        "picture",
        "`x:picture` (rank 24) — the image drawn behind the sheet, named by an `r:id`."
    );

    /// Every `x:cols` block, in document order.
    ///
    /// A list, not an `Option`: `CT_Macrosheet` declares this slot `maxOccurs="unbounded"`, exactly
    /// as `CT_Worksheet` does, and merging the blocks would change the file.
    pub fn column_blocks(&self) -> impl Iterator<Item = &ColumnBlock> + '_ {
        self.frame.children().filter_map(|child| match child {
            MacroSheetContent::Columns(block) => Some(block),
            _ => None,
        })
    }

    /// Every `x:conditionalFormatting` block, in document order.
    ///
    /// A list for the same reason [`column_blocks`](Self::column_blocks) is: the slot is
    /// `maxOccurs="unbounded"`, each block carries its own `@sqref`, and `cfRule@priority` orders
    /// **across** them.
    pub fn conditional_formatting_blocks(
        &self,
    ) -> impl Iterator<Item = &ConditionalFormatting> + '_ {
        self.frame.children().filter_map(|child| match child {
            MacroSheetContent::ConditionalFormatting(block) => Some(block),
            _ => None,
        })
    }
}
