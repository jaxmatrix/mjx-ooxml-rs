//! The plots and their series — `c:barChart` … `c:bubbleChart`, and `c:ser`.
//!
//! A plot is one chart *type* inside the plot area; a plot holds one or more **series** (`c:ser`),
//! and each series binds a name (`c:tx`), the category labels every series shares (`c:cat`) and its
//! own values (`c:val`) — the data layer of [`CategoryData`] and [`NumericData`]. All sixteen plot
//! types `CT_PlotArea`
//! admits are modeled here, on one spine: they differ only in their element name and in the
//! type-specific scalars each declares, so they share one content enum and one series API.
//!
//! ```xml
//! <c:barChart>
//!   <c:barDir val="col"/>
//!   <c:grouping val="clustered"/>
//!   <c:ser>
//!     <c:idx val="0"/><c:order val="0"/>
//!     <c:tx>…</c:tx> <c:cat>…</c:cat> <c:val>…</c:val>
//!   </c:ser>
//!   <c:axId val="…"/> <c:axId val="…"/>
//! </c:barChart>
//! ```
//!
//! The single-attribute scalars a plot declares — `c:barDir`, `c:grouping`, `c:varyColors`,
//! `c:axId`, and a series' `c:idx`/`c:order` — are kept in the `Raw` bucket and read through small
//! accessors, so they round-trip byte-for-byte while still being readable. A series' shape
//! properties (`c:spPr`) are typed, because they are what decides what a series looks like.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use mjx_dml::{Fill, FillSpec, LineProperties, LineSpec};
use mjx_ooxml_core::{FromXml, ToXml};
use mjx_ooxml_types::support::on_off;

use crate::build::{chart_element, fidelity_element_impls, is_dml, raw_child_attr};
use crate::data::{CategoryData, NumericData, SeriesText};

/// Which way a bar plot's bars run (`c:barDir@val`, `ST_BarDir`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarDirection {
    /// Vertical bars — a column chart (wire `col`).
    Column,
    /// Horizontal bars (wire `bar`).
    Bar,
}

impl BarDirection {
    /// Maps the wire token (`col`/`bar`) to a direction.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "col" => Some(Self::Column),
            "bar" => Some(Self::Bar),
            _ => None,
        }
    }
}

/// How a bar plot's series are combined (`c:grouping@val`, `ST_BarGrouping`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarGrouping {
    /// Bars for each category stand side by side (wire `clustered`).
    Clustered,
    /// Series stacked into one bar per category (wire `stacked`).
    Stacked,
    /// Stacked and normalized to 100% (wire `percentStacked`).
    PercentStacked,
    /// Series overlaid on a shared baseline (wire `standard`).
    Standard,
}

impl BarGrouping {
    /// Maps the wire token to a grouping.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "clustered" => Some(Self::Clustered),
            "stacked" => Some(Self::Stacked),
            "percentStacked" => Some(Self::PercentStacked),
            "standard" => Some(Self::Standard),
            _ => None,
        }
    }
}

/// How the series of a line, area or surface plot are combined (`c:grouping@val`, `ST_Grouping`).
/// The bar family has a wider set of its own — see [`BarGrouping`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesGrouping {
    /// Series drawn independently against a shared baseline (wire `standard`).
    Standard,
    /// Series stacked on top of one another (wire `stacked`).
    Stacked,
    /// Stacked and normalized to 100% (wire `percentStacked`).
    PercentStacked,
}

impl SeriesGrouping {
    /// Maps the wire token to a grouping.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "standard" => Some(Self::Standard),
            "stacked" => Some(Self::Stacked),
            "percentStacked" => Some(Self::PercentStacked),
            _ => None,
        }
    }
}

/// How a scatter plot joins and marks its points (`c:scatterStyle@val`, `ST_ScatterStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterStyle {
    /// Neither line nor marker (wire `none`).
    None,
    /// A straight line, no markers (wire `line`).
    Line,
    /// A straight line with markers (wire `lineMarker`).
    LineWithMarkers,
    /// Markers only (wire `marker`).
    Markers,
    /// A smoothed line, no markers (wire `smooth`).
    SmoothLine,
    /// A smoothed line with markers (wire `smoothMarker`).
    SmoothLineWithMarkers,
}

