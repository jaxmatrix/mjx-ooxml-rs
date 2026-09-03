//! The **embedded workbook** — the `.xlsx` package that backs a chart's data, written from scratch.
//!
//! A real PowerPoint chart does not only carry cached values: it embeds a whole spreadsheet package
//! at `/ppt/embeddings/*.xlsx`, related from the chart part by
//! `.../relationships/package` and named by the chart's `c:externalData@r:id`. That workbook is what
//! PowerPoint's **Edit Data** opens. A chart with no workbook renders perfectly — the caches are what
//! draw — but Edit Data has nothing to show; a chart whose workbook disagrees with its caches shows
//! the *old* numbers there.
//!
//! This module writes such a workbook: one sheet, a shared-string table, and a styles skeleton. It is
//! deliberately a **writer only** and deliberately not a spreadsheet model — [`EmbeddedWorkbook`] is
//! a grid of [`WorkbookCell`]s and nothing more.
//!
//! # Scheduled removal — E1 is this module's executioner
//!
//! `mjx-xlsx` is a Phase D crate and cannot be depended on from here anyway (it sits *above* shared
//! markup in the layering, and `mjx-chart` must never point sideways or up). Until it exists, a chart
//! that wants a workbook has to write one itself, so this is the workspace's one deliberate duplicate.
//! **E1 removes this module** and routes [`EmbeddedWorkbook::to_package_bytes`] through the real
//! `mjx-xlsx` writer once Phase D lands. A duplicate with a scheduled removal is a debt; a duplicate
//! nobody removes is an architecture — so this note names the executioner, and the module stays small
//! enough to make the swap a deletion rather than a migration. Do not grow a second general-purpose
//! spreadsheet model here: if a need arises that this grid cannot express, that need belongs to
//! `mjx-xlsx`.
//!
//! ```
//! use mjx_chart::{ChartData, ChartKind, EmbeddedWorkbook};
//!
//! let chart = ChartData::new(ChartKind::Bar)
//!     .categories(["Q1", "Q2"])
//!     .series("Revenue", [10.0, 20.0]);
//! let bytes = EmbeddedWorkbook::for_chart_data(&chart)
//!     .to_package_bytes()
//!     .expect("write the workbook");
//! assert_eq!(&bytes[..2], b"PK", "a workbook is a ZIP package");
//! ```

use mjx_ooxml_core::{
    Interner, QuoteStyle, RawAttribute, RawDocument, RawElement, RawName, RawNode,
};
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, SML};
use mjx_opc::{OpcError, Package, PartName, Relationship, TargetMode};
use mjx_xml::text::{escape_attribute, escape_text};

use crate::author::ChartData;
use crate::space::ChartSpace;

/// The XML declaration every part of the workbook opens with — the same one Office writes.
const XML_DECLARATION: &[u8] = br#"xml version="1.0" encoding="UTF-8" standalone="yes""#;

/// The content type of the workbook package itself, as the *host* package must register it for
/// `/ppt/embeddings/*.xlsx`.
pub const CONTENT_TYPE_WORKBOOK_PACKAGE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// The content type of `/xl/workbook.xml` inside the workbook package.
const CONTENT_TYPE_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
/// The content type of `/xl/worksheets/sheet1.xml`.
const CONTENT_TYPE_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
/// The content type of `/xl/sharedStrings.xml`.
const CONTENT_TYPE_SHARED_STRINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
/// The content type of `/xl/styles.xml`.
const CONTENT_TYPE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";

/// The package-root relationship type naming the workbook part.
const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
/// The workbook → worksheet relationship type.
const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
/// The workbook → shared-string-table relationship type.
const REL_SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";
/// The workbook → styles relationship type.
const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

/// The default sheet name — the one a chart's synthesized `c:f` formulas (`Sheet1!$A$2:$A$4`) name.
pub const DEFAULT_SHEET_NAME: &str = "Sheet1";

