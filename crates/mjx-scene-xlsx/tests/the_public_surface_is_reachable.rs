//! Every name this crate exports is reachable from outside it, and every one of them answers.
//!
//! A `pub use` that names a type nobody can construct or inspect from outside is a surface in name
//! only, and the compiler is perfectly happy with one. So this file names every export, uses it, and
//! reads something back.
//!
//! It is also where the **nineteen-value pattern table** is exercised end to end: `pattern_preset`
//! is a translation between two vocabularies that are not the same list, and a table nobody calls
//! for seventeen of its arms is a table nobody has checked.

mod support;

use mjx_layout::{DecorationRef, SourcePath, SourceRef};
use mjx_layout_xlsx::{CellFill, CellGradient, CellGradientStop, PageCatalogue};
use mjx_ooxml_types::spreadsheetml::{GradientType, PatternType};
use mjx_scene::{FillStyle, GeometryProvider, PatternPreset, ResourceResolver, SceneRect};
use mjx_scene_xlsx::{fill_style, pattern_preset, SheetGeometry, SheetPalette, SystemRole};
use mjx_sml::Color;

use support::{resolve, styles, viewport, workbook, worksheet};

const SHEET: &str = r#"<dimension ref="A1:A1"/>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#;

#[test]
fn the_palette_answers_every_spelling_of_a_colour() {
    let palette = SheetPalette::default();
    assert!(palette
        .theme()
        .rgb(mjx_dml::ColorSchemeSlot::Accent1)
        .is_none());
    assert!(!palette.indexed().is_empty());

    // `@rgb`, eight digits, alpha first.
    let stated = palette
        .resolve(&Color::from_opaque_rgb("112233"), SystemRole::Foreground)
        .expect("an `@rgb` colour resolves");
    assert_eq!(
        (stated.red, stated.green, stated.blue, stated.alpha),
        (0x11, 0x22, 0x33, 0xff)
    );

    // `@theme`, against a scheme that defines nothing: no colour, rather than an invented one.
    assert!(palette
        .resolve(&Color::from_theme(4, None), SystemRole::Foreground)
        .is_none());

    // `@indexed`, drawn opaque — see `mjx_scene_xlsx::colour` for why the palette's own `00` is
    // not an opacity.
    let indexed = palette
        .resolve(
            &Color {
                indexed: Some(2),
                ..Color::default()
            },
            SystemRole::Foreground,
        )
        .expect("row 2 of the default palette is red");
    assert_eq!((indexed.red, indexed.alpha), (0xff, 0xff));

    // `@auto`, and the two system rows, each answering the role they were asked for.
    let automatic = Color {
        automatic: Some(true),
        ..Color::default()
    };
    assert_eq!(
        palette.resolve(&automatic, SystemRole::Background),
        Some(palette.system(SystemRole::Background))
    );
    assert_ne!(
        palette.system(SystemRole::Foreground),
        palette.system(SystemRole::Background)
    );
    let system_row = Color {
        indexed: Some(64),
        ..Color::default()
    };
    assert_eq!(
        palette.resolve(&system_row, SystemRole::Foreground),
        Some(palette.system(SystemRole::Foreground))
    );

    // A colour that says nothing at all.
    assert!(palette
        .resolve(&Color::default(), SystemRole::Foreground)
        .is_none());
    assert_eq!(
        palette.resolve_or_system(None, SystemRole::Foreground),
        palette.system(SystemRole::Foreground)
    );

    // The shell's own system colours, rather than ours imposed on it.
    let dark = SheetPalette::default().with_system_colours(
        mjx_scene::Color {
            red: 0xee,
            green: 0xee,
            blue: 0xee,
            alpha: 0xff,
        },
        mjx_scene::Color {
            red: 0x1e,
            green: 0x1e,
            blue: 0x1e,
            alpha: 0xff,
        },
    );
    assert_eq!(dark.system(SystemRole::Foreground).red, 0xee);
    let _ = SheetPalette::new(
        mjx_dml::SchemeColors::default(),
        mjx_sml::IndexedColorPalette::default_palette(),
    );
}

#[test]
fn every_pattern_the_schema_declares_is_translated() {
    // The two that are not hatches at all.
    assert!(pattern_preset(PatternType::None).is_none());
    assert!(pattern_preset(PatternType::Solid).is_none());

    const HATCHES: &[PatternType] = &[
        PatternType::MediumGray,
        PatternType::DarkGray,
        PatternType::LightGray,
        PatternType::DarkHorizontal,
        PatternType::DarkVertical,
        PatternType::DarkDown,
        PatternType::DarkUp,
        PatternType::DarkGrid,
        PatternType::DarkTrellis,
        PatternType::LightHorizontal,
        PatternType::LightVertical,
        PatternType::LightDown,
        PatternType::LightUp,
        PatternType::LightGrid,
        PatternType::LightTrellis,
        PatternType::Gray12Point5Percent,
        PatternType::Gray6Point25Percent,
    ];
    assert_eq!(
        HATCHES.len(),
        17,
        "`ST_PatternValues` declares nineteen values, of which seventeen are hatches"
    );
    for &hatch in HATCHES {
        assert!(
            pattern_preset(hatch).is_some(),
            "{hatch:?} translated to nothing, so a cell hatched with it would paint no marks at all"
        );
    }

    // The two greys that are not exact twins, so the GUESS is exercised rather than described.
    assert_eq!(
        pattern_preset(PatternType::Gray12Point5Percent),
        Some(PatternPreset::Percent10)
    );
    assert_eq!(
        pattern_preset(PatternType::MediumGray),
        Some(PatternPreset::Percent50)
    );
    // The stated loss: two SpreadsheetML trellises, one DrawingML trellis.
    assert_eq!(
        pattern_preset(PatternType::DarkTrellis),
        pattern_preset(PatternType::LightTrellis)
    );
}