impl ScatterStyle {
    /// Maps the wire token to a style.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "none" => Some(Self::None),
            "line" => Some(Self::Line),
            "lineMarker" => Some(Self::LineWithMarkers),
            "marker" => Some(Self::Markers),
            "smooth" => Some(Self::SmoothLine),
            "smoothMarker" => Some(Self::SmoothLineWithMarkers),
            _ => None,
        }
    }
}

/// How a radar plot draws its series (`c:radarStyle@val`, `ST_RadarStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadarStyle {
    /// Lines only (wire `standard`).
    Standard,
    /// Lines with markers at each spoke (wire `marker`).
    Markers,
    /// Filled areas (wire `filled`).
    Filled,
}

impl RadarStyle {
    /// Maps the wire token to a style.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "standard" => Some(Self::Standard),
            "marker" => Some(Self::Markers),
            "filled" => Some(Self::Filled),
            _ => None,
        }
    }
}

/// What a pie-of-pie plot breaks its small slices out into (`c:ofPieType@val`, `ST_OfPieType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfPieType {
    /// A second pie (wire `pie`).
    Pie,
    /// A stacked bar (wire `bar`).
    Bar,
}

impl OfPieType {
    /// Maps the wire token to a type.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "pie" => Some(Self::Pie),
            "bar" => Some(Self::Bar),
            _ => None,
        }
    }
}

/// The chart type a plot area draws — one variant per modeled plot. A plot area may draw more than
/// one (a combo chart), so a chart is described by a *set* of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartKind {
    /// A bar/column plot (`c:barChart`).
    Bar,
    /// A three-dimensional bar/column plot (`c:bar3DChart`).
    Bar3D,
    /// A line plot (`c:lineChart`).
    Line,
    /// A three-dimensional line plot (`c:line3DChart`).
    Line3D,
    /// A pie plot (`c:pieChart`).
    Pie,
    /// A three-dimensional pie plot (`c:pie3DChart`).
    Pie3D,
    /// A pie-of-pie or bar-of-pie plot (`c:ofPieChart`) — a pie whose small slices are broken out
    /// into a second plot.
    OfPie,
    /// An area plot (`c:areaChart`).
    Area,
    /// A three-dimensional area plot (`c:area3DChart`).
    Area3D,
    /// An X/Y scatter plot (`c:scatterChart`).
    Scatter,
    /// A doughnut plot (`c:doughnutChart`).
    Doughnut,
    /// A radar plot (`c:radarChart`) — one spoke per category.
    Radar,
    /// A bubble plot (`c:bubbleChart`) — X/Y points sized by a third value.
    Bubble,
    /// A high-low-close style stock plot (`c:stockChart`).
    Stock,
    /// A surface plot seen from above (`c:surfaceChart`) — a contour map.
    Surface,
    /// A three-dimensional surface plot (`c:surface3DChart`).
    Surface3D,
}

impl ChartKind {
    /// The element name of the plot this kind describes, without its `c:` prefix (`Bar` →
    /// `barChart`) — the exact wire spelling, which is also what an authored plot is written as.
    #[must_use]
    pub fn element_local_name(self) -> &'static str {
        match self {
            Self::Bar => "barChart",
            Self::Bar3D => "bar3DChart",
            Self::Line => "lineChart",
            Self::Line3D => "line3DChart",
            Self::Pie => "pieChart",
            Self::Pie3D => "pie3DChart",
            Self::OfPie => "ofPieChart",
            Self::Area => "areaChart",
            Self::Area3D => "area3DChart",
            Self::Scatter => "scatterChart",
            Self::Doughnut => "doughnutChart",
            Self::Radar => "radarChart",
            Self::Bubble => "bubbleChart",
            Self::Stock => "stockChart",
            Self::Surface => "surfaceChart",
            Self::Surface3D => "surface3DChart",
        }
    }

    /// Whether a plot of this kind carries its data as X/Y pairs (`c:xVal`/`c:yVal`) rather than as
    /// shared categories and values (`c:cat`/`c:val`) — scatter and bubble.
    #[must_use]
    pub fn uses_xy_data(self) -> bool {
        matches!(self, Self::Scatter | Self::Bubble)
    }

    /// How many axes a plot of this kind names with `c:axId` — two for a flat plot, three for a
    /// depth-bearing one, none for the pie family, which draws against no axis at all.
    #[must_use]
    pub fn axis_count(self) -> usize {
        match self {
            Self::Pie | Self::Pie3D | Self::OfPie | Self::Doughnut => 0,
            Self::Line3D | Self::Surface | Self::Surface3D => 3,
            _ => 2,
        }
    }
}