/// One cell of an [`EmbeddedWorkbook`].
///
/// A chart's data is numbers with labels, so three kinds are enough: a number, a string, and the
/// blank that sits in the sheet's top-left corner above the category labels.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkbookCell {
    /// No cell is written at this position.
    Blank,
    /// A numeric cell (`<c r="B2"><v>19.2</v></c>`). A non-finite value has no spelling in
    /// SpreadsheetML and is written as blank.
    Number(f64),
    /// A string cell, written through the shared-string table (`<c r="A2" t="s"><v>0</v></c>`).
    Text(String),
}

impl WorkbookCell {
    /// A text cell from anything string-like.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
}

/// A single-sheet workbook, as a grid of cells — the embedded `.xlsx` that backs a chart.
///
/// Build one with [`for_chart_data`](Self::for_chart_data) (authoring a new chart) or
/// [`for_chart_space`](Self::for_chart_space) (refreshing an existing one after a data edit), then
/// serialize it with [`to_package_bytes`](Self::to_package_bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedWorkbook {
    /// The sheet's name — what the chart's `c:f` formulas qualify their ranges with.
    sheet_name: String,
    /// The grid, row-major, starting at cell `A1`. Rows may be ragged; a missing cell is blank.
    rows: Vec<Vec<WorkbookCell>>,
}

impl Default for EmbeddedWorkbook {
    fn default() -> Self {
        Self::new(DEFAULT_SHEET_NAME)
    }
}

impl EmbeddedWorkbook {
    /// An empty workbook whose one sheet is named `sheet_name`.
    pub fn new(sheet_name: impl Into<String>) -> Self {
        Self {
            sheet_name: sheet_name.into(),
            rows: Vec::new(),
        }
    }

    /// Appends a row of cells, starting at column `A`.
    pub fn push_row(&mut self, cells: Vec<WorkbookCell>) {
        self.rows.push(cells);
    }

    /// The sheet's name.
    #[must_use]
    pub fn sheet_name(&self) -> &str {
        &self.sheet_name
    }

    /// The grid, row-major from `A1`.
    #[must_use]
    pub fn rows(&self) -> &[Vec<WorkbookCell>] {
        &self.rows
    }

    /// The workbook backing a chart being **authored**: a header row of series names, then one row
    /// per category holding its label and each series' value at that position.
    ///
    /// The layout matches the formulas [`ChartData`] synthesizes exactly — column `A` the categories,
    /// column `B` onwards one per series, data starting at row 2 — so Edit Data opens on the cells the
    /// chart's `c:f` references name. For a plot whose category axis is numeric (scatter, bubble) the
    /// `A` column is written as numbers, matching the `c:numRef` the chart uses there.
    #[must_use]
    pub fn for_chart_data(chart: &ChartData) -> Self {
        let mut workbook = Self::default();
        let mut header = vec![WorkbookCell::Blank];
        for series in chart.series_names() {
            header.push(WorkbookCell::text(series));
        }
        workbook.push_row(header);

        let numeric_categories = chart.kind().uses_xy_data();
        let rows = chart.category_count().max(chart.longest_series());
        for index in 0..rows {
            let mut row = vec![match chart.category_label(index) {
                Some(label) if !numeric_categories => WorkbookCell::text(label),
                _ => WorkbookCell::Number(chart.category_number(index)),
            }];
            for values in chart.series_values() {
                row.push(match values.get(index) {
                    Some(&value) => WorkbookCell::Number(value),
                    None => WorkbookCell::Blank,
                });
            }
            workbook.push_row(row);
        }
        workbook
    }

