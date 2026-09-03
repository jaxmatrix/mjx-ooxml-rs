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

use mjx_ooxml_types::child_order::{
    ChildOrder, ChildSlot, AREA_3D_CHART, AREA_CHART, AREA_SERIES, BAR_3D_CHART, BAR_CHART,
    BAR_SERIES, BUBBLE_CHART, BUBBLE_SERIES, DOUGHNUT_CHART, LINE_3D_CHART, LINE_CHART,
    LINE_SERIES, OF_PIE_CHART, PIE_3D_CHART, PIE_CHART, PIE_SERIES, RADAR_CHART, RADAR_SERIES,
    SCATTER_CHART, SCATTER_SERIES, STOCK_CHART, SURFACE_3D_CHART, SURFACE_CHART, SURFACE_SERIES,
};

use crate::author::ChartDataError;
use crate::axis::chart_local;
use crate::build::{
    chart_element, fidelity_element_impls, insert_position, is_dml, raw_child_attr,
};
use crate::data::{CategoryData, NumericData, SeriesText};
use crate::decoration::{
    DanglingPointReference, DataLabelSettings, DataLabelSpec, DataLabels, DataPointFormat,
    ErrorBarSpec, ErrorBars, Trendline, TrendlineSpec,
};

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

    /// The generated child order of the `CT_*Chart` complex type this kind *is* — where a
    /// plot-level `c:dLbls` belongs among its siblings, which differs by kind (a bar plot puts it
    /// before `c:gapWidth`, a stock plot before `c:dropLines`).
    #[must_use]
    pub fn plot_child_order(self) -> &'static ChildOrder {
        match self {
            Self::Bar => BAR_CHART,
            Self::Bar3D => BAR_3D_CHART,
            Self::Line => LINE_CHART,
            Self::Line3D => LINE_3D_CHART,
            Self::Pie => PIE_CHART,
            Self::Pie3D => PIE_3D_CHART,
            Self::OfPie => OF_PIE_CHART,
            Self::Area => AREA_CHART,
            Self::Area3D => AREA_3D_CHART,
            Self::Scatter => SCATTER_CHART,
            Self::Doughnut => DOUGHNUT_CHART,
            Self::Radar => RADAR_CHART,
            Self::Bubble => BUBBLE_CHART,
            Self::Stock => STOCK_CHART,
            Self::Surface => SURFACE_CHART,
            Self::Surface3D => SURFACE_3D_CHART,
        }
    }

    /// The generated child order of the `CT_*Ser` complex type a series of this kind *is*.
    ///
    /// The sixteen plot types share only eight series types, and they differ in more than placement:
    /// `CT_PieSer` declares no `c:trendline` and no `c:errBars`, and `CT_SurfaceSer` declares no
    /// decoration at all. This is the one source of truth for both questions — see
    /// [`admits_series_child`](Self::admits_series_child).
    #[must_use]
    pub fn series_child_order(self) -> &'static ChildOrder {
        match self {
            Self::Bar | Self::Bar3D => BAR_SERIES,
            Self::Line | Self::Line3D | Self::Stock => LINE_SERIES,
            Self::Pie | Self::Pie3D | Self::OfPie | Self::Doughnut => PIE_SERIES,
            Self::Area | Self::Area3D => AREA_SERIES,
            Self::Scatter => SCATTER_SERIES,
            Self::Radar => RADAR_SERIES,
            Self::Bubble => BUBBLE_SERIES,
            Self::Surface | Self::Surface3D => SURFACE_SERIES,
        }
    }

    /// Whether a series of this kind may carry a child named `local` — asked of the generated table,
    /// never of a hand-written list.
    ///
    /// ```
    /// use mjx_chart::ChartKind;
    /// // A bar series may carry a trendline; a pie slice may not.
    /// assert!(ChartKind::Bar.admits_series_child("trendline"));
    /// assert!(!ChartKind::Pie.admits_series_child("trendline"));
    /// // A surface series carries no decoration at all.
    /// assert!(!ChartKind::Surface.admits_series_child("dLbls"));
    /// ```
    #[must_use]
    pub fn admits_series_child(self, local: &str) -> bool {
        self.series_child_order().slot(None, local).is_some()
    }

    /// Whether a plot of this kind may carry a child named `local`. The two surface plots declare no
    /// `c:dLbls`, so a chart-wide label setting is not something they can be given.
    #[must_use]
    pub fn admits_plot_child(self, local: &str) -> bool {
        self.plot_child_order().slot(None, local).is_some()
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
        let (name, empty) = (element.name, element.empty);
        let content = element.into_content();
        Self {
            name,
            attributes: content.attributes,
            children: content.children,
            empty,
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
/// or its X/Y data (scatter), its decoration, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
    /// One point drawn differently from the rest (`c:dPt`). A series may carry many.
    PointFormat(DataPointFormat),
    /// The series' data-label settings (`c:dLbls`) — at most one.
    DataLabels(DataLabels),
    /// A curve fitted through the series (`c:trendline`). A series may carry many.
    Trendline(Trendline),
    /// The uncertainty drawn around each point (`c:errBars`).
    ErrorBars(ErrorBars),
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
        child(local = "spPr", variant = ShapeProperties, ty = SeriesShapeProperties),
        child(local = "dPt", variant = PointFormat, ty = DataPointFormat),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels),
        child(local = "trendline", variant = Trendline, ty = Trendline),
        child(local = "errBars", variant = ErrorBars, ty = ErrorBars)
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

    /// The series' shape properties, creating an empty `c:spPr` at its schema rank if it has none.
    ///
    /// `c:spPr` is a member of `EG_SerShared`, which every one of the eight `CT_*Ser` types opens
    /// with, so it sits at rank 3 in all of them and the bar series' order places it correctly for
    /// any kind — `every_series_type_places_shape_properties_alike` holds that.
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
        let at = self.insert_index(BAR_SERIES, interner, "spPr");
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

    // --- decoration: read ------------------------------------------------------------------------

    /// How many points the series has — the addressing space every `c:idx` in its decoration is
    /// measured against.
    ///
    /// A series' points are its **values** (`c:val`, or `c:yVal` for an X/Y plot): a category axis
    /// may name more categories than a series has values, and those extra categories are not points
    /// of it. Only a series with no numeric source at all falls back to counting its categories.
    ///
    /// The count is the one its source **declares** (`c:ptCount`) where it declares one, rather than
    /// the number of `c:pt` it lists. A sparse cache omits the points that were blank, and a `c:dPt`
    /// naming one of the omitted points is addressing real data — counting the listed points alone
    /// would fault correct markup as dangling.
    #[must_use]
    pub fn point_count(&self, interner: &Interner) -> usize {
        if let Some(numeric) = self.values().or_else(|| self.y_data()) {
            let declared = numeric
                .declared_point_count(interner)
                .map_or(0, |count| count as usize);
            return declared.max(numeric.values().len());
        }
        self.categories()
            .or_else(|| self.x_data())
            .map_or(0, |data| data.labels().len())
    }

    /// The series' data-label settings (`c:dLbls`), or `None` when it states none and takes its
    /// plot's. This is the **middle** of the three tiers — see [`DataLabelSettings::inherit`].
    #[must_use]
    pub fn data_labels(&self) -> Option<&DataLabels> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::DataLabels(labels) => Some(labels),
            _ => None,
        })
    }

    /// The series' data-label settings, mutably. `None` when it declares none — use
    /// [`SeriesDecoration::data_labels_mut`] to create them, which knows whether the series' type
    /// admits them at all.
    pub fn data_labels_mut(&mut self) -> Option<&mut DataLabels> {
        self.content.iter_mut().find_map(|item| match item {
            SeriesContent::DataLabels(labels) => Some(labels),
            _ => None,
        })
    }

    /// The label settings in force for one point of this series, merging the point tier over the
    /// series tier. `plot` is the outermost tier, from the owning plot's own `c:dLbls`.
    #[must_use]
    pub fn resolved_data_labels(
        &self,
        interner: &Interner,
        plot: &DataLabelSettings,
        point_index: Option<u32>,
    ) -> DataLabelSettings {
        let labels = self.data_labels();
        let series = labels
            .map(|labels| labels.settings(interner))
            .unwrap_or_default();
        let merged = series.inherit(plot);
        let Some(point_index) = point_index else {
            return merged;
        };
        labels
            .and_then(|labels| labels.label_for_point(interner, point_index))
            .map(|label| label.settings(interner))
            .unwrap_or_default()
            .inherit(&merged)
    }

    /// Every point of the series that carries its own formatting (`c:dPt`), in document order.
    pub fn point_formats(&self) -> impl Iterator<Item = &DataPointFormat> {
        self.content.iter().filter_map(|item| match item {
            SeriesContent::PointFormat(format) => Some(format),
            _ => None,
        })
    }

    /// The formatting of the point at `index`, matched on its `c:idx` — never on its position in
    /// the list.
    #[must_use]
    pub fn point_format(&self, interner: &Interner, index: u32) -> Option<&DataPointFormat> {
        self.point_formats()
            .find(|format| format.index(interner) == Some(index))
    }

    /// Every trendline fitted through the series (`c:trendline`), in document order.
    pub fn trendlines(&self) -> impl Iterator<Item = &Trendline> {
        self.content.iter().filter_map(|item| match item {
            SeriesContent::Trendline(trendline) => Some(trendline),
            _ => None,
        })
    }

    /// Every set of error bars the series carries (`c:errBars`) — one for a bar or line series, up
    /// to two (x and y) for scatter, area and bubble.
    pub fn error_bars(&self) -> impl Iterator<Item = &ErrorBars> {
        self.content.iter().filter_map(|item| match item {
            SeriesContent::ErrorBars(bars) => Some(bars),
            _ => None,
        })
    }

    /// The series' per-point formatting, mutably — for editing one point's fill, outline or
    /// explosion in place. Adding one goes through [`SeriesDecoration::point_format_mut`], which
    /// knows where the schema puts it.
    pub fn point_formats_mut(&mut self) -> impl Iterator<Item = &mut DataPointFormat> {
        self.content.iter_mut().filter_map(|item| match item {
            SeriesContent::PointFormat(format) => Some(format),
            _ => None,
        })
    }

    /// The series' trendlines, mutably — for editing a curve in place.
    pub fn trendlines_mut(&mut self) -> impl Iterator<Item = &mut Trendline> {
        self.content.iter_mut().filter_map(|item| match item {
            SeriesContent::Trendline(trendline) => Some(trendline),
            _ => None,
        })
    }

    /// The series' error bars, mutably — for editing a set in place.
    pub fn error_bars_mut(&mut self) -> impl Iterator<Item = &mut ErrorBars> {
        self.content.iter_mut().filter_map(|item| match item {
            SeriesContent::ErrorBars(bars) => Some(bars),
            _ => None,
        })
    }

    /// Every `c:dPt` and `c:dLbl` whose `c:idx` names a point at or past
    /// [`point_count`](Self::point_count) — the decoration an edit that shortened the series left
    /// pointing at nothing.
    ///
    /// This crate never renumbers such an element, so this is a *report*, not a repair: what was
    /// anchored to point 4 still says point 4. [`SeriesDecoration::drop_decoration_beyond_data`]
    /// removes them when a caller decides that is what they want.
    #[must_use]
    pub fn decoration_beyond_data(&self, interner: &Interner) -> Vec<DanglingPointReference> {
        let count = self.point_count(interner);
        let limit = u32::try_from(count).unwrap_or(u32::MAX);
        let mut dangling: Vec<DanglingPointReference> = self
            .point_formats()
            .filter_map(|format| format.index(interner))
            .filter(|index| *index >= limit)
            .map(|index| DanglingPointReference {
                element: "dPt",
                index,
            })
            .collect();
        if let Some(labels) = self.data_labels() {
            dangling.extend(
                labels
                    .labels_beyond(interner, count)
                    .into_iter()
                    .map(|index| DanglingPointReference {
                        element: "dLbl",
                        index,
                    }),
            );
        }
        dangling
    }

    // --- decoration: placement -------------------------------------------------------------------

    /// Each child's local name in document order, or `None` for a node the schema does not name.
    fn content_locals<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> impl Iterator<Item = Option<&'a str>> {
        self.content.iter().map(move |item| match item {
            SeriesContent::Text(_) => Some("tx"),
            SeriesContent::Categories(_) => Some("cat"),
            SeriesContent::Values(_) => Some("val"),
            SeriesContent::XValues(_) => Some("xVal"),
            SeriesContent::YValues(_) => Some("yVal"),
            SeriesContent::BubbleSizes(_) => Some("bubbleSize"),
            SeriesContent::ShapeProperties(_) => Some("spPr"),
            SeriesContent::PointFormat(_) => Some("dPt"),
            SeriesContent::DataLabels(_) => Some("dLbls"),
            SeriesContent::Trendline(_) => Some("trendline"),
            SeriesContent::ErrorBars(_) => Some("errBars"),
            SeriesContent::Raw(node) => chart_local(node, interner),
        })
    }

    /// Where a child named `local` belongs among the series' current children, according to the
    /// `CT_*Ser` the owning plot's kind names.
    fn insert_index(&self, order: &ChildOrder, interner: &Interner, local: &str) -> usize {
        insert_position(order, self.content_locals(interner), local)
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

/// A series together with the plot type that owns it — the **write** surface for decoration.
///
/// Reading a series' decoration needs nothing but the series ([`Series::data_labels`],
/// [`Series::point_formats`], …). Writing it needs the plot's kind, for two reasons the schema
/// makes unavoidable:
///
/// * **Placement.** `CT_BarSer` puts `c:dPt` at rank 6 and `CT_PieSer` at rank 5, because the bar
///   series declares `c:invertIfNegative` and `c:pictureOptions` first and the pie series declares
///   `c:explosion`. A child at the wrong rank is invalid markup, not untidy markup.
/// * **Admissibility.** `CT_PieSer` declares no `c:trendline` and no `c:errBars`; `CT_SurfaceSer`
///   declares no decoration at all. Asking for one of those is refused with
///   [`ChartDataError::DecorationNotAllowed`] *before* anything is written, the same way
///   [`ChartData::validate`](crate::ChartData::validate) refuses a shape the schema rejects.
///
/// Obtain one from the plot that owns the series (`series_decoration_mut`), from
/// [`PlotArea::series_decoration_mut`](crate::PlotArea::series_decoration_mut) or from
/// [`ChartSpace::series_decoration_mut`](crate::ChartSpace::series_decoration_mut).
#[derive(Debug)]
pub struct SeriesDecoration<'a> {
    series: &'a mut Series,
    kind: ChartKind,
}

