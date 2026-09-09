//! **No layout path panics**, held two ways: by scanning the source for the constructs that would,
//! and by laying out chart parts that say things no well-formed file says.
//!
//! A chart part comes from an untrusted file, and `CLAUDE.md`'s rule is that library code returns
//! typed errors rather than unwrapping. The scan is the strong half — it catches a panic on a path
//! no fixture happens to reach — and the adversarial parts are the half that proves the scan is
//! measuring something real.
//!
//! # What the scan does not cover, stated rather than implied
//!
//! It does not catch a **slice index**, and this crate has some: the arithmetic in
//! `plot::gaussian_solve` and the graph walks in `diagram` index vectors they allocated a few lines
//! above, at lengths they computed themselves. [`every_index_is_into_a_vector_this_crate_sized`]
//! enumerates those files and says why each is safe, so the claim is a list a reader can check
//! rather than a sentence they have to trust. An index into anything a *file* sized is a `get`, and
//! that is the rule the enumeration exists to keep true.

mod support;

use mjx_layout_chart::{lay_out, ChartModel, ChartPalette, NominalMetrics};

/// The constructs that panic, as they appear in code rather than in prose.
const PANICKING: &[&str] = &[
    ".unwrap()",
    ".expect(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
    "assert!(",
    "assert_eq!(",
];

fn source_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn no_source_line_can_panic() {
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("a readable source file");
        for (number, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for construct in PANICKING {
                if line.contains(construct) {
                    offences.push(format!(
                        "{}:{}: `{construct}`\n    {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a chart part comes from an untrusted file, so no layout path may panic:\n{}",
        offences.join("\n")
    );
}

/// The two files that index a slice, and the reason each is safe.
///
/// This is an *allowlist with reasons*, not a suppression: a third file appearing here has to be
/// argued for in this comment before the test goes green again, which is the point.
///
/// * `plot.rs` — `polynomial_fit` allocates a `width × (width + 1)` matrix and `gaussian_solve`
///   indexes it with `row * stride + column` where every one of `row`, `column` and `offset` is
///   bounded by `width` in the same function. Nothing a file says reaches those bounds: `width` is
///   `order + 1` and `order` is clamped to `2 ..= 6` at the call site.
/// * `diagram.rs` — every `Vec` in `Graph::read` and `hierarchy` is sized from
///   `identifiers.len()` / `graph.nodes.len()` and indexed by a position taken from the same list.
///   The one place a *file's* number is used as an index — a connection's `srcId` and `destId` —
///   goes through `index_of`, which is a `position` returning `Option`.
///
/// * `palette.rs` — `accents[index % accents.len()]` on a fixed array of six. A modulo by a
///   non-zero constant cannot be out of range, and this is the line that makes a seventh series
///   draw in `accent1` again.
///
/// Everything else — a `c:idx`, a point number, a series number, a category count — is read with
/// `get`.
const FILES_THAT_INDEX: &[&str] = &["diagram.rs", "palette.rs", "plot.rs"];

#[test]
fn every_index_is_into_a_vector_this_crate_sized() {
    let mut indexing = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("a readable source file");
        let indexes = text.lines().any(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                return false;
            }
            // `name[expr]` where `expr` starts with a lower-case letter or `*` — a variable index
            // rather than a constant one.
            line.contains('[')
                && line.match_indices('[').any(|(at, _)| {
                    let before = line[..at].chars().next_back();
                    let after = line[at + 1..].chars().next();
                    before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ')')
                        && after.is_some_and(|c| c.is_lowercase() || c == '*')
                })
        });
        if indexes {
            let name = file
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("?")
                .to_owned();
            indexing.push(name);
        }
    }
    indexing.sort();
    indexing.dedup();
    assert_eq!(
        indexing, FILES_THAT_INDEX,
        "the set of files that index a slice changed. Every one has to be argued for in this \
         file's `FILES_THAT_INDEX` comment: an index into anything a *file* sized is a `get`."
    );
}