    /// The workbook backing an **existing** chart, read from its parsed part — the refresh a data
    /// edit needs so the workbook never disagrees with the caches that render.
    ///
    /// The categories come from the first series that declares any (they are shared across a plot's
    /// series); each series contributes its name and its values as one column.
    #[must_use]
    pub fn for_chart_space(space: &ChartSpace) -> Self {
        let mut workbook = Self::default();
        let Some(area) = space.plot_area() else {
            return workbook;
        };

        let mut header = vec![WorkbookCell::Blank];
        let mut columns: Vec<Vec<f64>> = Vec::new();
        let mut categories: Vec<WorkbookCell> = Vec::new();
        for series in area.all_series() {
            header.push(match series.name() {
                Some(name) => WorkbookCell::Text(name),
                None => WorkbookCell::Blank,
            });
            let source = series.categories().or_else(|| series.x_data());
            if categories.is_empty() {
                if let Some(source) = source {
                    categories = if source.is_numeric() {
                        source
                            .values()
                            .into_iter()
                            .map(WorkbookCell::Number)
                            .collect()
                    } else {
                        source
                            .labels()
                            .into_iter()
                            .map(WorkbookCell::Text)
                            .collect()
                    };
                }
            }
            columns.push(
                series
                    .values()
                    .map(crate::data::NumericData::values)
                    .or_else(|| series.y_data().map(crate::data::NumericData::values))
                    .unwrap_or_default(),
            );
        }
        workbook.push_row(header);

        let rows = categories
            .len()
            .max(columns.iter().map(Vec::len).max().unwrap_or(0));
        for index in 0..rows {
            let mut row = vec![categories
                .get(index)
                .cloned()
                .unwrap_or(WorkbookCell::Blank)];
            for column in &columns {
                row.push(match column.get(index) {
                    Some(&value) => WorkbookCell::Number(value),
                    None => WorkbookCell::Blank,
                });
            }
            workbook.push_row(row);
        }
        workbook
    }

    /// Serializes the workbook to a complete `.xlsx` package, ready to store as
    /// `/ppt/embeddings/*.xlsx` in the host document.
    ///
    /// # Errors
    /// Returns [`OpcError`] if the packaging layer rejects a part or the ZIP writer fails. Every part
    /// name and content type here is a crate constant, so in practice this cannot fail — it is a
    /// `Result` because the packaging API is fallible, not because there is a failure mode to handle.
    pub fn to_package_bytes(&self) -> Result<Vec<u8>, OpcError> {
        let mut strings = SharedStrings::default();
        let sheet = self.build_worksheet(&mut strings);

        let mut package = Package::empty();
        let workbook_part = part_name("/xl/workbook.xml")?;
        let sheet_part = part_name("/xl/worksheets/sheet1.xml")?;
        let strings_part = part_name("/xl/sharedStrings.xml")?;
        let styles_part = part_name("/xl/styles.xml")?;

        package.insert_part(&workbook_part, CONTENT_TYPE_WORKBOOK, self.build_workbook())?;
        package.insert_part(&sheet_part, CONTENT_TYPE_WORKSHEET, sheet)?;
        package.insert_part(
            &strings_part,
            CONTENT_TYPE_SHARED_STRINGS,
            strings.to_part_bytes(),
        )?;
        package.insert_part(&styles_part, CONTENT_TYPE_STYLES, build_styles())?;

        package.add_relationship(
            None,
            Relationship {
                id: "rId1".to_owned(),
                rel_type: REL_OFFICE_DOCUMENT.to_owned(),
                target: "xl/workbook.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )?;
        for (id, rel_type, target) in [
            ("rId1", REL_WORKSHEET, "worksheets/sheet1.xml"),
            ("rId2", REL_STYLES, "styles.xml"),
            ("rId3", REL_SHARED_STRINGS, "sharedStrings.xml"),
        ] {
            package.add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: id.to_owned(),
                    rel_type: rel_type.to_owned(),
                    target: target.to_owned(),
                    mode: TargetMode::Internal,
                },
            )?;
        }
        package.save()
    }