/// The `a:spPr` children a fill must precede, per `CT_ShapeProperties`'s content order — so a fill
/// inserted into a series' shape properties lands after any transform or geometry and before the
/// outline, the effects and the extensions.
const AFTER_FILL_LOCALS: [&str; 6] = ["ln", "effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

/// The `a:spPr` children an outline (`a:ln`) must precede — [`AFTER_FILL_LOCALS`] without the
/// leading `ln`, so a new outline lands after any fill and before the effects.
const AFTER_LINE_LOCALS: [&str; 5] = ["effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

/// `c:spPr` (`a:CT_ShapeProperties`) — a series' fill and outline: what decides what it looks like.
///
/// The element is kept opaque and re-emitted verbatim, exactly as `mjx-pptx` treats a shape's
/// `p:spPr`; the accessors read (and rewrite) the one fill and the one outline it may declare,
/// leaving its geometry, effects, 3-D and extensions untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesShapeProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(SeriesShapeProperties);

impl SeriesShapeProperties {
    /// A fresh, empty `c:spPr` — what a series gets when it is given a fill and had no properties.
    pub(crate) fn new(interner: &mut Interner) -> Self {
        let element = chart_element(interner, "spPr", Vec::new(), Vec::new());
        Self {
            name: element.name,
            attributes: element.attributes,
            children: element.children,
            empty: element.empty,
        }
    }

    /// The series' fill (`a:solidFill`, `a:gradFill`, …), or `None` when it declares none.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<FillSpec> {
        let index = self.fill_index(interner)?;
        let RawNode::Element(element) = &self.children[index] else {
            return None;
        };
        Fill::from_xml(element, interner)
            .ok()
            .map(|fill| fill.spec(interner))
    }

    /// Sets the series' fill, replacing an existing one in place or inserting a new one in its
    /// schema position.
    pub fn set_fill(&mut self, interner: &mut Interner, fill: &FillSpec) {
        let node = RawNode::Element(fill.to_fill(interner).to_xml(interner));
        match self.fill_index(interner) {
            Some(index) => self.children[index] = node,
            None => {
                let at = self.insert_index(interner, &AFTER_FILL_LOCALS);
                self.children.insert(at, node);
                self.empty = false;
            }
        }
    }

    /// The series' outline (`a:ln`), or `None` when it declares none.
    #[must_use]
    pub fn line(&self, interner: &Interner) -> Option<LineSpec> {
        let index = self.line_index(interner)?;
        let RawNode::Element(element) = &self.children[index] else {
            return None;
        };
        LineProperties::from_xml(element, interner)
            .ok()
            .map(|line| line.spec(interner))
    }

    /// Sets the series' outline, replacing an existing one in place or inserting a new one in its
    /// schema position.
    pub fn set_line(&mut self, interner: &mut Interner, line: &LineSpec) {
        let node = RawNode::Element(line.to_line(interner).to_xml(interner));
        match self.line_index(interner) {
            Some(index) => self.children[index] = node,
            None => {
                let at = self.insert_index(interner, &AFTER_LINE_LOCALS);
                self.children.insert(at, node);
                self.empty = false;
            }
        }
    }

    /// The index of the fill child, if any.
    fn fill_index(&self, interner: &Interner) -> Option<usize> {
        self.children
            .iter()
            .position(|node| dml_local(node, interner).is_some_and(Fill::is_fill_local))
    }

    /// The index of the outline child, if any.
    fn line_index(&self, interner: &Interner) -> Option<usize> {
        self.children
            .iter()
            .position(|node| dml_local(node, interner) == Some("ln"))
    }

    /// Where a new child belongs: before the first child in `after`, else at the end.
    fn insert_index(&self, interner: &Interner, after: &[&str]) -> usize {
        self.children
            .iter()
            .position(|node| dml_local(node, interner).is_some_and(|local| after.contains(&local)))
            .unwrap_or(self.children.len())
    }
}

/// The local name of a DrawingML element node, or `None` for anything else.
fn dml_local<'a>(node: &RawNode, interner: &'a Interner) -> Option<&'a str> {
    match node {
        RawNode::Element(element) if is_dml(&element.name, interner) => {
            Some(interner.resolve(element.name.local))
        }
        _ => None,
    }
}