/// A chart part that says something no well-formed file says, laid out at a normal frame.
fn survives(name: &str, part: &[u8]) {
    match ChartModel::read(part) {
        Ok(model) => {
            let geometry = lay_out(
                &model,
                support::frame(),
                &ChartPalette::OFFICE,
                &mut NominalMetrics,
            );
            // A chart that looks wrong is the correct outcome; one that crashes is not.
            println!("{name}: {} series laid out", geometry.series.len());
        }
        Err(error) => println!("{name}: reported {error}"),
    }
}

#[test]
fn a_series_whose_categories_and_values_disagree_in_length_still_lays_out() {
    survives(
        "four categories and two values",
        &support::bar_chart(
            "clustered",
            &[support::series(
                0,
                "Ragged",
                &["A", "B", "C", "D"],
                &support::values(&[1.0, 2.0]),
            )],
            "",
        ),
    );
}

#[test]
fn an_axis_with_no_scaling_at_all_still_lays_out() {
    // `c:scaling` is required by the schema.
    survives(
        "no scaling",
        br#"<?xml version="1.0"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
        <c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:ser><c:idx val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser></c:barChart>
        <c:valAx><c:axId val="1"/></c:valAx><c:catAx><c:axId val="2"/></c:catAx></c:plotArea></c:chart></c:chartSpace>"#,
    );
}

#[test]
fn a_plot_area_with_no_plot_in_it_still_lays_out() {
    survives(
        "an empty plot area",
        br#"<?xml version="1.0"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
        <c:chart><c:plotArea><c:layout/></c:plotArea></c:chart></c:chartSpace>"#,
    );
}

#[test]
fn a_logarithmic_axis_over_negative_data_still_lays_out() {
    // Every value negative, on an axis that says it is logarithmic. There is no logarithm of −5.
    survives(
        "a log axis over negative data",
        &support::bar_chart(
            "clustered",
            &[support::series(
                0,
                "Below zero",
                &["A", "B"],
                &support::values(&[-5.0, -9.0]),
            )],
            r#"<c:scaling><c:orientation val="minMax"/><c:logBase val="10"/></c:scaling>"#,
        ),
    );
}

#[test]
fn a_gap_width_and_overlap_at_their_extremes_still_lay_out() {
    let part = String::from_utf8(support::bar_chart(
        "clustered",
        &[
            support::series(0, "A", &["X"], &support::values(&[1.0])),
            support::series(1, "B", &["X"], &support::values(&[2.0])),
        ],
        "",
    ))
    .expect("UTF-8")
    .replace(
        r#"<c:varyColors val="0"/>"#,
        r#"<c:varyColors val="0"/><c:gapWidth val="60000"/><c:overlap val="-100"/>"#,
    );
    survives("an absurd gap width", part.as_bytes());
}

#[test]
fn a_trendline_over_one_point_still_lays_out() {
    let part = String::from(
        r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:trendline><c:trendlineType val="poly"/><c:order val="6"/></c:trendline>
        <c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>7</c:v></c:pt></c:numLit></c:val></c:ser>"#,
    );
    survives(
        "a sixth-order polynomial through one point",
        &support::bar_chart("clustered", &[part], ""),
    );
}

#[test]
fn a_diagram_with_one_point_and_no_connections_still_lays_out() {
    // The SmartArt half of the same rule.
    let xml = br#"<?xml version="1.0"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram">
  <dgm:ptLst><dgm:pt modelId="only" type="node"/></dgm:ptLst><dgm:cxnLst/>
</dgm:dataModel>"#;
    let document = mjx_xml::fidelity::parse(xml).expect("well-formed");
    let model = <mjx_dml::diagram::data::DataModel as mjx_ooxml_core::FromXml>::from_xml(
        &document.root,
        &document.interner,
    )
    .expect("a data model");
    let layout =
        mjx_layout_chart::lay_out_diagram(&model, None, &document.interner, support::frame())
            .expect("one point lays out");
    assert_eq!(layout.nodes.len(), 1);
}