    /// Builds `/xl/workbook.xml` — one sheet, named and bound to the worksheet relationship.
    fn build_workbook(&self) -> Vec<u8> {
        let mut interner = Interner::new();
        let sheet = {
            let attributes = vec![
                sml_attr(&mut interner, "name", &self.sheet_name),
                sml_attr(&mut interner, "sheetId", "1"),
                relationship_id_attr(&mut interner, "rId1"),
            ];
            sml_element(&mut interner, "sheet", attributes, Vec::new())
        };
        let sheets = sml_element(
            &mut interner,
            "sheets",
            Vec::new(),
            vec![RawNode::Element(sheet)],
        );
        let mut root = sml_element(
            &mut interner,
            "workbook",
            Vec::new(),
            vec![RawNode::Element(sheets)],
        );
        root.attributes = vec![
            default_namespace_declaration(&mut interner, SML.transitional),
            namespace_declaration(
                &mut interner,
                "r",
                SHARED_RELATIONSHIP_REFERENCE.transitional,
            ),
        ];
        serialize(interner, root)
    }

    /// Builds `/xl/worksheets/sheet1.xml` — the grid, interning every string cell into `strings`.
    fn build_worksheet(&self, strings: &mut SharedStrings) -> Vec<u8> {
        let mut interner = Interner::new();
        let mut rows = Vec::new();
        for (index, cells) in self.rows.iter().enumerate() {
            let row_number = index + 1;
            let mut children = Vec::new();
            for (column, cell) in cells.iter().enumerate() {
                let reference = format!("{}{row_number}", column_letters(column));
                match cell {
                    WorkbookCell::Blank => {}
                    WorkbookCell::Number(value) if !value.is_finite() => {}
                    WorkbookCell::Number(value) => {
                        let v = sml_text_leaf(&mut interner, "v", &value.to_string());
                        let attributes = vec![sml_attr(&mut interner, "r", &reference)];
                        children.push(RawNode::Element(sml_element(
                            &mut interner,
                            "c",
                            attributes,
                            vec![RawNode::Element(v)],
                        )));
                    }
                    WorkbookCell::Text(text) => {
                        let index = strings.intern(text);
                        let v = sml_text_leaf(&mut interner, "v", &index.to_string());
                        let attributes = vec![
                            sml_attr(&mut interner, "r", &reference),
                            sml_attr(&mut interner, "t", "s"),
                        ];
                        children.push(RawNode::Element(sml_element(
                            &mut interner,
                            "c",
                            attributes,
                            vec![RawNode::Element(v)],
                        )));
                    }
                }
            }
            if children.is_empty() {
                continue;
            }
            let attributes = vec![sml_attr(&mut interner, "r", &row_number.to_string())];
            rows.push(RawNode::Element(sml_element(
                &mut interner,
                "row",
                attributes,
                children,
            )));
        }
        let sheet_data = sml_element(&mut interner, "sheetData", Vec::new(), rows);
        let mut children = Vec::new();
        if let Some(dimension) = self.dimension() {
            let attributes = vec![sml_attr(&mut interner, "ref", &dimension)];
            children.push(RawNode::Element(sml_element(
                &mut interner,
                "dimension",
                attributes,
                Vec::new(),
            )));
        }
        children.push(RawNode::Element(sheet_data));
        let mut root = sml_element(&mut interner, "worksheet", Vec::new(), children);
        root.attributes = vec![default_namespace_declaration(
            &mut interner,
            SML.transitional,
        )];
        serialize(interner, root)
    }

    /// The sheet's used range (`A1:C4`), or `None` when nothing is written.
    fn dimension(&self) -> Option<String> {
        let width = self.rows.iter().map(Vec::len).max().unwrap_or(0);
        if width == 0 || self.rows.is_empty() {
            return None;
        }
        Some(format!(
            "A1:{}{}",
            column_letters(width - 1),
            self.rows.len()
        ))
    }
}

/// The shared-string table being accumulated while the sheet is written: unique strings in
/// first-use order, which is the order `xl/sharedStrings.xml` lists them in.
#[derive(Debug, Default)]
struct SharedStrings {
    entries: Vec<String>,
}

impl SharedStrings {
    /// The index of `text` in the table, adding it if new.
    fn intern(&mut self, text: &str) -> usize {
        if let Some(index) = self.entries.iter().position(|entry| entry == text) {
            return index;
        }
        self.entries.push(text.to_owned());
        self.entries.len() - 1
    }

