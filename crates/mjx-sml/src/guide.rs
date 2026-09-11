// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s,
// `mjx_docx::guide`'s, `mjx_xlsx::guide`'s, `mjx_ooxml::guide`'s, `mjx_opc::guide`'s and
// `mjx_dml::guide`'s shape — see any of them for why each module imports the crate's public
// vocabulary: so that the guide's intra-doc links resolve.
//
// Two of the six pages were already written and were sitting *beside* a crate with no guide:
// `the_cell_store` and `shared_strings` are MJXOFF-95's and MJXOFF-97's decision records, moved into
// this set by MJXOFF-220 rather than left as orphans. They read as design notes because that is what
// they are, and they carry numbers and machines the rest of the set refers back to.
//
// This set is written for a caller holding a *part* — a worksheet, a stylesheet, a workbook — and
// for a crate being wired onto this one. It is deliberately not a tour of the public surface: this
// is the largest crate in the workspace, a page that walked sixteen hundred declarations would teach
// nobody anything, and rustdoc already carries every item's own doc comment.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            apply_tint, builtin_format_code, builtin_table_style_name, is_locale_dependent,
            parse_comments, styles, write, ApplyFlag, AutoFilter, Border, BorderEdgeSpec,
            BorderSpec, BorderTable, BuiltInTableStyle, CalculationChain, CalculationProperties,
            Cell, CellFormat, CellFormatResolver, CellFormatSpec, CellFormatTable,
            CellFormatTableKind, CellFormula, CellRange, CellRangeList, CellReference, CellValue,
            ChartSheetPart, Color, ColorElement, ColorScaleSpec, ColorTable, ColumnBlock,
            CommentText, Comments, ConditionalFormatting, ConditionalRuleChain,
            ConditionalRuleSpec, ConnectionIdentity, DataBarSpec, DataValidations, DialogSheetPart,
            DifferentialFormat, DifferentialFormatSpec, DifferentialFormats, EffectiveCellFormat,
            EmbeddedObjects, ExternalLinkIdentity, Fill, FillTable, Font, FontProperties,
            FontPropertyOwner, FontTable, FormControls, FormatLayer, GridAnomaly, HeaderFooter,
            InlineString, MacroSheetPart, MergedCells, NamedCellStyles, NumberFormatTable,
            ObjectAnchor, PageMargins, PageSetup, PatternFillSpec, PivotCacheIdentity,
            PivotTableIdentity, QueryTableIdentity, RevisionHeadersIdentity, RichTextRun,
            RichTextRunSpec, SharedFormulaGroups, SharedStringTable, SheetData, SheetDataAnomaly,
            SheetDimension, SheetViews, SmlError, SortState, StringItem, StylesheetPart,
            TableParts, TableStyleLookup, TableStyles, WorkbookPackage, WorkbookPart,
            WorksheetPart, WorksheetTable, WorksheetTableSpec, XmlMapIdentity,
        };
    };
}

guide_vocabulary!();

/// Who reaches this crate and for what, where the line with `mjx-xlsx` runs, and how to write a
/// package from nothing.
pub mod reaching_spreadsheetml {
    #![doc = include_str!("../docs/guide/reaching_spreadsheetml.md")]
    guide_vocabulary!();
}

/// What *held* means, why an untouched child comes back byte for byte, and where a new one is
/// placed.
pub mod the_slot_frame {
    #![doc = include_str!("../docs/guide/the_slot_frame.md")]
    guide_vocabulary!();
}

/// The packed representation a worksheet's cells are held in, what it costs per cell, and why it is
/// not a subtree (MJXOFF-95's decision record).
pub mod the_cell_store {
    #![doc = include_str!("../docs/guide/the_cell_store.md")]
    guide_vocabulary!();
}

/// The shared string table, its interner, the inline-string path beside it, and the two lifetime
/// policies (MJXOFF-97's decision record).
pub mod shared_strings {
    #![doc = include_str!("../docs/guide/shared_strings.md")]
    guide_vocabulary!();
}

/// The `xf` indirection, the resource tables that indices point into, and the four spellings of a
/// colour.
pub mod the_stylesheet {
    #![doc = include_str!("../docs/guide/the_stylesheet.md")]
    guide_vocabulary!();
}

/// The four serialization mechanisms, what backs each, half a schema preserved rather than modelled,
/// and every gap still open.
pub mod fidelity_and_gaps {
    #![doc = include_str!("../docs/guide/fidelity_and_gaps.md")]
    guide_vocabulary!();
}
