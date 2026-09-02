//! The binding, exercised **inside a WebAssembly runtime**.
//!
//! ```sh
//! wasm-pack test --node    bindings/mjx-wasm     # what a developer runs
//! wasm-pack test --headless --chrome bindings/mjx-wasm   # what CI runs
//! ```
//!
//! The Node tests under `tests/node/` drive the *published shape* — the generated JavaScript glue,
//! the `.d.ts`, the conditional exports. These drive the *Rust side* in the same runtime the
//! published package runs in, which is the only place three things can be checked at all:
//!
//! * that the conversions work against a real `JsValue` — `as_f64`, `Reflect::get`, `js_sys::Array`
//!   — rather than against the host stubs a `cargo test` would link;
//! * that a failure really arrives as a `js_sys::Error` with the properties this crate sets; and
//! * that nothing panics, which in wasm is not a recoverable error but a dead module.
//!
//! There is no `#[wasm_bindgen_test_configure]` here on purpose, so the same tests run under Node
//! and under a browser. A browser is not a different library — but it is a different runtime, and
//! the point of running there is to prove that.

#![allow(unsafe_code)]

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test;

use mjx_wasm::deck::Deck;
use mjx_wasm::enums::{ChartKind, PresetShapeType, ShapeKind};
use mjx_wasm::format::{detect_format, Format};
use mjx_wasm::geometry::{ShapeBounds, SlideSize};
use mjx_wasm::paint::{ColorSpec, FillSpec};

/// A surface argument, from a plain number — the spelling every example uses.
fn surface(index: u32) -> mjx_wasm::address::SurfaceArg {
    JsValue::from_f64(f64::from(index)).unchecked_into()
}

/// A shape argument, from a plain number.
fn shape(index: u32) -> mjx_wasm::address::ShapePathArg {
    JsValue::from_f64(f64::from(index)).unchecked_into()
}

/// One property of a JavaScript object, as a string.
fn property(value: &JsValue, name: &str) -> Option<String> {
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|found| found.as_string())
}

#[wasm_bindgen_test]
fn a_blank_deck_is_built_edited_saved_and_reopened() {
    let mut deck = Deck::blank(&SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    let title = deck
        .add_text_box(
            &surface(slide),
            "Quarterly results",
            &ShapeBounds::from_inches(0.5, 0.4, 9.0, 1.2),
        )
        .expect("a text box");
    deck.set_shape_fill(
        &surface(slide),
        &shape(title),
        &FillSpec::solid(&ColorSpec::srgb("1F3864")),
    )
    .expect("a fill");

    deck.validate().expect("the deck validates");
    let saved = deck.save().expect("the deck saves");
    assert_eq!(
        &saved[..2],
        b"PK",
        "a package starts with the ZIP signature"
    );

    let mut reopened = Deck::open(&saved).expect("it reopens");
    assert_eq!(reopened.slide_count(), 1);
    assert_eq!(
        reopened
            .shape_text(&surface(slide), &shape(title))
            .expect("the text"),
        "Quarterly results"
    );
    assert_eq!(
        reopened
            .shape_kind(&surface(slide), &shape(title))
            .expect("the kind"),
        ShapeKind::Shape
    );
}

#[wasm_bindgen_test]
fn detection_reads_the_package_rather_than_a_name() {
    let mut deck = Deck::blank(&SlideSize::widescreen()).expect("a blank deck");
    deck.add_slide().expect("a slide");
    let saved = deck.save().expect("the deck saves");
    assert_eq!(
        detect_format(&saved).expect("detection"),
        Format::Presentation
    );
    assert_eq!(deck.format().expect("the format"), Format::Presentation);
}

#[wasm_bindgen_test]
fn a_failure_arrives_as_an_error_with_a_code_and_coordinates() {
    let mut deck = Deck::blank(&SlideSize::widescreen()).expect("a blank deck");
    deck.add_slide().expect("a slide");

    let raised = deck
        .shape_text(&surface(0), &shape(99))
        .expect_err("shape 99 is out of range");
    assert!(raised.is_instance_of::<js_sys::Error>(), "a real Error");
    assert_eq!(property(&raised, "name").as_deref(), Some("OoxmlError"));
    assert_eq!(
        property(&raised, "code").as_deref(),
        Some("IndexOutOfRange")
    );

    let detail = js_sys::Reflect::get(&raised, &JsValue::from_str("detail")).expect("detail");
    let surface_detail =
        js_sys::Reflect::get(&detail, &JsValue::from_str("surface")).expect("a surface");
    assert!(
        !surface_detail.is_undefined(),
        "the failure names its surface"
    );
    let row = js_sys::Reflect::get(&detail, &JsValue::from_str("row")).expect("row");
    assert!(
        row.is_undefined(),
        "a coordinate it did not carry is absent"
    );
}

#[wasm_bindgen_test]
fn a_number_and_a_class_address_the_same_shape() {
    let mut deck = Deck::blank(&SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    deck.add_shape(
        &surface(slide),
        PresetShapeType::Ellipse,
        &ShapeBounds::from_inches(1.0, 1.0, 1.0, 1.0),
    )
    .expect("a shape");

    let by_number = deck.shape_count(&surface(0)).expect("by number");
    let named: mjx_wasm::address::SurfaceArg =
        JsValue::from(mjx_wasm::address::Surface::slide(0)).unchecked_into();
    let by_class = deck.shape_count(&named).expect("by class");
    let again = deck
        .shape_count(&named)
        .expect("the class survives being read");
    assert_eq!(by_number, 1);
    assert_eq!(by_class, 1);
    assert_eq!(again, 1);
}

#[wasm_bindgen_test]
fn a_bad_address_is_refused_rather_than_truncated() {
    let mut deck = Deck::blank(&SlideSize::widescreen()).expect("a blank deck");
    deck.add_slide().expect("a slide");

    let fractional: mjx_wasm::address::SurfaceArg = JsValue::from_f64(1.5).unchecked_into();
    let refused = deck
        .shape_count(&fractional)
        .expect_err("1.5 is not an index");
    assert!(
        property(&refused, "message")
            .unwrap_or_default()
            .contains("whole number"),
        "the message says what an index is"
    );

    let negative: mjx_wasm::address::SurfaceArg = JsValue::from_f64(-1.0).unchecked_into();
    assert!(deck.shape_count(&negative).is_err(), "-1 is not an index");

    let text: mjx_wasm::address::SurfaceArg = JsValue::from_str("nonsense").unchecked_into();
    assert!(
        deck.shape_count(&text).is_err(),
        "a string is not a surface"
    );
}

#[wasm_bindgen_test]
fn a_chart_survives_the_round_trip() {
    let mut deck = Deck::blank(&SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    let chart = mjx_wasm::charts::ChartData::new(ChartKind::Bar)
        .categories(vec!["Q1".to_owned(), "Q2".to_owned()])
        .series("2026", vec![12.0, 15.5]);
    let frame = deck
        .add_chart(
            &surface(slide),
            &chart,
            &ShapeBounds::from_inches(1.0, 1.0, 8.0, 4.0),
        )
        .expect("a chart");

    let saved = deck.save().expect("the deck saves");
    let mut reopened = Deck::open(&saved).expect("it reopens");
    let series = reopened
        .chart_series(&surface(slide), &shape(frame))
        .expect("the series");
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].name().as_deref(), Some("2026"));
    assert_eq!(series[0].values(), vec![12.0, 15.5]);
}