impl<'a> SeriesDecoration<'a> {
    /// Binds `series` to the `kind` of plot that holds it.
    pub(crate) fn new(series: &'a mut Series, kind: ChartKind) -> Self {
        Self { series, kind }
    }

    /// The kind of plot this series belongs to.
    #[must_use]
    pub fn kind(&self) -> ChartKind {
        self.kind
    }

    /// The series itself.
    #[must_use]
    pub fn series(&self) -> &Series {
        self.series
    }

    /// The series' data-label settings, creating an empty `c:dLbls` at its schema rank if it had
    /// none.
    ///
    /// # Errors
    /// [`ChartDataError::DecorationNotAllowed`] when the series' type declares no `c:dLbls` — the
    /// two surface plots.
    pub fn data_labels_mut(
        &mut self,
        interner: &mut Interner,
    ) -> Result<&mut DataLabels, ChartDataError> {
        self.require("dLbls")?;
        if self
            .series
            .content
            .iter()
            .all(|item| !matches!(item, SeriesContent::DataLabels(_)))
        {
            let labels = DataLabels::new(interner, &DataLabelSpec::default());
            let at = self
                .series
                .insert_index(self.kind.series_child_order(), interner, "dLbls");
            self.series
                .content
                .insert(at, SeriesContent::DataLabels(labels));
            self.series.empty = false;
        }
        self.series
            .data_labels_mut()
            .ok_or(ChartDataError::DecorationNotAllowed {
                plot: self.kind.element_local_name(),
                element: "dLbls",
                series_type: self.kind.series_child_order().symbol,
            })
    }

