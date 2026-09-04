//! `mjx-docx` — WordprocessingML: document body, styles, numbering, sections, headers/footers.
//!
//! The entry point is [`Document`]: open a `.docx`'s container bytes with [`Document::open`] — or
//! start from nothing with [`Document::blank`] — read its part graph with [`Document::parts`], and
//! save with [`Document::save`]. It owns an
//! [`mjx_opc::Package`] and resolves the part graph `tests/fixtures/sample.docx` and a richer
//! document alike carry; everything it does not yet model is preserved verbatim by the OPC
//! copy-on-write layer.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = std::fs::read("document.docx")?;
//! let mut document = mjx_docx::Document::open(&bytes)?;
//! println!("{:?}", document.conformance()?);
//! let saved = document.save()?;
//! # let _ = saved;
//! # Ok(())
//! # }
//! ```
//!
//! See `crates/mjx-docx/src/document/mod.rs`'s own doc comment for the module layout this crate is
//! built on and the plan for the files later children add.

mod address;
mod blank;
pub mod constants;
mod document;
pub mod effective_properties;
mod error;
pub mod guide;
mod page;

pub use address::{BlockPath, RunPath};
pub use document::{
    applicable_regions, AbstractNumbering, AbstractNumberingContent, Background, BlockContent,
    Body, Border, BottomPageBorder, Break, Cell, CellBorderContent, CellBorderEdge, CellBorders,
    CellHeaderReferences, CellMargins, CellProperties, CellPropertiesContent, CellTextDirection,
    CellVerticalAlignment, CharacterStyle, Color, Column, Columns, ColumnsContent,
    ConditionalFormatRegion, ConditionalFormatting, ConditionalFormattingBits, DecimalNumberValue,
    DefaultParagraphProperties, DefaultParagraphPropertyContent, DefaultRunProperties,
    DefaultRunPropertyContent, Document, DocumentDefaults, DocumentDefaultsContent, DocumentGrid,
    DocumentParts, EastAsianLayout, EffectiveBorder, EffectiveCharacterProperties, EffectiveColor,
    EffectiveConditionalFormatting, EffectiveEastAsianLayout, EffectiveFonts,
    EffectiveFrameProperties, EffectiveIndentation, EffectiveLanguages, EffectiveManualRunWidth,
    EffectiveNumberingReference, EffectiveParagraphBorders, EffectiveParagraphProperties,
    EffectiveShading, EffectiveTabStop, EffectiveUnderline, Emphasis, FloatingTableOverlap,
    FloatingTablePosition, Fonts, FrameProperties, Grid, GridColumn, GridContent, GridDiscrepancy,
    HalfPoint, HalfPointMeasureValue, HdrFtr, HeaderFooterReference, HeaderFooterType,
    HeaderReferenceContent, HexColor, HexIdentifier, Highlight, Hyperlink, Indentation, Lang,
    Languages, LatentStyleContent, LatentStyleException, LatentStyles, LevelLegacyFormatting,
    LevelNumberFormat, LevelSuffix, LevelTextSegment, LevelTextTemplate, LineNumbering,
    LineSpacing, LinkedStyleResolution, LongHex, MainDocument, MainDocumentContent, ManualRunWidth,
    MarginContent, MergeMarker, MergedCellType, MultiLevelKind, Numbering, NumberingContent,
    NumberingIndex, NumberingInstance, NumberingInstanceContent, NumberingLevel,
    NumberingLevelContent, NumberingLevelOverride, NumberingLevelOverrideContent, NumberingLookup,
    NumberingPictureBullet, NumberingPictureBulletContent, NumberingProperties,
    NumberingPropertyContent, NumberingResolution, PageBorder, PageBorderSet, PageBorderSetContent,
    PageNumbering, PageVerticalAlignment, PaperSource, Paragraph, ParagraphAlignment,
    ParagraphBorderContent, ParagraphBorders, ParagraphContent, ParagraphMarkRunProperties,
    ParagraphMarkRunPropertyContent, ParagraphProperties, ParagraphPropertyContent, ParagraphStyle,
    ParagraphTextFlowDirection, PartKind, PermissionRangeEnd, PermissionRangeStart, PhoneticGuide,
    PhoneticGuideChild, PhoneticGuideContent, PhoneticGuideContentItem, PhoneticGuideProperties,
    PhoneticGuidePropertyContent, PhoneticGuideTextAlignment, PositionalTab, ProofingError,
    RelationshipReference, RevisionSaveId, Row, RowContent, RowHeight, RowProperties,
    RowPropertyContent, Run, RunInnerContent, RunProperties, RunPropertyContent, Scale,
    SectionLocation, SectionProperties, SectionPropertyContent, SectionSpan, SectionType, Shading,
    ShortHex, SignedHalfPoint, SignedHalfPointMeasureValue, SignedTwips, SignedTwipsMeasureValue,
    Spacing, StyleDefinition, StyleDefinitionContent, StyleIndex, StyleParagraphProperties,
    StyleParagraphPropertyContent, StyleSheet, StyleSheetContent, StyleString, Symbol, TabStop,
    TabStopContent, TabStops, Table, TableAlignment, TableBorderContent, TableBorders,
    TableCellMargins, TableContent, TableExceptionProperties, TableExceptionPropertyContent,
    TableLayout, TableLook, TableLookFlags, TableProperties, TablePropertyContent,
    TableStringValue, TableStyleOverride, TableStyleOverrideContent, TableWidth, TableWidthMeasure,
    Text, TextBoxTightWrapSetting, TextEffect, TextScaleValue, ThemeHexDigit, Toggle,
    TopPageBorder, Twips, Underline, Unmodeled, VerticalAlignment, VerticalCharacterAlignment,
    WhitespacePreservation, MAX_BASED_ON_CHAIN_DEPTH, MAX_NUM_STYLE_LINK_DEPTH,
};
pub use error::DocxError;
pub use page::{PageMargins, PageOrientation, PageSize};
// The OPC vocabulary a caller of this crate's own signatures must be able to name: the package
// `Document::from_package` takes, and the part names `DocumentParts` hands back. Re-exported so
// nothing downstream has to depend on `mjx-opc` to state a parameter type this crate chose — the
// same reasoning `mjx_pptx` documents for its own re-export of the same vocabulary.
pub use mjx_opc::{OpcError, Package, PartName, TargetMode};
