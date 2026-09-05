//! Worksheet tables: `x:table` and everything under it, plus the `x:tableParts` list a worksheet
//! reaches them through.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_Table` | 3946 | `x:table` — **the root of a part of its own** |
//! | `CT_TableColumns` | 3991 | `x:table/tableColumns` |
//! | `CT_TableColumn` | 3997 | `x:tableColumns/tableColumn` |
//! | `CT_TableFormula` | 4018 | `x:tableColumn/calculatedColumnFormula`, `x:tableColumn/totalsRowFormula` |
//! | `CT_XmlColumnPr` | 4039 | `x:tableColumn/xmlColumnPr` |
//! | `CT_TableStyleInfo` | 3984 | `x:table/tableStyleInfo` |
//! | `CT_TableParts` | 3179 | `x:tableParts` (rank **37** of `CT_Worksheet`) |
//! | `CT_TablePart` | 3185 | `x:tableParts/tablePart` |
//!
//! # A table is a part, and this crate never resolves the edge to it
//!
//! Every other worksheet feature in [`crate::features`] lives inside `xl/worksheets/sheetN.xml`. A
//! table does not: it is `xl/tables/tableN.xml`, a part of its own with its own content type, and the
//! worksheet reaches it through `tablePart@r:id`. [`TablePart`] therefore holds the **raw
//! relationship identifier** and nothing else, exactly as `CT_Sheet`, `CT_Drawing` and
//! `CT_ExternalReference` do — resolving one to a part is `mjx-xlsx`'s, in
//! `crates/mjx-xlsx/src/worksheet/tables.rs`. This crate has never heard of a package.
//!
//! # What is embedded, and what would have been a second model
//!
//! `CT_Table` embeds `CT_AutoFilter` and `CT_SortState`, the **same** two complex types MJXOFF-123
//! built for `CT_Worksheet`'s ranks 10 and 11. They are [`AutoFilter`] and [`SortState`] from
//! [`crate::features::filters`], reached from here, and there is no filter or sort model in this
//! file. A second one would be exactly the duplication the `mjx-sml`/`mjx-xlsx` split exists to
//! prevent, and MJXOFF-89 already deleted fourteen of that shape.
//!
//! Note the three *distinct* `sortState` slots, which are easy to conflate and are not the same
//! element: rank 1 of `CT_AutoFilter`, rank 11 of `CT_Worksheet`, and rank 1 of `CT_Table`. A table
//! carries its own, beside its own autofilter, and neither is the sheet's.
//!
//! # A formula here is text, on MJXOFF-115's terms
//!
//! `calculatedColumnFormula` is the expression Excel fills a whole column with, and
//! `totalsRowFormula` is the custom aggregation a `totalsRowFunction="custom"` column uses. Neither
//! is evaluated, expanded down the column, offset, or translated — **a calculated column is never
//! expanded into per-cell formulas** and a totals row is never computed. Text in, the same text out.
//!
//! [`TableFormula`] is its own type rather than [`FormulaElement`](crate::FormulaElement) because
//! `CT_TableFormula` is a `simpleContent` **extension** of `ST_Formula`: it declares an `@array` that
//! `formula`, `formula1` and `formula2` do not. The *content* is the same content, so the decoding
//! and the replay are `crate::formula::element`'s one implementation, called from both.
//!
//! # `@dxfId` is a position, and this cluster names nine of them
//!
//! Six on the table — `@headerRowDxfId`, `@dataDxfId`, `@totalsRowDxfId`, `@headerRowBorderDxfId`,
//! `@tableBorderDxfId`, `@totalsRowBorderDxfId` — and three more on **each column**:
//! `@headerRowDxfId`, `@dataDxfId` and `@totalsRowDxfId`. (A tenth is
//! [`TableStyleRegion`](crate::TableStyleRegion)'s, in `xl/styles.xml`.) Every one of them indexes
//! `xl/styles.xml`'s `dxfs`, exactly as `cfRule@dxfId` and `colorFilter@dxfId` do. [`DifferentialFormats`](crate::DifferentialFormats)' rule is unchanged:
//! **append, never reorder**, because inserting a `dxf` silently repoints every index above it. The
//! one allocator is
//! [`StylesheetPart::append_differential_format`](crate::StylesheetPart::append_differential_format)
//! and this file adds no second one.
//!
//! # Never silently resize, and never renumber
//!
//! Two rules, both of them about not being helpful:
//!
//! * **`@ref` and the two row counts move together or not at all.** A table's `@ref` includes its
//!   header rows and — §18.5.1.2, *"The reference shall include the totals row if it is shown"* — its
//!   totals rows. [`WorksheetTable::resize`] is therefore the only way to change any of the three:
//!   it takes all three, refuses a combination that does not fit, and there is deliberately **no**
//!   `set_range`. Nothing else in this workspace touches a table's `@ref` — editing a cell inside a
//!   table changes the cell and moves no boundary.
//! * **A table's `@id` is never renumbered.** §18.5.1.2: *"Each table in the workbook shall have a
//!   unique id."* Ids are what other records name a table by, so creating one allocates an unused id
//!   ([`Workbook::add_table`](https://docs.rs/mjx-xlsx)) and nothing anywhere renumbers an existing
//!   table.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawDocument, RawElement,
    RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::{WORKSHEET_TABLE, WORKSHEET_TABLE_COLUMN};