    /// Applies `spec` to the series' data labels, creating them if it had none and leaving every
    /// setting `spec` does not state alone.
    ///
    /// # Errors
    /// As [`data_labels_mut`](Self::data_labels_mut).
    pub fn set_data_labels(
        &mut self,
        interner: &mut Interner,
        spec: &DataLabelSpec,
    ) -> Result<(), ChartDataError> {
        self.data_labels_mut(interner)?;
        let Some(labels) = self.series.data_labels_mut() else {
            return Ok(());
        };
        labels.apply(interner, spec);
        Ok(())
    }

    /// Suppresses every label of the series — a `c:dLbls` carrying `c:delete val="1"`, which is how
    /// one series of a labelled plot is silenced.
    ///
    /// # Errors
    /// As [`data_labels_mut`](Self::data_labels_mut).
    pub fn suppress_data_labels(&mut self, interner: &mut Interner) -> Result<(), ChartDataError> {
        self.data_labels_mut(interner)?;
        if let Some(labels) = self.series.data_labels_mut() {
            labels.suppress_all(interner);
        }
        Ok(())
    }

    /// Removes the series' `c:dLbls` entirely, so it inherits its plot's. Answers whether one was
    /// there.
    pub fn remove_data_labels(&mut self) -> bool {
        let before = self.series.content.len();
        self.series
            .content
            .retain(|item| !matches!(item, SeriesContent::DataLabels(_)));
        before != self.series.content.len()
    }