/// One ordered child of a [`Series`]: its name, its category/value data (bar/line/pie/area/doughnut)
/// or its X/Y data (scatter), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesContent {
    /// The series name (`c:tx`).
    Text(SeriesText),
    /// The shared category labels (`c:cat`) — category/value plots.
    Categories(CategoryData),
    /// The series' values (`c:val`) — category/value plots.
    Values(NumericData),
    /// The series' X data (`c:xVal`) — scatter plots.
    XValues(CategoryData),
    /// The series' Y data (`c:yVal`) — scatter plots.
    YValues(NumericData),
    /// The series' bubble sizes (`c:bubbleSize`) — bubble plots.
    BubbleSizes(NumericData),
    /// The series' shape properties (`c:spPr`) — the fill and outline that decide what it looks like.
    ShapeProperties(SeriesShapeProperties),
    /// Any other child — `c:idx`, `c:order`, `c:marker`, whitespace, unknown — preserved verbatim.
    Raw(RawNode),
}

/// `c:ser` — one series of a plot. Every plot type shares this element and its `c:tx`/`c:idx`/
/// `c:order` header; a category/value plot (bar, line, pie, area, doughnut) fills `c:cat`/`c:val`,
/// while a scatter plot fills `c:xVal`/`c:yVal` instead.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct Series {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tx", variant = Text, ty = SeriesText),
        child(local = "cat", variant = Categories, ty = CategoryData),
        child(local = "val", variant = Values, ty = NumericData),
        child(local = "xVal", variant = XValues, ty = CategoryData),
        child(local = "yVal", variant = YValues, ty = NumericData),
        child(local = "bubbleSize", variant = BubbleSizes, ty = NumericData),
        child(local = "spPr", variant = ShapeProperties, ty = SeriesShapeProperties)
    )]
    content: Vec<SeriesContent>,
}