use mjx_ooxml_types::namespaces::SML;
use mjx_ooxml_types::spreadsheetml::{TableType, TotalsRowFunction};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRange;
use crate::error::SmlError;
use crate::formula::element::{decoded_text, replayed_children};
use crate::leaf::{attribute_bag, bag_without_declared_attributes, relationship_reference};
use crate::worksheet::rebuild_element;

use super::filters::{AutoFilter, SortState};

// -----------------------------------------------------------------------------------------------
// The leaves: the three complex types that declare no children a model has to place
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:tableStyleInfo` (`CT_TableStyleInfo`, `sml.xsd:3984`) — which style this table wears, and
    /// which parts of it the style is applied to.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_TableStyleInfo`. Wire element: `tableStyleInfo`. ECMA-376 Part 1
    /// §18.5.1.5 titles it *"Table Style"*, which is why this type is named for the **reference**
    /// rather than for the definition: the definition is
    /// [`TableStyleDefinition`](crate::TableStyleDefinition) in
    /// `xl/styles.xml`, and this is a table pointing at one by name.
    ///
    /// **`@name` naming a style the file does not define is the normal case, not a defect.**
    /// §18.5.1.5: *"If the style name does not correspond to the name of a table style then the
    /// spreadsheet application should use default style."* Excel's presets — `TableStyleMedium2` and
    /// its 143 siblings — are in no `.xlsx` at all. See
    /// [`TableStyles::lookup`](crate::TableStyles::lookup), which reports the three cases apart.
    ///
    /// The four flags are `xsd:boolean` with **no schema default**, so an absent one is genuinely
    /// absent and this type answers `None` for it rather than inventing a value.
    #[xml(attribute(local = "name", codec = Text, accessor = name))]
    #[xml(attribute(local = "showFirstColumn", codec = OnOff, accessor = shows_first_column))]
    #[xml(attribute(local = "showLastColumn", codec = OnOff, accessor = shows_last_column))]
    #[xml(attribute(local = "showRowStripes", codec = OnOff, accessor = shows_row_stripes))]
    #[xml(attribute(local = "showColumnStripes", codec = OnOff, accessor = shows_column_stripes))]
    TableStyleReference, "tableStyleInfo"
}

attribute_bag! {
    /// `x:xmlColumnPr` (`CT_XmlColumnPr`, `sml.xsd:4039`) — the XML-map binding on one table column.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_XmlColumnPr`. Wire element: `xmlColumnPr`.
    ///
    /// Three of its four attributes are `use="required"`: `@mapId`, the XML map this column is bound
    /// through; `@xpath`, the path inside the mapped document; and `@xmlDataType`, whose simple type
    /// `ST_XmlDataType` is an **unrestricted `xsd:string`** — the schema states no enumeration for
    /// it, so this crate carries it as text rather than inventing one.
    ///
    /// The map itself lives in an `xl/xmlMaps.xml` part this workspace does not model. It is
    /// preserved, like every other part nothing here claims.
    #[xml(attribute(local = "mapId", codec = Number<u32>, accessor = map_id, required))]
    #[xml(attribute(local = "xpath", codec = Text, accessor = xpath, required))]
    #[xml(attribute(local = "denormalized", codec = OnOff, accessor = is_denormalized, default = false))]
    #[xml(attribute(local = "xmlDataType", codec = Text, accessor = xml_data_type, required))]
    XmlColumnProperties, "xmlColumnPr"
}

bag_without_declared_attributes! {
    /// `x:tablePart` (`CT_TablePart`, `sml.xsd:3185`) — one worksheet-to-table edge.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_TablePart`. Wire element: `tablePart`.
    ///
    /// Its only attribute is `r:id`, `use="required"`, whose *prefix* is the file's choice rather
    /// than the schema's — which is why it is reached through
    /// [`relationship_id`](Self::relationship_id) with the part's own prefix rather than declared
    /// through the attribute grammar. The identifier is held as the string the file wrote:
    /// resolving it to a part is `mjx-xlsx`'s.
    TablePart, "tablePart"
}