    /// Builds `/xl/sharedStrings.xml`.
    fn to_part_bytes(&self) -> Vec<u8> {
        let mut interner = Interner::new();
        let mut children = Vec::new();
        for entry in &self.entries {
            let t = sml_text_leaf(&mut interner, "t", entry);
            children.push(RawNode::Element(sml_element(
                &mut interner,
                "si",
                Vec::new(),
                vec![RawNode::Element(t)],
            )));
        }
        let count = self.entries.len().to_string();
        let attributes = vec![
            default_namespace_declaration(&mut interner, SML.transitional),
            sml_attr(&mut interner, "count", &count),
            sml_attr(&mut interner, "uniqueCount", &count),
        ];
        let mut root = sml_element(&mut interner, "sst", Vec::new(), children);
        root.attributes = attributes;
        serialize(interner, root)
    }
}

/// Builds `/xl/styles.xml` — the skeleton a workbook needs to open: one font, the two fills Excel
/// always writes, one border, and one cell format referencing them.
fn build_styles() -> Vec<u8> {
    let mut interner = Interner::new();

    let font = {
        let size = sml_val_leaf(&mut interner, "sz", "11");
        let name = sml_val_leaf(&mut interner, "name", "Calibri");
        let family = sml_val_leaf(&mut interner, "family", "2");
        sml_element(
            &mut interner,
            "font",
            Vec::new(),
            vec![
                RawNode::Element(size),
                RawNode::Element(name),
                RawNode::Element(family),
            ],
        )
    };
    let fonts = counted(&mut interner, "fonts", vec![RawNode::Element(font)]);

    let mut fill_nodes = Vec::new();
    for pattern in ["none", "gray125"] {
        let attributes = vec![sml_attr(&mut interner, "patternType", pattern)];
        let pattern_fill = sml_element(&mut interner, "patternFill", attributes, Vec::new());
        fill_nodes.push(RawNode::Element(sml_element(
            &mut interner,
            "fill",
            Vec::new(),
            vec![RawNode::Element(pattern_fill)],
        )));
    }
    let fills = counted(&mut interner, "fills", fill_nodes);

    let border_sides: Vec<RawNode> = ["left", "right", "top", "bottom", "diagonal"]
        .into_iter()
        .map(|side| RawNode::Element(sml_element(&mut interner, side, Vec::new(), Vec::new())))
        .collect();
    let border = sml_element(&mut interner, "border", Vec::new(), border_sides);
    let borders = counted(&mut interner, "borders", vec![RawNode::Element(border)]);

    let base_format = |interner: &mut Interner, with_style: bool| {
        let mut attributes = vec![
            sml_attr(interner, "numFmtId", "0"),
            sml_attr(interner, "fontId", "0"),
            sml_attr(interner, "fillId", "0"),
            sml_attr(interner, "borderId", "0"),
        ];
        if with_style {
            attributes.push(sml_attr(interner, "xfId", "0"));
        }
        RawNode::Element(sml_element(interner, "xf", attributes, Vec::new()))
    };
    let style_format = base_format(&mut interner, false);
    let cell_format = base_format(&mut interner, true);
    let cell_style_xfs = counted(&mut interner, "cellStyleXfs", vec![style_format]);
    let cell_xfs = counted(&mut interner, "cellXfs", vec![cell_format]);

    let cell_style = {
        let attributes = vec![
            sml_attr(&mut interner, "name", "Normal"),
            sml_attr(&mut interner, "xfId", "0"),
            sml_attr(&mut interner, "builtinId", "0"),
        ];
        RawNode::Element(sml_element(
            &mut interner,
            "cellStyle",
            attributes,
            Vec::new(),
        ))
    };
    let cell_styles = counted(&mut interner, "cellStyles", vec![cell_style]);

    let children = vec![
        RawNode::Element(fonts),
        RawNode::Element(fills),
        RawNode::Element(borders),
        RawNode::Element(cell_style_xfs),
        RawNode::Element(cell_xfs),
        RawNode::Element(cell_styles),
    ];
    let mut root = sml_element(&mut interner, "styleSheet", Vec::new(), children);
    root.attributes = vec![default_namespace_declaration(
        &mut interner,
        SML.transitional,
    )];
    serialize(interner, root)
}