    /// Sets the label of one point, overriding the series' settings for it alone.
    ///
    /// # Errors
    /// [`ChartDataError::DataPointOutOfRange`] when `point` names no point of this series,
    /// [`ChartDataError::DecorationNotAllowed`] when the series' type declares no `c:dLbls`, and
    /// [`ChartDataError::SettingNotAtThisTier`] when `spec` asks for leader lines.
    pub fn set_point_label(
        &mut self,
        interner: &mut Interner,
        point: u32,
        spec: &DataLabelSpec,
    ) -> Result<(), ChartDataError> {
        self.require_point(interner, point)?;
        self.data_labels_mut(interner)?;
        let Some(labels) = self.series.data_labels_mut() else {
            return Ok(());
        };
        labels.set_label_for_point(interner, point, spec)
    }

    /// Suppresses the label of one point, leaving the rest of the series labelled.
    ///
    /// # Errors
    /// As [`set_point_label`](Self::set_point_label), minus the leader-line case.
    pub fn suppress_point_label(
        &mut self,
        interner: &mut Interner,
        point: u32,
    ) -> Result<(), ChartDataError> {
        self.require_point(interner, point)?;
        self.data_labels_mut(interner)?;
        if let Some(labels) = self.series.data_labels_mut() {
            labels.suppress_label_for_point(interner, point);
        }
        Ok(())
    }