relationship_reference!(TablePart);

// -----------------------------------------------------------------------------------------------
// `x:tableParts` — the worksheet's list of the tables on it
// -----------------------------------------------------------------------------------------------

/// `x:tableParts` (`CT_TableParts`, `sml.xsd:3179`) — every table on this sheet, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_TableParts`. Wire element: `tableParts`, rank **37** of
/// `CT_Worksheet` — the second-to-last slot, just before `extLst`.
///
/// Unlike `CT_MergeCells`, `tablePart` is `minOccurs="0"`, so an empty `<tableParts/>` is valid
/// markup and a file that writes one keeps it.
///
/// `@count` is a producer's cache. It is refreshed when this collection is edited **and the file
/// declared one**, and never added to an element that wrote none — the rule every counted collection
/// in this crate follows.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct TableParts {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tablePart", variant = Part, ty = TablePart))]
    content: Vec<TablePartsContent>,
}

/// One child of [`TableParts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TablePartsContent {
    /// `x:tablePart` — one edge to a table part.
    Part(TablePart),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl TableParts {
    /// Builds an empty `x:tableParts`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "tableParts"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[TablePartsContent] {
        &self.content
    }

    /// Every `x:tablePart`, in document order.
    pub fn parts(&self) -> impl Iterator<Item = &TablePart> + '_ {
        self.content.iter().filter_map(|item| match item {
            TablePartsContent::Part(part) => Some(part),
            TablePartsContent::Raw(_) => None,
        })
    }

    /// How many tables the sheet lists.
    #[must_use]
    pub fn len(&self) -> usize {
        self.parts().count()
    }

    /// Whether the sheet lists no table at all, which is valid markup.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends an edge after the ones already present, refreshing `@count` when the file declared
    /// one.
    pub fn push(&mut self, interner: &mut Interner, part: TablePart) {
        self.content.push(TablePartsContent::Part(part));
        self.empty = false;
        self.refresh_count(interner);
    }

    /// Writes `@count` from the edges actually present — but only onto an element that already
    /// declared one.
    fn refresh_count(&mut self, interner: &mut Interner) {
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                TablePartsContent::Part(part) => RawNode::Element(part.as_raw_element()),
                TablePartsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for TableParts {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `CT_TableFormula` — an `ST_Formula` with an `@array`
// -----------------------------------------------------------------------------------------------

/// `x:calculatedColumnFormula` / `x:totalsRowFormula` (`CT_TableFormula`, `sml.xsd:4018`) — a
/// formula a table column carries.
///
/// **`ST_`/`CT_` symbol:** `CT_TableFormula`. Wire elements: `calculatedColumnFormula`,
/// `totalsRowFormula`.
///
/// A `simpleContent` extension of `ST_Formula` with one attribute, `@array` (default `false`), which
/// says the expression is an array formula. The character data is replayed **byte for byte** —
/// entity spellings and CDATA sections included — until [`set_text`](Self::set_text) replaces it,
/// which is [`FormulaElement`](crate::FormulaElement)'s contract and this type shares its
/// implementation of it.
///
/// **Nothing evaluates it.** A calculated column is not expanded into per-cell formulas on read and
/// not collapsed from them on write; a totals row is not computed. See this module's own
/// documentation, and [`crate::formula`] for the whole of MJXOFF-115's contract.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "array", codec = OnOff, accessor = is_array, default = false))]
pub struct TableFormula {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    /// The character data, decoded — what [`text`](Self::text) answers with.
    text: String,
    /// The element's children exactly as the file wrote them, or `None` once the text has been
    /// replaced and there is nothing left to preserve.
    verbatim: Option<Vec<RawNode>>,
}

impl TableFormula {
    /// Builds an element named `local` holding `text`, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `local` is the wire name of the slot being filled — `"calculatedColumnFormula"` or
    /// `"totalsRowFormula"`. It is a parameter for the reason
    /// [`FormulaElement::new`](crate::FormulaElement::new)'s is: one complex type serves two slots.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        prefix: Option<&str>,
        local: &str,
        text: impl Into<String>,
    ) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, local),
            attributes: Vec::new(),
            empty: false,
            text: text.into(),
            verbatim: None,
        }
    }

    /// The formula's text, exactly as the file wrote it (entity references decoded).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text. Nothing validates it, and nothing evaluates it.
    ///
    /// This is the point at which the preserved character data is given up.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.verbatim = None;
        self.empty = false;
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = replayed_children(self.verbatim.as_ref(), &self.text);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

