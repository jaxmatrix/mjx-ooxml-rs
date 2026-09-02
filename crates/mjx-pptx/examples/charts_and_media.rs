//! Charts, images, and the discover-then-neutralise pattern for external content.
//!
//! ```sh
//! cargo run -p mjx-pptx --example charts_and_media -- out.pptx
//! ```
//!
//! The second half is the one worth reading twice. A deck that arrives from elsewhere can reference
//! images, media, OLE payloads and chart workbooks that live on a server you cannot reach. This
//! library performs **no network access** — it tells you what is linked and lets you decide. Every
//! external kind follows the same shape: a discovery reader, then a `replace_*_with_placeholder`.

use anyhow::Result;
use mjx_dml::{ColorSpec, FillSpec};
use mjx_pptx::{
    ChartData, ChartKind, ChartLabelScope, DataLabelPosition, DataLabelSpec, ErrorBarSpec,
    ErrorBarType, ErrorValueType, Presentation, ShapeBounds, TrendlineKind, TrendlineSpec,
    DEFAULT_PLACEHOLDER_IMAGE,
};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("charts_and_media.pptx");
    let mut deck = Presentation::open(&support::template()?)?;
    let slide = deck.add_slide_from_layout(2)?;

    // ---- Author a chart --------------------------------------------------------------------
    let data = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2025", [9.0, 11.0, 10.5, 13.0])
        .series("2026", [12.0, 15.5, 14.0, 19.25]);
    let chart = deck.add_chart(slide, &data, ShapeBounds::from_inches(0.5, 1.0, 6.0, 4.0))?;

    println!(
        "chart is shape {chart}: {:?}",
        deck.graphic_frame_kind(slide, chart)?
    );
    for series in deck.chart_series(slide, chart)? {
        println!("  {:?} → {:?}", series.name, series.values);
    }

    // Editing rewrites the cached values, which is what actually renders.
    deck.set_chart_series_values(slide, chart, 0, &[9.5, 11.5, 11.0, 13.5])?;
    println!(
        "  after edit: {:?}",
        deck.chart_series(slide, chart)?[0].values
    );

    // ---- Decorate it: the three label tiers, a highlighted point, a fit and error bars ------
    // The plot tier is the default every series takes...
    deck.set_chart_data_labels(
        slide,
        chart,
        ChartLabelScope::Plot { plot_idx: 0 },
        &DataLabelSpec::new()
            .value(true)
            .position(DataLabelPosition::OutsideEnd)
            .number_format("#,##0.0"),
    )?;
    // ...one series overrides it...
    deck.set_chart_data_labels(
        slide,
        chart,
        ChartLabelScope::Series { series_idx: 1 },
        &DataLabelSpec::new().position(DataLabelPosition::InsideEnd),
    )?;
    // ...and one point of that series overrides that.
    deck.set_chart_data_labels(
        slide,
        chart,
        ChartLabelScope::Point {
            series_idx: 1,
            point_idx: 3,
        },
        &DataLabelSpec::new().category_name(true),
    )?;

    let resolved = deck.chart_data_labels(slide, chart, 1, Some(3))?;
    println!(
        "\nlabel in force for series 1, point 3: value={:?} category={:?} position={:?} format={:?}",
        resolved.shows_value,
        resolved.shows_category_name,
        resolved.position,
        resolved.number_format,
    );
    println!(
        "  the value comes from the plot tier, the position from the series tier, the category name \
         from the point"
    );

    // One column in its own colour, addressed by index into the series.
    deck.set_chart_point_fill(
        slide,
        chart,
        1,
        3,
        &FillSpec::Solid(ColorSpec::Srgb("C00000".to_owned())),
    )?;
    deck.add_chart_trendline(
        slide,
        chart,
        1,
        &TrendlineSpec::new(TrendlineKind::Linear).display(true, true),
    )?;
    deck.set_chart_error_bars(
        slide,
        chart,
        1,
        &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::Percentage, 5.0),
    )?;
    println!(
        "  {} point format(s), {} trendline(s), {} error bar set(s)",
        deck.chart_point_formats(slide, chart, 1)?.len(),
        deck.chart_trendlines(slide, chart, 1)?.len(),
        deck.chart_error_bars(slide, chart, 1)?.len(),
    );

    // Shortening the series leaves the `c:dPt` anchored to point 3 saying *point 3* — renumbering it
    // would move the highlight silently onto a different column. The now-dangling anchors are
    // reported, and dropped only because this example asks.
    deck.set_chart_series_values(slide, chart, 1, &[12.0, 15.5])?;
    let dangling = deck.chart_dangling_decoration(slide, chart, 1)?;
    println!("  after shortening series 1: {dangling:?}");
    let dropped = deck.drop_chart_dangling_decoration(slide, chart, 1)?;
    println!("  dropped {dropped} anchor(s) that named a point the series no longer has");

    // ---- An image, added twice -------------------------------------------------------------
    // Media parts deduplicate by content, so the same logo on twenty slides costs one part.
    let first = deck.add_image(slide, DEFAULT_PLACEHOLDER_IMAGE)?;
    let second = deck.add_image(slide, DEFAULT_PLACEHOLDER_IMAGE)?;
    println!(
        "\nsame bytes twice → rel {first} and {second} (deduplicated: {})",
        first == second
    );

    let picture = deck.add_picture(
        slide,
        DEFAULT_PLACEHOLDER_IMAGE,
        ShapeBounds::from_inches(7.0, 1.0, 2.0, 2.0),
    )?;
    if let Some(bytes) = deck.picture_image_bytes(slide, picture)? {
        println!("picture holds {} bytes", bytes.len());
    }

    // ---- Discover what points outside the package ------------------------------------------
    // On this template there is nothing external to find, which is the point: the readers answer
    // honestly rather than guessing, and cost nothing when a deck is self-contained.
    let mut external = 0usize;

    for linked in deck.linked_images(slide)? {
        println!(
            "linked image on shape {} → {}",
            linked.shape_index, linked.target
        );
        // Passing `None` embeds a neutral built-in placeholder; pass your own bytes instead.
        deck.replace_linked_image_with_placeholder(slide, linked.shape_index, None)?;
        external += 1;
    }
    for media in deck.media_references(slide)? {
        println!("{:?} media {} → {}", media.kind, media.rel_id, media.target);
        if media.external {
            deck.replace_media_with_placeholder(slide, &media.rel_id, None)?;
            external += 1;
        }
    }
    for workbook in deck.chart_workbooks(slide)? {
        println!(
            "chart on shape {} → workbook {}",
            workbook.shape_index, workbook.target
        );
        if workbook.external {
            // A chart renders from its cache, so detaching an unreachable workbook loses nothing.
            deck.detach_chart_workbook(slide, workbook.shape_index)?;
            external += 1;
        }
    }
    for ole in deck.ole_objects(slide)? {
        println!("OLE {:?} on shape {}", ole.prog_id, ole.shape_index);
        if ole.external {
            deck.replace_ole_object_with_placeholder(slide, ole.shape_index, None)?;
            external += 1;
        }
    }
    println!("neutralised {external} external reference(s)");

    let bytes = deck.save()?;
    std::fs::write(&out, &bytes)?;

    let mut reopened = Presentation::open(&bytes)?;
    anyhow::ensure!(reopened.chart_series(slide, chart)?.len() == 2);
    anyhow::ensure!(reopened.picture_image_bytes(slide, picture)?.is_some());
    println!("wrote {} and verified", out.display());

    Ok(())
}
