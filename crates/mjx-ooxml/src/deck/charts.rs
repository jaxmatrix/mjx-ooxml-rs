//! Charts: adding one from a description, reading and rewriting its series, and its axes, title and
//! legend.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{
    AxisOrientation, ChartAxisData, ChartData, ChartKind, ChartLegendData, ChartSeriesData,
    ChartWorkbook, Deck, Error, LegendPosition, ShapeBounds, ShapePath, Surface,
};

impl Deck {
    /// Adds `chart` to `surface` as a new chart, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_chart`](mjx_pptx::Presentation::add_chart).
    pub fn add_chart(
        &mut self,
        surface: Surface,
        chart: &ChartData,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_chart(
            surface.to_model(),
            chart,
            bounds,
        )?))
    }

    /// The raw XML bytes of the chart part the chart frame `shape_idx` on `surface` references
    /// (`/ppt/charts/chartN.xml`), exactly as the package holds them, or `None` when the shape frames
    /// no chart. Borrowed from the package, so the part is not copied.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_part_bytes`](mjx_pptx::Presentation::chart_part_bytes).
    pub fn chart_part_bytes(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .presentation
            .chart_part_bytes(surface.to_model(), shape_idx.to_model())?
            .map(<[u8]>::to_vec))
    }

    /// Every chart on `surface` that references a backing workbook (`c:externalData`), with where each
    /// is referenced from and whether that reference is external.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_workbooks`](mjx_pptx::Presentation::chart_workbooks).
    pub fn chart_workbooks(&mut self, surface: Surface) -> Result<Vec<ChartWorkbook>, Error> {
        Ok(self.presentation.chart_workbooks(surface.to_model())?)
    }

    /// Detaches the backing workbook from the chart `shape_idx` on `surface`: removes its
    /// `c:externalData` reference — the element and its relationship — leaving the chart to render from
    /// its cached values. This neutralizes a chart that links an unreachable external workbook (the
    /// caller decides accessibility; use `chart_workbooks` to find the candidates), and yields exactly
    /// the cache-only shape a freshly authored chart has.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::detach_chart_workbook`](mjx_pptx::Presentation::detach_chart_workbook).
    pub fn detach_chart_workbook(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .detach_chart_workbook(surface.to_model(), shape_idx.to_model())?)
    }

    /// The series of the chart the frame `shape_idx` on `surface` references — for each, its name,
    /// category labels and values (for a scatter series, its X labels and Y values), flattened across
    /// the chart's plots. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_series`](mjx_pptx::Presentation::chart_series).
    pub fn chart_series(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Vec<ChartSeriesData>, Error> {
        Ok(self
            .presentation
            .chart_series(surface.to_model(), shape_idx.to_model())?)
    }

    /// Rewrites the values of series `series_idx` (0-based across the chart's plots) of the chart the
    /// frame `shape_idx` on `surface` references — whichever source the series names: a `c:numRef`'s
    /// cache or a `c:numLit`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_series_values`](mjx_pptx::Presentation::set_chart_series_values).
    pub fn set_chart_series_values(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        values: &[f64],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_series_values(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            values,
        )?)
    }

    /// Rewrites the category labels of series `series_idx` (0-based across the chart's plots) of the
    /// chart the frame `shape_idx` on `surface` references, and refreshes the chart's embedded workbook
    /// alongside it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_series_categories`](mjx_pptx::Presentation::set_chart_series_categories).
    pub fn set_chart_series_categories(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        labels: &[&str],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_series_categories(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            labels,
        )?)
    }

    /// Rewrites the embedded workbook of the chart the frame `shape_idx` on `surface` references so its
    /// cells hold exactly what the chart now draws, and answers whether it rewrote one.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::refresh_chart_workbook`](mjx_pptx::Presentation::refresh_chart_workbook).
    pub fn refresh_chart_workbook(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<bool, Error> {
        Ok(self
            .presentation
            .refresh_chart_workbook(surface.to_model(), shape_idx.to_model())?)
    }

    /// The kind of every plot the chart the frame `shape_idx` on `surface` references draws, in
    /// document order — one entry per plot element, so a combo chart yields several. Reading does not
    /// dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_kinds`](mjx_pptx::Presentation::chart_kinds).
    pub fn chart_kinds(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Vec<ChartKind>, Error> {
        Ok(self
            .presentation
            .chart_kinds(surface.to_model(), shape_idx.to_model())?)
    }

    /// The axes of the chart the frame `shape_idx` on `surface` references, in document order. Reading
    /// does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_axes`](mjx_pptx::Presentation::chart_axes).
    pub fn chart_axes(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Vec<ChartAxisData>, Error> {
        Ok(self
            .presentation
            .chart_axes(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order) of the chart the
    /// frame `shape_idx` on `surface` references. `None` returns that end of the axis to automatic
    /// scaling. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_axis_scale`](mjx_pptx::Presentation::set_chart_axis_scale).
    pub fn set_chart_axis_scale(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        axis_idx: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_axis_scale(
            surface.to_model(),
            shape_idx.to_model(),
            index(axis_idx),
            minimum,
            maximum,
        )?)
    }

    /// Sets the direction of axis `axis_idx` of the chart the frame `shape_idx` on `surface` references
    /// — smallest value first, or reversed. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_axis_orientation`](mjx_pptx::Presentation::set_chart_axis_orientation).
    pub fn set_chart_axis_orientation(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        axis_idx: u32,
        orientation: AxisOrientation,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_axis_orientation(
            surface.to_model(),
            shape_idx.to_model(),
            index(axis_idx),
            orientation,
        )?)
    }

    /// Sets or removes the title of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references. `None` removes the title. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_axis_title`](mjx_pptx::Presentation::set_chart_axis_title).
    pub fn set_chart_axis_title(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        axis_idx: u32,
        text: Option<&str>,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_axis_title(
            surface.to_model(),
            shape_idx.to_model(),
            index(axis_idx),
            text,
        )?)
    }

    /// Turns the gridlines of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references on or off. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_axis_gridlines`](mjx_pptx::Presentation::set_chart_axis_gridlines).
    pub fn set_chart_axis_gridlines(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        axis_idx: u32,
        major: bool,
        minor: bool,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_axis_gridlines(
            surface.to_model(),
            shape_idx.to_model(),
            index(axis_idx),
            major,
            minor,
        )?)
    }

    /// The heading of the chart the frame `shape_idx` on `surface` references (`c:title`), or `None`
    /// when it has none. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_title`](mjx_pptx::Presentation::chart_title).
    pub fn chart_title(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .chart_title(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets or removes the heading of the chart the frame `shape_idx` on `surface` references. `None`
    /// removes it. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_title`](mjx_pptx::Presentation::set_chart_title).
    pub fn set_chart_title(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        text: Option<&str>,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_chart_title(surface.to_model(), shape_idx.to_model(), text)?)
    }

    /// The legend of the chart the frame `shape_idx` on `surface` references, or `None` when it has
    /// none. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_legend`](mjx_pptx::Presentation::chart_legend).
    pub fn chart_legend(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<ChartLegendData>, Error> {
        Ok(self
            .presentation
            .chart_legend(surface.to_model(), shape_idx.to_model())?)
    }

    /// Places the legend of the chart the frame `shape_idx` on `surface` references at `position`,
    /// adding one if the chart had none. `None` removes the legend. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_legend`](mjx_pptx::Presentation::set_chart_legend).
    pub fn set_chart_legend(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        position: Option<LegendPosition>,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_legend(
            surface.to_model(),
            shape_idx.to_model(),
            position,
        )?)
    }

    /// The built-in style id the chart the frame `shape_idx` on `surface` references names
    /// (`c:style@val`, 1 to 48) — the palette and effect set Office draws an unstyled series with — or
    /// `None` when it names none. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_style_id`](mjx_pptx::Presentation::chart_style_id).
    pub fn chart_style_id(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<u32>, Error> {
        Ok(self
            .presentation
            .chart_style_id(surface.to_model(), shape_idx.to_model())?)
    }
}