impl FromXml for TableFormula {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            text: decoded_text(element)?,
            verbatim: Some(element.children.clone()),
        })
    }
}

impl ToXml for TableFormula {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:tableColumn` — one column of a table
// -----------------------------------------------------------------------------------------------

/// `x:tableColumn` (`CT_TableColumn`, `sml.xsd:3997`) — one column of a table.
///
/// **`ST_`/`CT_` symbol:** `CT_TableColumn`. Wire element: `tableColumn`.
///
/// `@id` is `use="required"` and unique within the table; `@name` is `use="required"` and is the
/// heading text, which is *also* what a structured reference (`Table1[Region]`) names — text this
/// library carries inside formulas and never parses. `@uniqueName` is what a producer falls back on
/// when two columns would otherwise share a heading.
///
/// `@totalsRowFunction` defaults to `none`. Its generated variant names are **not** its wire tokens:
/// `count` is [`TotalsRowFunction::CountNonEmpty`], `countNums` is `CountNumbers`, `stdDev` is
/// `EstimatedStandardDeviation`, `var` is `EstimatedVariance` and `custom` is `CustomFormula` — read
/// the enumeration rather than spelling a token by hand.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "id", codec = Number<u32>, accessor = id, required))]
#[xml(attribute(local = "uniqueName", codec = Text, accessor = unique_name))]
#[xml(attribute(local = "name", codec = Text, accessor = name, required))]
#[xml(attribute(
    local = "totalsRowFunction",
    codec = Enumeration<TotalsRowFunction>,
    accessor = totals_row_function,
    default = TotalsRowFunction::None
))]
#[xml(attribute(local = "totalsRowLabel", codec = Text, accessor = totals_row_label))]
#[xml(attribute(local = "queryTableFieldId", codec = Number<u32>, accessor = query_table_field_id))]
#[xml(attribute(local = "headerRowDxfId", codec = Number<u32>, accessor = header_row_format_index))]
#[xml(attribute(local = "dataDxfId", codec = Number<u32>, accessor = data_format_index))]
#[xml(attribute(local = "totalsRowDxfId", codec = Number<u32>, accessor = totals_row_format_index))]
#[xml(attribute(local = "headerRowCellStyle", codec = Text, accessor = header_row_cell_style))]
#[xml(attribute(local = "dataCellStyle", codec = Text, accessor = data_cell_style))]
#[xml(attribute(local = "totalsRowCellStyle", codec = Text, accessor = totals_row_cell_style))]
pub struct TableColumn {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "calculatedColumnFormula", variant = CalculatedColumnFormula, ty = TableFormula),
        child(local = "totalsRowFormula", variant = TotalsRowFormula, ty = TableFormula),
        child(local = "xmlColumnPr", variant = XmlColumnProperties, ty = XmlColumnProperties)
    )]
    content: Vec<TableColumnContent>,
}

/// One child of [`TableColumn`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableColumnContent {
    /// `x:calculatedColumnFormula` (rank 0) — the expression every data cell of the column carries.
    CalculatedColumnFormula(TableFormula),
    /// `x:totalsRowFormula` (rank 1) — the custom aggregation a `totalsRowFunction="custom"` column
    /// uses.
    TotalsRowFormula(TableFormula),
    /// `x:xmlColumnPr` (rank 2) — the XML-map binding.
    XmlColumnProperties(XmlColumnProperties),
    /// `x:extLst` (rank 3) — and anything else, preserved verbatim and in position.
    Raw(RawNode),
}

impl TableColumnContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::CalculatedColumnFormula(_) => "calculatedColumnFormula",
            Self::TotalsRowFormula(_) => "totalsRowFormula",
            Self::XmlColumnProperties(_) => "xmlColumnPr",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_TableColumn`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        WORKSHEET_TABLE_COLUMN.rank_of(None, self.local()?)
    }
}