impl Series {
    /// The series' 0-based index among its plot's series (`c:idx@val`).
    #[must_use]
    pub fn index(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "idx")
    }

    /// The series' draw order (`c:order@val`) — which series paints over which.
    #[must_use]
    pub fn order(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "order")
    }

    /// The series name (`c:tx`), or `None` if it declares none.
    #[must_use]
    pub fn name_source(&self) -> Option<&SeriesText> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::Text(text) => Some(text),
            _ => None,
        })
    }

    /// The series' resolved name, or `None` when it has no `c:tx` (or the name is unmodeled).
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.name_source().and_then(SeriesText::text)
    }

    /// The series' category labels (`c:cat`), or `None` if it declares none.
    #[must_use]
    pub fn categories(&self) -> Option<&CategoryData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::Categories(categories) => Some(categories),
            _ => None,
        })
    }

    /// The series' values (`c:val`), or `None` if it declares none.
    #[must_use]
    pub fn values(&self) -> Option<&NumericData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::Values(values) => Some(values),
            _ => None,
        })
    }

    /// The series' X data (`c:xVal`), for a scatter plot — `None` for a category/value plot.
    #[must_use]
    pub fn x_data(&self) -> Option<&CategoryData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::XValues(x) => Some(x),
            _ => None,
        })
    }

    /// The series' Y data (`c:yVal`), for a scatter plot — `None` for a category/value plot.
    #[must_use]
    pub fn y_data(&self) -> Option<&NumericData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::YValues(y) => Some(y),
            _ => None,
        })
    }

    /// The series' bubble sizes (`c:bubbleSize`), for a bubble plot — `None` for any other kind.
    #[must_use]
    pub fn bubble_sizes(&self) -> Option<&NumericData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::BubbleSizes(sizes) => Some(sizes),
            _ => None,
        })
    }

    /// The series' shape properties (`c:spPr`), or `None` if it declares none.
    #[must_use]
    pub fn shape_properties(&self) -> Option<&SeriesShapeProperties> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::ShapeProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The series' fill — what colour it is drawn in — or `None` when it declares none and inherits
    /// its colour from the chart style.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<FillSpec> {
        self.shape_properties()
            .and_then(|properties| properties.fill(interner))
    }

    /// Sets the series' fill, creating its `c:spPr` if it had none.
    pub fn set_fill(&mut self, interner: &mut Interner, fill: &FillSpec) {
        self.shape_properties_mut(interner).set_fill(interner, fill);
    }

    /// The series' outline (`a:ln`), or `None` when it declares none.
    #[must_use]
    pub fn line(&self, interner: &Interner) -> Option<LineSpec> {
        self.shape_properties()
            .and_then(|properties| properties.line(interner))
    }

    /// Sets the series' outline, creating its `c:spPr` if it had none.
    pub fn set_line(&mut self, interner: &mut Interner, line: &LineSpec) {
        self.shape_properties_mut(interner).set_line(interner, line);
    }

    /// The series' shape properties, creating an empty `c:spPr` in its schema position (after the
    /// `c:tx` that may precede it, before the data sources) if it has none.
    fn shape_properties_mut(&mut self, interner: &mut Interner) -> &mut SeriesShapeProperties {
        if let Some(index) = self
            .content
            .iter()
            .position(|item| matches!(item, SeriesContent::ShapeProperties(_)))
        {
            let SeriesContent::ShapeProperties(properties) = &mut self.content[index] else {
                unreachable!("the index was just found by matching this variant")
            };
            return properties;
        }
        let at = self
            .content
            .iter()
            .position(|item| {
                matches!(
                    item,
                    SeriesContent::Categories(_)
                        | SeriesContent::Values(_)
                        | SeriesContent::XValues(_)
                        | SeriesContent::YValues(_)
                        | SeriesContent::BubbleSizes(_)
                )
            })
            .unwrap_or(self.content.len());
        self.content.insert(
            at,
            SeriesContent::ShapeProperties(SeriesShapeProperties::new(interner)),
        );
        self.empty = false;
        let SeriesContent::ShapeProperties(properties) = &mut self.content[at] else {
            unreachable!("the element inserted at `at` was a ShapeProperties")
        };
        properties
    }

    /// Rewrites the series' numeric values — its `c:val` (category/value plots) or, failing that,
    /// its `c:yVal` (scatter, bubble), through whichever source it names: a `c:numRef`'s cache or a
    /// `c:numLit`. Returns `false` (unchanged) when the series has no numeric slot to rewrite.
    ///
    /// This rewrites what **renders**. A chart's embedded workbook is a separate part and is brought
    /// back in line by its owner — `mjx_pptx::Presentation::set_chart_series_values` refreshes it in
    /// the same call, so the two never disagree.
    pub fn set_values(&mut self, interner: &mut Interner, values: &[f64]) -> bool {
        for item in &mut self.content {
            match item {
                SeriesContent::Values(data) | SeriesContent::YValues(data) => {
                    return data.set_values(interner, values);
                }
                _ => {}
            }
        }
        false
    }

    /// Rewrites the series' category labels — its `c:cat` (a `c:strRef`'s cache or a `c:strLit`),
    /// or its `c:xVal` for an X/Y plot. Returns `false` (unchanged) when the series' category source
    /// is numeric or multi-level and so has no string labels to rewrite.
    pub fn set_categories(&mut self, interner: &mut Interner, labels: &[&str]) -> bool {
        for item in &mut self.content {
            if let SeriesContent::Categories(data) | SeriesContent::XValues(data) = item {
                return data.set_labels(interner, labels);
            }
        }
        false
    }

    /// Reads the `@val` of a raw scalar child (`c:idx`, `c:order`) as a `u32`.
    fn raw_val(&self, interner: &Interner, local: &str) -> Option<u32> {
        let raw = self.content.iter().filter_map(|item| match item {
            SeriesContent::Raw(node) => Some(node),
            _ => None,
        });
        raw_child_attr(raw, interner, local, "val").and_then(|s| s.trim().parse().ok())
    }
}