#[test]
fn a_fill_translates_in_every_one_of_its_three_shapes() {
    let palette = SheetPalette::default();

    // `patternType="none"` paints nothing, which is not the same as painting white.
    assert_eq!(
        fill_style(
            &CellFill {
                pattern: Some(PatternType::None),
                foreground: None,
                background: None,
                gradient: None,
            },
            &palette
        ),
        FillStyle::None
    );

    // `patternType="solid"` paints the **foreground**.
    let solid = fill_style(
        &CellFill {
            pattern: Some(PatternType::Solid),
            foreground: Some(Color::from_opaque_rgb("00FF00")),
            background: Some(Color::from_opaque_rgb("FF0000")),
            gradient: None,
        },
        &palette,
    );
    let FillStyle::Solid(colour) = solid else {
        panic!("a solid fill resolved to {solid:?}");
    };
    assert_eq!(
        (colour.red, colour.green, colour.blue),
        (0x00, 0xff, 0x00),
        "`patternType=\"solid\"` paints its `fgColor`; reading `bgColor` there gives {colour:?}"
    );

    // A hatch, with both of its colours.
    let hatched = fill_style(
        &CellFill {
            pattern: Some(PatternType::DarkTrellis),
            foreground: Some(Color::from_opaque_rgb("FFFF00")),
            background: None,
            gradient: None,
        },
        &palette,
    );
    assert!(matches!(
        hatched,
        FillStyle::Pattern {
            preset: PatternPreset::Trellis,
            ..
        }
    ));

    // A `dxf`'s third state: a colour and no `@patternType` at all.
    let differential = fill_style(
        &CellFill {
            pattern: None,
            foreground: None,
            background: Some(Color::from_opaque_rgb("FFC7CE")),
            gradient: None,
        },
        &palette,
    );
    assert!(matches!(differential, FillStyle::Solid(_)));

    // A gradient, in both of its kinds.
    let ramp = fill_style(
        &CellFill {
            pattern: None,
            foreground: None,
            background: None,
            gradient: Some(CellGradient {
                kind: GradientType::Linear,
                degrees: 90.0,
                inset: [0.0; 4],
                stops: vec![
                    CellGradientStop {
                        position: 0.0,
                        colour: Some(Color::from_opaque_rgb("000000")),
                    },
                    CellGradientStop {
                        position: 1.0,
                        colour: Some(Color::from_opaque_rgb("FFFFFF")),
                    },
                ],
            }),
        },
        &palette,
    );
    let FillStyle::Gradient(gradient) = ramp else {
        panic!("a gradient resolved to {ramp:?}");
    };
    assert_eq!(gradient.stops.len(), 2);
    assert!((gradient.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);

    // A gradient whose every stop is unreadable is not a gradient.
    assert_eq!(
        fill_style(
            &CellFill {
                pattern: None,
                foreground: None,
                background: None,
                gradient: Some(CellGradient {
                    kind: GradientType::Path,
                    degrees: 0.0,
                    inset: [0.1, 0.9, 0.1, 0.9],
                    stops: vec![CellGradientStop {
                        position: 0.0,
                        colour: Some(Color::from_theme(3, None)),
                    }],
                }),
            },
            &palette
        ),
        FillStyle::None
    );
}

#[test]
fn the_resolver_answers_from_the_catalogue_it_was_built_with() {
    let mut book = workbook(&worksheet(SHEET), &styles("", "", "", &[]));
    let constraints = viewport(4.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);

    let catalogue: &PageCatalogue = resolved.resources.catalogue();
    assert!(catalogue.decoration_count() > 0);
    let _ = resolved.resources.palette();

    // A handle no catalogue issued answers nothing rather than panicking.
    assert!(resolved
        .resources
        .decoration(DecorationRef::new(u64::MAX))
        .is_none());

    // An address that names no cell — the page's own root path — answers nothing.
    let root = SourceRef::node(mjx_layout::PartId::new(0), SourcePath::root());
    assert!(resolved.resources.text_decoration(&root).is_none());

    // A worksheet issues no image handle at all.
    assert!(resolved
        .resources
        .image(mjx_layout::ImageRef::new(0))
        .is_none());

    // A1 holds text, so it has a report and therefore a decoration.
    assert!(resolved.resources.cell_decoration(0, 0).is_some());
    assert!(
        resolved.resources.cell_decoration(900, 900).is_none(),
        "a cell that laid no text out has no entry, which is what makes the table exactly the set \
         of addresses `text_decoration` can be asked about"
    );
}

#[test]
fn the_geometry_provider_refuses_a_handle_a_worksheet_never_issues() {
    let geometry = SheetGeometry::new();
    assert_eq!(geometry.registered(), 0);
    assert_eq!(geometry.unregistered(), 0);
    let refused = geometry.outline(7, SceneRect::new(0.0, 0.0, 10.0, 10.0));
    assert!(
        matches!(
            refused,
            Err(mjx_scene::SceneError::UnresolvedOutline { outline: 7 })
        ),
        "the provider answered {refused:?}. A stand-in here would turn a handle nobody issued — \
         which can only be a defect — into a shape a reader reports as a rendering bug."
    );
    assert_eq!(SheetGeometry, geometry);
}