/// Declares one of [`TableColumn`]'s singleton slots: a getter, a mutable getter and a setter that
/// replaces the existing child in place or inserts one at its rank.
macro_rules! column_slot {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                TableColumnContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the column writes none.")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> Option<&mut $ty> {
            self.content.iter_mut().find_map(|item| match item {
                TableColumnContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some` replaces the existing \
            element **where it is**, or inserts one at its rank in `CT_TableColumn`'s \
            `xsd:sequence`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let existing = self
                .content
                .iter()
                .position(|item| matches!(item, TableColumnContent::$variant(_)));
            match (existing, value) {
                (Some(at), Some(value)) => self.content[at] = TableColumnContent::$variant(value),
                (Some(at), None) => {
                    self.content.remove(at);
                }
                (None, Some(value)) => {
                    let at = WORKSHEET_TABLE_COLUMN.insert_index_of_names(
                        self.content.iter().map(TableColumnContent::rank),
                        $local,
                    );
                    self.content.insert(at, TableColumnContent::$variant(value));
                    self.empty = false;
                }
                (None, None) => {}
            }
        }
    };
}

impl TableColumn {
    /// Builds an `x:tableColumn` with no attribute at all, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `@id` and `@name` are both `use="required"` and neither is set here: a column id or a heading
    /// invented on a caller's behalf would name a column they did not describe. Build the element
    /// and then state them — or use [`TableColumnSpec`](crate::TableColumnSpec), which requires
    /// both.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "tableColumn"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[TableColumnContent] {
        &self.content
    }

    column_slot!(
        calculated_column_formula,
        calculated_column_formula_mut,
        set_calculated_column_formula,
        CalculatedColumnFormula,
        TableFormula,
        "calculatedColumnFormula",
        "`x:calculatedColumnFormula` — the expression every data cell of this column carries. \
         **Never expanded into per-cell formulas**, and never evaluated."
    );
    column_slot!(
        totals_row_formula,
        totals_row_formula_mut,
        set_totals_row_formula,
        TotalsRowFormula,
        TableFormula,
        "totalsRowFormula",
        "`x:totalsRowFormula` — the custom aggregation this column's totals cell uses. §18.5.1.6 \
         says `@totalsRowFunction` *\"shall be set to `custom`\"* when it is present; nothing here \
         enforces that on a file, and nothing computes the total."
    );
    column_slot!(
        xml_column_properties,
        xml_column_properties_mut,
        set_xml_column_properties,
        XmlColumnProperties,
        XmlColumnProperties,
        "xmlColumnPr",
        "`x:xmlColumnPr` — the XML-map binding, preserved. The map part it names is not modelled \
         anywhere in this workspace."
    );

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                TableColumnContent::CalculatedColumnFormula(formula)
                | TableColumnContent::TotalsRowFormula(formula) => {
                    RawNode::Element(formula.as_raw_element())
                }
                TableColumnContent::XmlColumnProperties(properties) => {
                    RawNode::Element(properties.as_raw_element())
                }
                TableColumnContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for TableColumn {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:tableColumns` — the column list
// -----------------------------------------------------------------------------------------------

/// `x:tableColumns` (`CT_TableColumns`, `sml.xsd:3991`) — every column of a table, in document
/// order.
///
/// **`ST_`/`CT_` symbol:** `CT_TableColumns`. Wire element: `tableColumns`, rank 2 of `CT_Table` and
/// its one `minOccurs="1"` child.
///
/// `tableColumn` is itself `minOccurs="1"`, so a table that writes this element writes at least one
/// column — which is why [`WorksheetTable`] never removes the last one.
///
/// `@count` is a producer's cache, and one that files really do get wrong. It is refreshed when this
/// collection is edited **and the file declared one**, and never added to an element that wrote
/// none. A stale count on a table nobody edited is left exactly as it was found: correcting a cache
/// nobody asked about is the repair this phase keeps refusing.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct TableColumns {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tableColumn", variant = Column, ty = TableColumn))]
    content: Vec<TableColumnsContent>,
}

/// One child of [`TableColumns`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableColumnsContent {
    /// `x:tableColumn` — one column.
    Column(TableColumn),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl TableColumns {
    /// Builds an empty `x:tableColumns`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `tableColumn` `minOccurs="1"`, so a list with no column is invalid; it is
    /// still constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "tableColumns"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[TableColumnsContent] {
        &self.content
    }

    /// Every `x:tableColumn`, in document order — **which is left-to-right order in the table**.
    pub fn columns(&self) -> impl Iterator<Item = &TableColumn> + '_ {
        self.content.iter().filter_map(|item| match item {
            TableColumnsContent::Column(column) => Some(column),
            TableColumnsContent::Raw(_) => None,
        })
    }

    /// How many columns the table has — which is what `@count` claims and is not always what it
    /// says.
    #[must_use]
    pub fn len(&self) -> usize {
        self.columns().count()
    }

    /// Whether the list holds no column at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:tableColumn`, mutably.
    pub fn column_mut(&mut self, index: usize) -> Option<&mut TableColumn> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                TableColumnsContent::Column(column) => Some(column),
                TableColumnsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a column after the ones already present, refreshing `@count` when the file declared
    /// one.
    ///
    /// **This does not widen the table's `@ref`.** A column added here without the enclosing
    /// [`WorksheetTable::resize`] leaves a table whose range says one thing and whose column list
    /// says another — which is why every authoring path in this workspace goes through
    /// [`WorksheetTableSpec`](crate::WorksheetTableSpec) or through `resize`.
    pub fn push(&mut self, interner: &mut Interner, column: TableColumn) {
        self.content.push(TableColumnsContent::Column(column));
        self.empty = false;
        self.refresh_count(interner);
    }

    /// Writes `@count` from the columns actually present — but only onto an element that already
    /// declared one.
    fn refresh_count(&mut self, interner: &mut Interner) {
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                TableColumnsContent::Column(column) => RawNode::Element(column.as_raw_element()),
                TableColumnsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for TableColumns {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:table` — the whole part
// -----------------------------------------------------------------------------------------------

/// `xl/tables/tableN.xml` — `x:table` (`CT_Table`, `sml.xsd:3946`), the whole table part.
///
/// **`ST_`/`CT_` symbol:** `CT_Table`. Wire element: `table`, a **global** element of `sml.xsd` and
/// the root of a part of its own.
///
/// See the [module documentation](self) for the part boundary, for the two rules about resizing and
/// renumbering, and for why the autofilter and the sort state are MJXOFF-123's types rather than new
/// ones.
///
/// # The five slots
///
/// | rank | element | held as |
/// |---|---|---|
/// | 0 | `autoFilter` | [`AutoFilter`] |
/// | 1 | `sortState` | [`SortState`] |
/// | 2 | `tableColumns` | [`TableColumns`] — `minOccurs="1"` |
/// | 3 | `tableStyleInfo` | [`TableStyleReference`] |
/// | 4 | `extLst` | [`WorksheetTableContent::Raw`], on purpose and for good |
///
/// The ranks are never written down: every placement goes through
/// [`mjx_ooxml_types::child_order::WORKSHEET_TABLE`], generated from `sml.xsd` by
/// `cargo run -p xtask -- codegen`.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "id", codec = Number<u32>, accessor = id, required))]
#[xml(attribute(local = "name", codec = Text, accessor = name))]
#[xml(attribute(local = "displayName", codec = Text, accessor = display_name, required))]
#[xml(attribute(local = "comment", codec = Text, accessor = comment))]
#[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
#[xml(attribute(
    local = "tableType",
    codec = Enumeration<TableType>,
    accessor = table_type,
    default = TableType::Worksheet
))]
#[xml(attribute(local = "headerRowCount", codec = Number<u32>, accessor = header_row_count, default = 1))]
#[xml(attribute(local = "insertRow", codec = OnOff, accessor = shows_insert_row, default = false))]
#[xml(attribute(local = "insertRowShift", codec = OnOff, accessor = insert_row_shifted_cells, default = false))]
#[xml(attribute(local = "totalsRowCount", codec = Number<u32>, accessor = totals_row_count, default = 0))]
#[xml(attribute(local = "totalsRowShown", codec = OnOff, accessor = totals_row_shown, default = true))]
#[xml(attribute(local = "published", codec = OnOff, accessor = is_published, default = false))]
#[xml(attribute(local = "headerRowDxfId", codec = Number<u32>, accessor = header_row_format_index))]
#[xml(attribute(local = "dataDxfId", codec = Number<u32>, accessor = data_format_index))]
#[xml(attribute(local = "totalsRowDxfId", codec = Number<u32>, accessor = totals_row_format_index))]
#[xml(attribute(
    local = "headerRowBorderDxfId",
    codec = Number<u32>,
    accessor = header_row_border_format_index
))]
#[xml(attribute(local = "tableBorderDxfId", codec = Number<u32>, accessor = table_border_format_index))]
#[xml(attribute(
    local = "totalsRowBorderDxfId",
    codec = Number<u32>,
    accessor = totals_row_border_format_index
))]
#[xml(attribute(local = "headerRowCellStyle", codec = Text, accessor = header_row_cell_style))]
#[xml(attribute(local = "dataCellStyle", codec = Text, accessor = data_cell_style))]
#[xml(attribute(local = "totalsRowCellStyle", codec = Text, accessor = totals_row_cell_style))]
#[xml(attribute(local = "connectionId", codec = Number<u32>, accessor = connection_id))]
pub struct WorksheetTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "autoFilter", variant = AutoFilter, ty = AutoFilter),
        child(local = "sortState", variant = SortState, ty = SortState),
        child(local = "tableColumns", variant = Columns, ty = TableColumns),
        child(local = "tableStyleInfo", variant = Style, ty = TableStyleReference)
    )]
    content: Vec<WorksheetTableContent>,
}

/// One child of [`WorksheetTable`]: four modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorksheetTableContent {
    /// `x:autoFilter` (rank 0) — the table's own filter, which is **not** the sheet's. Recorded,
    /// never applied.
    AutoFilter(AutoFilter),
    /// `x:sortState` (rank 1) — the table's own recorded sort, which is neither the sheet's nor the
    /// autofilter's. Recorded, never performed.
    SortState(SortState),
    /// `x:tableColumns` (rank 2) — `minOccurs="1"`.
    Columns(TableColumns),
    /// `x:tableStyleInfo` (rank 3).
    Style(TableStyleReference),
    /// `x:extLst` (rank 4) — and anything else, preserved verbatim and in position.
    Raw(RawNode),
}

impl WorksheetTableContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::AutoFilter(_) => "autoFilter",
            Self::SortState(_) => "sortState",
            Self::Columns(_) => "tableColumns",
            Self::Style(_) => "tableStyleInfo",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Table`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        WORKSHEET_TABLE.rank_of(None, self.local()?)
    }
}

/// Declares one of [`WorksheetTable`]'s singleton slots.
macro_rules! table_slot {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                WorksheetTableContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the table writes none.")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> Option<&mut $ty> {
            self.content.iter_mut().find_map(|item| match item {
                WorksheetTableContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some` replaces the existing \
            element **where it is**, or inserts one at its rank in `CT_Table`'s `xsd:sequence`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let existing = self
                .content
                .iter()
                .position(|item| matches!(item, WorksheetTableContent::$variant(_)));
            match (existing, value) {
                (Some(at), Some(value)) => {
                    self.content[at] = WorksheetTableContent::$variant(value);
                }
                (Some(at), None) => {
                    self.content.remove(at);
                }
                (None, Some(value)) => {
                    let at = WORKSHEET_TABLE.insert_index_of_names(
                        self.content.iter().map(WorksheetTableContent::rank),
                        $local,
                    );
                    self.content
                        .insert(at, WorksheetTableContent::$variant(value));
                    self.empty = false;
                }
                (None, None) => {}
            }
        }
    };
}

impl WorksheetTable {
    /// Reads a whole `xl/tables/tableN.xml` part.
    ///
    /// `Ok(None)` when the document's root is not an `x:table` — the caller handed over a different
    /// part, which is a question rather than an error, exactly as
    /// [`StylesheetPart::read_part`](crate::StylesheetPart::read_part) treats it.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a modelled element does not match the shape its complex type declares.
    /// Nothing a well-formed file can *say* is refused.
    pub fn read_part(document: &RawDocument) -> Result<Option<Self>, SmlError> {
        Self::read_root(&document.root, &document.interner)
    }

    /// [`read_part`](Self::read_part) for a caller that holds the root element and the interner
    /// rather than the whole document — which is the shape an *editing* caller is in.
    ///
    /// # Errors
    /// As [`read_part`](Self::read_part).
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        let namespace = root.name.namespace.map(|symbol| interner.resolve(symbol));
        let in_spreadsheetml =
            namespace == Some(SML.transitional) || (namespace.is_some() && namespace == SML.strict);
        if !in_spreadsheetml || interner.resolve(root.name.local) != "table" {
            return Ok(None);
        }
        Ok(Some(Self::from_xml(root, interner)?))
    }

    /// Builds an `x:table` with no attribute at all, bound to `prefix` or to the default namespace.
    ///
    /// `@id`, `@displayName` and `@ref` are all `use="required"` and none is set here, for the reason
    /// [`TableColumn::new`] leaves `@id` and `@name` unset.
    /// [`AuthoredTable`](crate::write::AuthoredTable) is what builds a whole, valid part.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "table"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// The prefix this part binds to the SpreadsheetML namespace, or `None` for the default
    /// namespace — what a caller authoring a child element has to pass along so the new element is
    /// spelled the way the rest of the part is.
    #[must_use]
    pub fn element_prefix<'a>(&self, interner: &'a Interner) -> Option<&'a str> {
        self.name.prefix.map(|symbol| interner.resolve(symbol))
    }

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[WorksheetTableContent] {
        &self.content
    }

    table_slot!(
        auto_filter,
        auto_filter_mut,
        set_auto_filter,
        AutoFilter,
        AutoFilter,
        "autoFilter",
        "`x:autoFilter` — **the table's own** filter, distinct from the sheet's at rank 10 of \
         `CT_Worksheet`. Recorded, never applied: no row's `@hidden` is set from it."
    );
    table_slot!(
        sort_state,
        sort_state_mut,
        set_sort_state,
        SortState,
        SortState,
        "sortState",
        "`x:sortState` — **the table's own** recorded sort, distinct both from the sheet's at rank \
         11 of `CT_Worksheet` and from the one the autofilter above may carry at its own rank 1. \
         Recorded, never performed."
    );
    table_slot!(
        columns,
        columns_mut,
        set_columns,
        Columns,
        TableColumns,
        "tableColumns",
        "`x:tableColumns` — the column list. `minOccurs=\"1\"`, so a schema-valid table always has \
         one; a file that omits it is read, reported as `None`, and not repaired."
    );
    table_slot!(
        style,
        style_mut,
        set_style,
        Style,
        TableStyleReference,
        "tableStyleInfo",
        "`x:tableStyleInfo` — which style this table wears. `None` means the table names no style \
         at all, which is a different statement from naming one the file does not define; see \
         [`TableStyles::lookup`](crate::TableStyles::lookup)."
    );

    /// Every column, in left-to-right order — the empty iterator for a table with no
    /// `x:tableColumns`.
    pub fn table_columns(&self) -> impl Iterator<Item = &TableColumn> + '_ {
        self.columns().into_iter().flat_map(TableColumns::columns)
    }

    /// How many rows of `@ref` are data rows: the range's height less the header rows and the totals
    /// rows.
    ///
    /// `None` when the counts do not fit inside the range — which is a **report about the file**,
    /// not a repair of it. A table saying `ref="A1:C3" headerRowCount="1" totalsRowCount="9"` keeps
    /// saying exactly that, and answers `None` here.
    ///
    /// # Errors
    /// [`SmlError::Model`] if `@ref` is absent or will not parse, or if `@headerRowCount` or
    /// `@totalsRowCount` is not an `xsd:unsignedInt`.
    pub fn data_row_count(&self, interner: &Interner) -> Result<Option<u32>, SmlError> {
        let range = self.range(interner).map_err(FromXmlError::from)?;
        let header = self
            .header_row_count(interner)
            .map_err(FromXmlError::from)?;
        let totals = self
            .totals_row_count(interner)
            .map_err(FromXmlError::from)?;
        Ok(data_rows(range, header, totals))
    }

    /// Moves the table to `range`, with `header_rows` header rows and `totals_rows` totals rows —
    /// **all three together, or none of them**.
    ///
    /// There is no `set_range`, and that is the point. §18.5.1.2 says `@ref` *"shall include the
    /// totals row if it is shown"*, so the range and the two counts are one statement in three
    /// attributes: changing the range alone turns a table with a totals row into a table whose
    /// totals row is outside itself, and changing a count alone does the same from the other side.
    /// This method writes all three or writes nothing.
    ///
    /// It also does not touch a single cell. Moving a table's boundary neither creates the cells
    /// inside the new range nor clears the ones outside it — the same rule
    /// [`WorksheetPart::merge_cells`](crate::WorksheetPart::merge_cells) states for merging, and for
    /// the same reason: the alternative is destroying data nobody asked to lose.
    ///
    /// # Errors
    /// [`SmlError::TableGeometryDoesNotFit`] when `header_rows + totals_rows` exceeds the range's
    /// height — the table would have no data rows and its own totals row would fall outside it.
    pub fn resize(
        &mut self,
        interner: &mut Interner,
        range: CellRange,
        header_rows: u32,
        totals_rows: u32,
    ) -> Result<(), SmlError> {
        if data_rows(range, header_rows, totals_rows).is_none() {
            return Err(SmlError::TableGeometryDoesNotFit {
                range: range.to_string(),
                header_rows,
                totals_rows,
            });
        }
        self.set_range(interner, range);
        self.set_header_row_count(interner, Some(header_rows));
        self.set_totals_row_count(interner, Some(totals_rows));
        Ok(())
    }
}

/// How many data rows a range of this height has once its header and totals rows are taken off, or
/// `None` when they do not fit.
///
/// The height comes from [`CellRange::normalized_bounds`], so a range written bottom-up (`C5:A1`) is
/// measured the same as the one written top-down — ordering happens there and only there.
fn data_rows(range: CellRange, header_rows: u32, totals_rows: u32) -> Option<u32> {
    let bounds = range.normalized_bounds();
    let height = bounds
        .last_row()
        .saturating_sub(bounds.first_row())
        .checked_add(1)?;
    let reserved = header_rows.checked_add(totals_rows)?;
    height.checked_sub(reserved)
}