    /// Removes one point's label override, so it falls back to the series' settings. Answers
    /// whether one was there.
    pub fn remove_point_label(&mut self, interner: &Interner, point: u32) -> bool {
        self.series
            .data_labels_mut()
            .is_some_and(|labels| labels.remove_label_for_point(interner, point))
    }

    /// The formatting of the point at `point`, creating a `c:dPt` at its schema rank if there is
    /// none. Existing formatting is found by `c:idx`, never by list position.
    ///
    /// # Errors
    /// [`ChartDataError::DataPointOutOfRange`] when `point` names no point of this series, and
    /// [`ChartDataError::DecorationNotAllowed`] when the series' type declares no `c:dPt`.
    pub fn point_format_mut(
        &mut self,
        interner: &mut Interner,
        point: u32,
    ) -> Result<&mut DataPointFormat, ChartDataError> {
        self.require("dPt")?;
        self.require_point(interner, point)?;
        let existing = self.series.content.iter().position(|item| match item {
            SeriesContent::PointFormat(format) => format.index(interner) == Some(point),
            _ => false,
        });
        let at = match existing {
            Some(at) => at,
            None => {
                let format = DataPointFormat::new(interner, point);
                let at = self.point_format_insert_index(interner, point);
                self.series
                    .content
                    .insert(at, SeriesContent::PointFormat(format));
                self.series.empty = false;
                at
            }
        };
        match &mut self.series.content[at] {
            SeriesContent::PointFormat(format) => Ok(format),
            _ => unreachable!("the index names a PointFormat"),
        }
    }

    /// Colours the point at `point` differently from the rest of its series.
    ///
    /// # Errors
    /// As [`point_format_mut`](Self::point_format_mut).
    pub fn set_point_fill(
        &mut self,
        interner: &mut Interner,
        point: u32,
        fill: &FillSpec,
    ) -> Result<(), ChartDataError> {
        self.point_format_mut(interner, point)?;
        let Some(at) = self.point_format_position(interner, point) else {
            return Ok(());
        };
        if let SeriesContent::PointFormat(format) = &mut self.series.content[at] {
            format.set_fill(interner, fill);
        }
        Ok(())
    }

    /// Outlines the point at `point` differently from the rest of its series.
    ///
    /// # Errors
    /// As [`point_format_mut`](Self::point_format_mut).
    pub fn set_point_line(
        &mut self,
        interner: &mut Interner,
        point: u32,
        line: &LineSpec,
    ) -> Result<(), ChartDataError> {
        self.point_format_mut(interner, point)?;
        let Some(at) = self.point_format_position(interner, point) else {
            return Ok(());
        };
        if let SeriesContent::PointFormat(format) = &mut self.series.content[at] {
            format.set_line(interner, line);
        }
        Ok(())
    }

    /// Removes the formatting of the point at `point`, so it is drawn like the rest of its series.
    /// Answers whether any was there.
    pub fn remove_point_format(&mut self, interner: &Interner, point: u32) -> bool {
        match self.point_format_position(interner, point) {
            Some(at) => {
                self.series.content.remove(at);
                true
            }
            None => false,
        }
    }

