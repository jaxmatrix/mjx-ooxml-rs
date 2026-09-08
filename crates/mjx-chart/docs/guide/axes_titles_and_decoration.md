# Axes, titles and decoration

Everything a chart carries that is not a number. Two groups, and they behave differently enough to be
worth keeping apart: the **furniture** around the plot (axes, gridlines, the title, the legend) is a
sibling of the plot area, while a chart's **decoration** (data labels, per-point formatting,
trendlines, error bars) hangs off the series and inherits down three tiers.

## The furniture

| Wire | Type | What it answers |
|---|---|---|
| `c:catAx`, `c:valAx`, `c:dateAx`, `c:serAx` | [`Axis`](crate::Axis) | one axis, whichever of the four kinds ([`AxisKind`](crate::AxisKind)) |
| `c:scaling` | [`Scaling`](crate::Scaling) | its minimum, maximum, log base, and [`AxisOrientation`](crate::AxisOrientation) |
| `c:majorGridlines`, `c:minorGridlines` | [`Gridlines`](crate::Gridlines) | whether the axis draws them |
| `c:title` | [`ChartTitle`](crate::ChartTitle), [`TitleText`](crate::TitleText) | the chart's own title, or an axis' |
| `c:legend` | [`Legend`](crate::Legend), [`LegendPosition`](crate::LegendPosition) | where the key sits |
| `c:dispBlanksAs` | [`BlankDisplay`](crate::BlankDisplay) | what a gap in the data draws as |

[`AxisPosition`](crate::AxisPosition), [`TickMark`](crate::TickMark) and
[`TickLabelPosition`](crate::TickLabelPosition) are the remaining scalars, each an enumeration whose
variants carry their exact wire token in their own docs.

The read side is `mjx_chart::chart_ops::axes`, `legend` and `title`; the write side is
`set_axis_scale`, `set_axis_orientation`, `set_axis_gridlines`, `set_axis_title`, `set_legend` and
`set_title`. Both sides are stated once here and called by all three format crates, which is the
whole point of the module — see [Reading a chart](reading_a_chart)'s last section.

## Decoration inherits over three tiers, per setting

Data labels are the one part of a chart where the schema puts the same settings in three places:

```text
c:dLbls on the plot     ← the default for every series in it
  c:dLbls on the series ← overrides, for that series
    c:dLbl for a point  ← overrides again, for that one point
```

[`DataLabelSettings::inherit`](crate::DataLabelSettings::inherit) resolves that, **merging per
setting rather than per tier**: a series that sets only *show value* keeps its plot's position, its
plot's separator and its plot's number format. A tier that suppresses its label with `c:delete`
short-circuits and inherits nothing at all, because `CT_DLbls` and `CT_DLbl` put `c:delete` and the
settings group in one `xsd:choice` — an element carrying one cannot carry the other, so there is
nothing to merge.

[`ChartLabelScope`](crate::ChartLabelScope) is how a caller names which tier it means, and
`mjx_chart::chart_ops::data_label_tier` reads one tier without resolving, which is the call to use
when the question is *what does this element actually say* rather than *what will be drawn*.

## Writing decoration goes through the plot that holds the series

[`SeriesDecoration`](crate::SeriesDecoration) binds a series to the
[`ChartKind`](crate::ChartKind) of the plot it belongs to, and every decoration write goes through
it. That is not ceremony — **it is where the schema's own refusals come from**:

* `CT_PieSer` declares no `c:trendline` and no `c:errBars`, so asking a pie series for either is
  [`ChartDataError::DecorationNotAllowed`](crate::ChartDataError::DecorationNotAllowed), naming the
  plot, the child and the XSD symbol of the type that declares none.
* `CT_SurfaceSer` declares no decoration at all, and `CT_SurfaceChart` no plot-level `c:dLbls`.
* `CT_BarSer` and `CT_PieSer` place `c:dPt` at **different ranks** in their sequences, so where a new
  `c:dPt` is inserted depends on the plot kind too.

Both the placement and the refusal therefore follow from the generated child-order tables rather than
from a list written in this crate — which is what stops the two from drifting apart when a schema
detail is looked at again.

The four decoration types are [`DataLabels`](crate::DataLabels) / [`DataLabel`](crate::DataLabel),
[`DataPointFormat`](crate::DataPointFormat) (`c:dPt` — one point's own fill, outline and explosion),
[`Trendline`](crate::Trendline) and [`ErrorBars`](crate::ErrorBars). Their describing values are
[`DataLabelSpec`](crate::DataLabelSpec), [`TrendlineSpec`](crate::TrendlineSpec) and
[`ErrorBarSpec`](crate::ErrorBarSpec), and each of those validates on the way in: a polynomial
trendline's order outside `ST_Order`'s 2–6, a moving average's period below `ST_Period`'s 2, custom
error bars with neither `c:plus` nor `c:minus`, and a non-finite `f64` — which has no `xsd:double`
spelling OOXML admits — are each their own [`ChartDataError`](crate::ChartDataError) variant rather
than markup that fails validation later.

## Decoration that points at a point which is not there

`c:dPt` and `c:dLbl` are anchored by `c:idx` into the series. A file can carry an index past the end
of its own data — deleting points does not renumber the overrides — and this crate neither follows
such a reference nor deletes it behind your back:

* `mjx_chart::chart_ops::dangling_decoration` reports every one, as
  [`DanglingPointReference`](crate::DanglingPointReference).
* `mjx_chart::chart_ops::drop_dangling_decoration` removes them, when the caller asks.

A per-point *edit* naming a missing point is refused before anything is written, as
[`ChartDataError::DataPointOutOfRange`](crate::ChartDataError::DataPointOutOfRange). That check is
also what stops a hostile `c:idx` in an untrusted file from being copied into markup this library
authors.

## The series' own look

[`SeriesShapeProperties`](crate::SeriesShapeProperties) is the series' `c:spPr`, declared by
`dml-chart.xsd` as DrawingML's own `a:CT_ShapeProperties`. The element is kept **opaque and re-emitted
verbatim** — the same treatment `mjx-pptx` gives a shape's `p:spPr` — and the accessors read and
rewrite only the one fill and the one outline it may declare, as
[`mjx_dml::FillSpec`](mjx_dml::FillSpec) and [`mjx_dml::LineSpec`](mjx_dml::LineSpec), leaving its
geometry, its effects, its 3-D and its extensions untouched. A new fill or outline is inserted at its
own rank in `CT_ShapeProperties`' sequence rather than appended.
`mjx_chart::chart_ops::series_fill`, `set_series_fill`, `set_series_line`, `set_point_fill` and
`set_point_line` are the surface.

**Nothing here authors an `spPr` a caller did not ask for.** A series with no `c:spPr` keeps none, and
takes its colour from the document's theme through the chart style — which is the standing rule
(*where OOXML lets a value inherit, let it inherit*) applied to the one place in a chart where getting
it wrong would override the branding of whoever opens the file.