/// One ordered child of a plot (`c:barChart`, `c:lineChart`, …): a typed series (`c:ser`), or an
/// opaque node (`c:barDir`, `c:grouping`, `c:axId`, `c:firstSliceAng`, whitespace, unknown).
///
/// Every plot type shares this shape — a run of series interleaved with the type-specific scalars
/// and axes this tier does not model — so it needs one content enum, not one per type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlotContent {
    /// A series (`c:ser`).
    Series(Series),
    /// Any other child — a type-specific scalar, an axis id, whitespace, unknown — kept verbatim.
    Raw(RawNode),
}

/// Generates the series accessors every plot type shares (`series`, `series_at`, `series_count`) and
/// its `kind`, over a `content: Vec<PlotContent>` field. The plot types differ only in their element
/// name (preserved in `name`) and the type-specific scalars they bucket into `Raw`, so their series
/// API is identical and lives here rather than being written six times.
macro_rules! series_plot_impls {
    ($ty:ty, $kind:expr) => {
        impl $ty {
            /// The plot's series, in order.
            pub fn series(&self) -> impl Iterator<Item = &Series> {
                self.content.iter().filter_map(|item| match item {
                    PlotContent::Series(series) => Some(series),
                    PlotContent::Raw(_) => None,
                })
            }

            /// The plot's series, in order, mutably — for rewriting a series' cached data.
            pub fn series_mut(&mut self) -> impl Iterator<Item = &mut Series> {
                self.content.iter_mut().filter_map(|item| match item {
                    PlotContent::Series(series) => Some(series),
                    PlotContent::Raw(_) => None,
                })
            }

            /// The `n`-th series, or `None` if the plot has fewer.
            #[must_use]
            pub fn series_at(&self, n: usize) -> Option<&Series> {
                self.series().nth(n)
            }

            /// How many series the plot draws.
            #[must_use]
            pub fn series_count(&self) -> usize {
                self.series().count()
            }

            /// The kind of plot this is.
            #[must_use]
            pub fn kind(&self) -> ChartKind {
                $kind
            }

            /// Whether each data point is given its own colour (`c:varyColors`) — the chart-level
            /// switch that decides whether a plot's points share the series colour or vary. `None`
            /// when the plot does not declare it.
            #[must_use]
            pub fn vary_colors(&self, interner: &Interner) -> Option<bool> {
                self.raw_val(interner, "varyColors")
                    .and_then(::mjx_ooxml_types::support::on_off::from_wire)
            }

            /// The ids of the axes this plot draws against (`c:axId`), in document order — two for a
            /// flat plot, three for a depth-bearing one, none for the pie family. They match the
            /// `axis_id` of the [`Axis`](crate::Axis) entries the plot area declares.
            #[must_use]
            pub fn axis_ids(&self, interner: &Interner) -> Vec<u32> {
                self.content
                    .iter()
                    .filter_map(|item| match item {
                        PlotContent::Raw(::mjx_ooxml_core::RawNode::Element(element)) => {
                            (crate::build::is_chart(&element.name, interner)
                                && interner.resolve(element.name.local) == "axId")
                                .then(|| {
                                    crate::build::attr_str(&element.attributes, interner, "val")
                                })
                                .flatten()
                        }
                        _ => None,
                    })
                    .filter_map(|value| value.trim().parse().ok())
                    .collect()
            }

            /// Reads the `@val` of a raw scalar child of this plot (`c:grouping`, `c:holeSize`, …).
            fn raw_val(&self, interner: &Interner, local: &str) -> Option<&str> {
                let raw = self.content.iter().filter_map(|item| match item {
                    PlotContent::Raw(node) => Some(node),
                    PlotContent::Series(_) => None,
                });
                raw_child_attr(raw, interner, local, "val")
            }
        }
    };
}

/// `c:barChart` (`CT_BarChart`) — a bar/column plot and its series.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct BarChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<PlotContent>,
}

series_plot_impls!(BarChart, ChartKind::Bar);