    /// Adds a trendline to the series. `c:trendline` is repeatable, so this appends rather than
    /// replacing — a series may carry a linear fit and a moving average at once.
    ///
    /// # Errors
    /// [`ChartDataError::DecorationNotAllowed`] when the series' type declares no `c:trendline`
    /// (pie, doughnut, pie-of-pie, radar and surface), plus whatever
    /// [`TrendlineSpec::validate`] answers — all checked before anything is written.
    pub fn add_trendline(
        &mut self,
        interner: &mut Interner,
        spec: &TrendlineSpec,
    ) -> Result<(), ChartDataError> {
        self.require("trendline")?;
        spec.validate()?;
        let trendline = Trendline::new(interner, spec);
        let at = self
            .series
            .insert_index(self.kind.series_child_order(), interner, "trendline");
        self.series
            .content
            .insert(at, SeriesContent::Trendline(trendline));
        self.series.empty = false;
        Ok(())
    }

    /// Rewrites the `n`-th trendline of the series from `spec`, in place. Answers `false`, changing
    /// nothing, when the series carries fewer.
    ///
    /// # Errors
    /// Whatever [`TrendlineSpec::validate`] answers.
    pub fn set_trendline(
        &mut self,
        interner: &mut Interner,
        trendline_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<bool, ChartDataError> {
        spec.validate()?;
        match self.series.trendlines_mut().nth(trendline_idx) {
            Some(trendline) => trendline.apply_spec(interner, spec).map(|()| true),
            None => Ok(false),
        }
    }

    /// Removes every trendline from the series, answering how many went.
    pub fn remove_trendlines(&mut self) -> usize {
        let before = self.series.content.len();
        self.series
            .content
            .retain(|item| !matches!(item, SeriesContent::Trendline(_)));
        before - self.series.content.len()
    }

    /// Gives the series error bars, replacing an existing set that runs along the same axis and
    /// inserting a new one otherwise.
    ///
    /// A series type whose `c:errBars` is not repeatable (`CT_BarSer`, `CT_LineSer`) admits only
    /// one set, so an existing set is replaced whatever its direction; scatter, area and bubble
    /// admit two — one per axis — and this keeps them apart by `c:errDir`.
    ///
    /// # Errors
    /// [`ChartDataError::DecorationNotAllowed`] when the series' type declares no `c:errBars`, plus
    /// whatever [`ErrorBarSpec::validate`] answers.
    pub fn set_error_bars(
        &mut self,
        interner: &mut Interner,
        spec: &ErrorBarSpec,
    ) -> Result<(), ChartDataError> {
        let slot = self.require("errBars")?;
        spec.validate()?;
        let existing = self.series.content.iter().position(|item| match item {
            SeriesContent::ErrorBars(bars) => {
                !slot.repeatable || bars.direction(interner) == spec.direction
            }
            _ => false,
        });
        if let Some(at) = existing {
            if let SeriesContent::ErrorBars(bars) = &mut self.series.content[at] {
                return bars.apply_spec(interner, spec);
            }
        }
        let bars = ErrorBars::new(interner, spec);
        let at = self
            .series
            .insert_index(self.kind.series_child_order(), interner, "errBars");
        self.series
            .content
            .insert(at, SeriesContent::ErrorBars(bars));
        self.series.empty = false;
        Ok(())
    }

    /// Removes every set of error bars from the series, answering how many went.
    pub fn remove_error_bars(&mut self) -> usize {
        let before = self.series.content.len();
        self.series
            .content
            .retain(|item| !matches!(item, SeriesContent::ErrorBars(_)));
        before - self.series.content.len()
    }

    /// Removes every `c:dPt` and `c:dLbl` that names a point at or past the end of the series'
    /// data, answering how many went — the explicit repair for what
    /// [`Series::decoration_beyond_data`] reports.
    ///
    /// Nothing calls this on a caller's behalf. An edit that shortens a series leaves its `c:idx`
    /// values exactly as they were, because renumbering them would silently move one point's colour
    /// onto another; dropping them is a decision, and this is where a caller makes it.
    pub fn drop_decoration_beyond_data(&mut self, interner: &Interner) -> usize {
        let count = self.series.point_count(interner);
        let limit = u32::try_from(count).unwrap_or(u32::MAX);
        let before = self.series.content.len();
        self.series.content.retain(|item| match item {
            SeriesContent::PointFormat(format) => {
                format.index(interner).is_none_or(|index| index < limit)
            }
            _ => true,
        });
        let mut removed = before - self.series.content.len();
        if let Some(labels) = self.series.data_labels_mut() {
            removed += labels.drop_labels_beyond(interner, count);
        }
        removed
    }

    /// Checks that the series' type declares a child named `local`, answering its slot so a caller
    /// can also ask whether it repeats.
    fn require(&self, local: &'static str) -> Result<&'static ChildSlot, ChartDataError> {
        self.kind.series_child_order().slot(None, local).ok_or(
            ChartDataError::DecorationNotAllowed {
                plot: self.kind.element_local_name(),
                element: local,
                series_type: self.kind.series_child_order().symbol,
            },
        )
    }

    /// Checks that `point` names a point this series actually has.
    fn require_point(&self, interner: &Interner, point: u32) -> Result<(), ChartDataError> {
        let count = self.series.point_count(interner);
        if (point as usize) < count {
            return Ok(());
        }
        Err(ChartDataError::DataPointOutOfRange {
            index: point,
            count,
        })
    }

    /// The position in the series' content of the `c:dPt` naming `point`.
    fn point_format_position(&self, interner: &Interner, point: u32) -> Option<usize> {
        self.series.content.iter().position(|item| match item {
            SeriesContent::PointFormat(format) => format.index(interner) == Some(point),
            _ => false,
        })
    }

    /// Where a new `c:dPt` for `point` belongs: at the schema rank of the `c:dPt` run, and within
    /// that run in ascending `c:idx` order — which is how Office writes them.
    fn point_format_insert_index(&self, interner: &Interner, point: u32) -> usize {
        let rank_at = self
            .series
            .insert_index(self.kind.series_child_order(), interner, "dPt");
        let mut at = rank_at;
        for (offset, item) in self.series.content[..rank_at].iter().enumerate().rev() {
            match item {
                SeriesContent::PointFormat(format) => {
                    if format.index(interner).is_some_and(|index| index > point) {
                        at = offset;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        at
    }
}

/// One ordered child of a plot (`c:barChart`, `c:lineChart`, …): a typed series (`c:ser`), the
/// plot-wide data-label settings (`c:dLbls`), or an opaque node (`c:barDir`, `c:grouping`,
/// `c:axId`, `c:firstSliceAng`, whitespace, unknown).
///
/// Every plot type shares this shape — a run of series interleaved with the type-specific scalars
/// and axes this tier does not model — so it needs one content enum, not one per type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlotContent {
    /// A series (`c:ser`).
    Series(Series),
    /// The data-label settings for every series of this plot (`c:dLbls`) — the outermost of the
    /// three tiers. The two surface plots declare none.
    DataLabels(DataLabels),
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
                    _ => None,
                })
            }

            /// The plot's series, in order, mutably — for rewriting a series' cached data.
            pub fn series_mut(&mut self) -> impl Iterator<Item = &mut Series> {
                self.content.iter_mut().filter_map(|item| match item {
                    PlotContent::Series(series) => Some(series),
                    _ => None,
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

            /// The `n`-th series bound to this plot's kind — the write surface for its decoration.
            /// `None` when the plot has fewer series.
            pub fn series_decoration_mut(&mut self, n: usize) -> Option<SeriesDecoration<'_>> {
                self.series_mut()
                    .nth(n)
                    .map(|series| SeriesDecoration::new(series, $kind))
            }

            /// The plot-wide data-label settings (`c:dLbls`) — the outermost of the three tiers —
            /// or `None` when the plot states none.
            #[must_use]
            pub fn data_labels(&self) -> Option<&DataLabels> {
                self.content.iter().find_map(|item| match item {
                    PlotContent::DataLabels(labels) => Some(labels),
                    _ => None,
                })
            }

            /// The plot-wide data-label settings, creating an empty `c:dLbls` at its schema rank if
            /// the plot had none.
            ///
            /// # Errors
            /// [`ChartDataError::DecorationNotAllowed`] when this plot type declares no `c:dLbls`
            /// — `CT_SurfaceChart` and `CT_Surface3DChart` do not.
            pub fn data_labels_mut(
                &mut self,
                interner: &mut Interner,
            ) -> Result<&mut DataLabels, ChartDataError> {
                if !$kind.admits_plot_child("dLbls") {
                    return Err(ChartDataError::DecorationNotAllowed {
                        plot: $kind.element_local_name(),
                        element: "dLbls",
                        series_type: $kind.plot_child_order().symbol,
                    });
                }
                let existing = self
                    .content
                    .iter()
                    .position(|item| matches!(item, PlotContent::DataLabels(_)));
                let at = match existing {
                    Some(at) => at,
                    None => {
                        let labels = DataLabels::new(interner, &DataLabelSpec::default());
                        let at = insert_position(
                            $kind.plot_child_order(),
                            self.plot_content_locals(interner),
                            "dLbls",
                        );
                        self.content.insert(at, PlotContent::DataLabels(labels));
                        self.empty = false;
                        at
                    }
                };
                match &mut self.content[at] {
                    PlotContent::DataLabels(labels) => Ok(labels),
                    _ => unreachable!("the index names a DataLabels"),
                }
            }

            /// Applies `spec` to the plot's data labels, creating them if it had none. Every series
            /// that states nothing of its own takes these.
            ///
            /// # Errors
            /// As [`data_labels_mut`](Self::data_labels_mut).
            pub fn set_data_labels(
                &mut self,
                interner: &mut Interner,
                spec: &DataLabelSpec,
            ) -> Result<(), ChartDataError> {
                self.data_labels_mut(interner)?;
                let Some(at) = self
                    .content
                    .iter()
                    .position(|item| matches!(item, PlotContent::DataLabels(_)))
                else {
                    return Ok(());
                };
                if let PlotContent::DataLabels(labels) = &mut self.content[at] {
                    labels.apply(interner, spec);
                }
                Ok(())
            }

            /// Removes the plot's `c:dLbls`, answering whether one was there.
            pub fn remove_data_labels(&mut self) -> bool {
                let before = self.content.len();
                self.content
                    .retain(|item| !matches!(item, PlotContent::DataLabels(_)));
                before != self.content.len()
            }

            /// The label settings the plot states in its own right — the tier every series of it
            /// inherits from.
            #[must_use]
            pub fn plot_label_settings(&self, interner: &Interner) -> DataLabelSettings {
                self.data_labels()
                    .map(|labels| labels.settings(interner))
                    .unwrap_or_default()
            }

            /// The label settings in force for one point of one series, merged across all three
            /// tiers: the point's `c:dLbl` over the series' `c:dLbls` over this plot's.
            ///
            /// Pass `point_index = None` to stop at the series tier. A `series_index` the plot does
            /// not have yields the plot tier alone.
            #[must_use]
            pub fn resolved_data_labels(
                &self,
                interner: &Interner,
                series_index: usize,
                point_index: Option<u32>,
            ) -> DataLabelSettings {
                let plot = self.plot_label_settings(interner);
                match self.series_at(series_index) {
                    Some(series) => series.resolved_data_labels(interner, &plot, point_index),
                    None => plot,
                }
            }

            /// Each child's local name in document order, for placement.
            fn plot_content_locals<'a>(
                &'a self,
                interner: &'a Interner,
            ) -> impl Iterator<Item = Option<&'a str>> {
                self.content.iter().map(move |item| match item {
                    PlotContent::Series(_) => Some("ser"),
                    PlotContent::DataLabels(_) => Some("dLbls"),
                    PlotContent::Raw(node) => chart_local(node, interner),
                })
            }

            /// Reads the `@val` of a raw scalar child of this plot (`c:grouping`, `c:holeSize`, …).
            fn raw_val(&self, interner: &Interner, local: &str) -> Option<&str> {
                let raw = self.content.iter().filter_map(|item| match item {
                    PlotContent::Raw(node) => Some(node),
                    _ => None,
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
    #[xml(
        children,
        child(local = "ser", variant = Series, ty = Series),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels)
    )]
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
    #[xml(
        children,
        child(local = "ser", variant = Series, ty = Series),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels)
    )]
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
    #[xml(
        children,
        child(local = "ser", variant = Series, ty = Series),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels)
    )]
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
    #[xml(
        children,
        child(local = "ser", variant = Series, ty = Series),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels)
    )]
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
    #[xml(
        children,
        child(local = "ser", variant = Series, ty = Series),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels)
    )]
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
    #[xml(
        children,
        child(local = "ser", variant = Series, ty = Series),
        child(local = "dLbls", variant = DataLabels, ty = DataLabels)
    )]
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
            #[xml(
                children,
                child(local = "ser", variant = Series, ty = Series),
                child(local = "dLbls", variant = DataLabels, ty = DataLabels)
            )]
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