/// Builds a `<local count="N">…</local>` wrapper — the shape of every collection in `styles.xml`.
fn counted(interner: &mut Interner, local: &str, children: Vec<RawNode>) -> RawElement {
    let attributes = vec![sml_attr(interner, "count", &children.len().to_string())];
    sml_element(interner, local, attributes, children)
}

/// Wraps a built root element in a document and serializes it with the fidelity writer.
fn serialize(interner: Interner, root: RawElement) -> Vec<u8> {
    let document = RawDocument::new(
        interner,
        false,
        vec![
            RawNode::Declaration(XML_DECLARATION.into()),
            RawNode::Text(Box::from(&b"\n"[..])),
        ],
        root,
        Vec::new(),
    );
    mjx_xml::fidelity::serialize_to_vec(&document)
}

/// Parses a crate-constant part name. The names are literals in this module, so a failure is a bug
/// here rather than bad input — it surfaces as a packaging error rather than a panic.
fn part_name(name: &str) -> Result<PartName, OpcError> {
    PartName::new(name)
        .map_err(|_| OpcError::Malformed(format!("invalid workbook part name: {name}")))
}

/// Builds an unprefixed SpreadsheetML name — the workbook binds `sml` as its *default* namespace,
/// exactly as Office does, so its elements carry no prefix.
fn sml_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: None,
        local: interner.intern(local),
        namespace: Some(interner.intern(SML.transitional)),
    }
}

/// Builds an unprefixed SpreadsheetML element (self-closing when it has no children).
fn sml_element(
    interner: &mut Interner,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    let empty = children.is_empty();
    RawElement::new(sml_name(interner, local), attributes, children, empty)
}

/// Builds a text-bearing leaf (`<v>19.2</v>`, `<t>North</t>`).
fn sml_text_leaf(interner: &mut Interner, local: &str, text: &str) -> RawElement {
    let escaped = escape_text(text);
    let children = if escaped.is_empty() {
        Vec::new()
    } else {
        vec![RawNode::Text(escaped.as_bytes().into())]
    };
    sml_element(interner, local, Vec::new(), children)
}

/// Builds a `<local val="value"/>` leaf — the shape of the style parts' scalar children.
fn sml_val_leaf(interner: &mut Interner, local: &str, value: &str) -> RawElement {
    let attribute = sml_attr(interner, "val", value);
    sml_element(interner, local, vec![attribute], Vec::new())
}

/// Builds an unprefixed, double-quoted attribute, escaping `value`.
fn sml_attr(interner: &mut Interner, local: &str, value: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: None,
            local: interner.intern(local),
            namespace: None,
        },
        value: escape_attribute(value).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// Builds an `r:id="rId1"` attribute — the only prefixed attribute the workbook writes.
fn relationship_id_attr(interner: &mut Interner, id: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(interner.intern("r")),
            local: interner.intern("id"),
            namespace: Some(interner.intern(SHARED_RELATIONSHIP_REFERENCE.transitional)),
        },
        value: escape_attribute(id).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// Builds an `xmlns="uri"` default-namespace declaration.
fn default_namespace_declaration(interner: &mut Interner, uri: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: None,
            local: interner.intern("xmlns"),
            namespace: None,
        },
        value: escape_attribute(uri).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// Builds an `xmlns:prefix="uri"` declaration.
fn namespace_declaration(interner: &mut Interner, prefix: &str, uri: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(interner.intern("xmlns")),
            local: interner.intern(prefix),
            namespace: None,
        },
        value: escape_attribute(uri).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// The spreadsheet column letters for a 0-based column index (`0` → `A`, `25` → `Z`, `26` → `AA`).
pub(crate) fn column_letters(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    letters.reverse();
    // Every pushed byte is an ASCII uppercase letter, so this is always valid UTF-8.
    String::from_utf8(letters).unwrap_or_default()
}