impl BarChart {
    /// Which way the bars run (`c:barDir`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn direction(&self, interner: &Interner) -> Option<BarDirection> {
        self.raw_val(interner, "barDir")
            .and_then(BarDirection::from_wire)
    }

    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<BarGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(BarGrouping::from_wire)
    }

    /// The space between bar clusters as a percentage of bar width (`c:gapWidth`).
    #[must_use]
    pub fn gap_width(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "gapWidth")
            .and_then(|value| value.trim().parse().ok())
    }

    /// How far the bars of a cluster overlap, as a percentage (`c:overlap`); negative values push
    /// them apart.
    #[must_use]
    pub fn overlap(&self, interner: &Interner) -> Option<i32> {
        self.raw_val(interner, "overlap")
            .and_then(|value| value.trim().parse().ok())
    }
}

/// `c:lineChart` (`CT_LineChart`) — a line plot and its series.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct LineChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<PlotContent>,
}

series_plot_impls!(LineChart, ChartKind::Line);

impl LineChart {
    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<SeriesGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(SeriesGrouping::from_wire)
    }
}

/// `c:pieChart` (`CT_PieChart`) — a pie plot and its series.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct PieChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<PlotContent>,
}

series_plot_impls!(PieChart, ChartKind::Pie);

/// `c:areaChart` (`CT_AreaChart`) — an area plot and its series.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct AreaChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<PlotContent>,
}

series_plot_impls!(AreaChart, ChartKind::Area);

impl AreaChart {
    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<SeriesGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(SeriesGrouping::from_wire)
    }
}

/// `c:scatterChart` (`CT_ScatterChart`) — an X/Y scatter plot and its series. Its series carry
/// `c:xVal`/`c:yVal` rather than `c:cat`/`c:val` (see [`Series::x_data`]/[`Series::y_data`]).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct ScatterChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<PlotContent>,
}

series_plot_impls!(ScatterChart, ChartKind::Scatter);

impl ScatterChart {
    /// How the points are joined and marked (`c:scatterStyle`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn scatter_style(&self, interner: &Interner) -> Option<ScatterStyle> {
        self.raw_val(interner, "scatterStyle")
            .and_then(ScatterStyle::from_wire)
    }
}

/// `c:doughnutChart` (`CT_DoughnutChart`) — a doughnut plot and its series (like a pie with a hole).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct DoughnutChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<PlotContent>,
}

series_plot_impls!(DoughnutChart, ChartKind::Doughnut);

impl DoughnutChart {
    /// The size of the hole, as a percentage of the doughnut's diameter (`c:holeSize`).
    #[must_use]
    pub fn hole_size(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "holeSize")
            .and_then(|value| value.trim().parse().ok())
    }

    /// The angle of the first slice, in degrees clockwise from twelve o'clock (`c:firstSliceAng`).
    #[must_use]
    pub fn first_slice_angle(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "firstSliceAng")
            .and_then(|value| value.trim().parse().ok())
    }
}

/// Declares one more plot type on the shared spine: a struct whose only modeled child is `c:ser`,
/// plus the series/kind/`varyColors`/`axId` API every plot has. The ten types below differ from the
/// six above only in their element name and their type-specific scalars, which stay in the `Raw`
/// bucket and are read by the `impl` blocks that follow.
macro_rules! declare_plot {
    ($(#[$meta:meta])* $ty:ident, $kind:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
        #[xml(namespace = DML_CHART)]
        pub struct $ty {
            name: RawName,
            attributes: Vec<RawAttribute>,
            empty: bool,
            #[xml(children, child(local = "ser", variant = Series, ty = Series))]
            content: Vec<PlotContent>,
        }

        series_plot_impls!($ty, $kind);
    };
}

declare_plot!(
    /// `c:bar3DChart` (`CT_Bar3DChart`) — a three-dimensional bar/column plot and its series.
    Bar3DChart,
    ChartKind::Bar3D
);

impl Bar3DChart {
    /// Which way the bars run (`c:barDir`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn direction(&self, interner: &Interner) -> Option<BarDirection> {
        self.raw_val(interner, "barDir")
            .and_then(BarDirection::from_wire)
    }

    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<BarGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(BarGrouping::from_wire)
    }
}

declare_plot!(
    /// `c:line3DChart` (`CT_Line3DChart`) — a three-dimensional line plot and its series.
    Line3DChart,
    ChartKind::Line3D
);

impl Line3DChart {
    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<SeriesGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(SeriesGrouping::from_wire)
    }
}

declare_plot!(
    /// `c:area3DChart` (`CT_Area3DChart`) — a three-dimensional area plot and its series.
    Area3DChart,
    ChartKind::Area3D
);

impl Area3DChart {
    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<SeriesGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(SeriesGrouping::from_wire)
    }
}

declare_plot!(
    /// `c:pie3DChart` (`CT_Pie3DChart`) — a three-dimensional pie plot and its series.
    Pie3DChart,
    ChartKind::Pie3D
);

declare_plot!(
    /// `c:ofPieChart` (`CT_OfPieChart`) — a pie whose small slices are broken out into a second pie
    /// or a stacked bar.
    OfPieChart,
    ChartKind::OfPie
);

impl OfPieChart {
    /// What the small slices are broken out into (`c:ofPieType`), or `None` if unrecognized.
    #[must_use]
    pub fn of_pie_type(&self, interner: &Interner) -> Option<OfPieType> {
        self.raw_val(interner, "ofPieType")
            .and_then(OfPieType::from_wire)
    }

    /// The size of the second plot as a percentage of the first (`c:secondPieSize`).
    #[must_use]
    pub fn second_plot_size(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "secondPieSize")
            .and_then(|value| value.trim().parse().ok())
    }
}

declare_plot!(
    /// `c:radarChart` (`CT_RadarChart`) — a radar plot: one spoke per category, one ring per series.
    RadarChart,
    ChartKind::Radar
);

impl RadarChart {
    /// How the series are drawn (`c:radarStyle`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn radar_style(&self, interner: &Interner) -> Option<RadarStyle> {
        self.raw_val(interner, "radarStyle")
            .and_then(RadarStyle::from_wire)
    }
}

declare_plot!(
    /// `c:bubbleChart` (`CT_BubbleChart`) — X/Y points sized by a third value. Its series carry
    /// `c:xVal`/`c:yVal`/`c:bubbleSize` (see [`Series::bubble_sizes`]).
    BubbleChart,
    ChartKind::Bubble
);

impl BubbleChart {
    /// The bubble sizes' scale, as a percentage of the default (`c:bubbleScale`).
    #[must_use]
    pub fn bubble_scale(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "bubbleScale")
            .and_then(|value| value.trim().parse().ok())
    }

    /// Whether a negative size is drawn as a bubble rather than dropped (`c:showNegBubbles`).
    #[must_use]
    pub fn shows_negative_bubbles(&self, interner: &Interner) -> Option<bool> {
        self.raw_val(interner, "showNegBubbles")
            .and_then(on_off::from_wire)
    }
}

declare_plot!(
    /// `c:stockChart` (`CT_StockChart`) — a high-low-close style plot. The schema requires three or
    /// four series (open, high, low, close), each an ordinary line series.
    StockChart,
    ChartKind::Stock
);

declare_plot!(
    /// `c:surfaceChart` (`CT_SurfaceChart`) — a surface seen from above: a contour map.
    SurfaceChart,
    ChartKind::Surface
);

impl SurfaceChart {
    /// Whether the surface is drawn as a wireframe rather than filled (`c:wireframe`).
    #[must_use]
    pub fn is_wireframe(&self, interner: &Interner) -> Option<bool> {
        self.raw_val(interner, "wireframe")
            .and_then(on_off::from_wire)
    }
}

declare_plot!(
    /// `c:surface3DChart` (`CT_Surface3DChart`) — a three-dimensional surface plot.
    Surface3DChart,
    ChartKind::Surface3D
);

impl Surface3DChart {
    /// Whether the surface is drawn as a wireframe rather than filled (`c:wireframe`).
    #[must_use]
    pub fn is_wireframe(&self, interner: &Interner) -> Option<bool> {
        self.raw_val(interner, "wireframe")
            .and_then(on_off::from_wire)
    }
}
