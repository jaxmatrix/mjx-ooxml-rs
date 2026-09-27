//! The three hand-entered tables in this crate, checked against the **relationships** that define
//! them rather than against copies of themselves.
//!
//! # Why a table needs a gate at all
//!
//! A table asserted against a second copy of itself passes for ever and proves nothing. So each of
//! the three is checked against something it is *derived from*:
//!
//! * the **fifty-four preset hatches** — the twelve percentages against the coverage their own names
//!   state, and the whole table against distinctness, because a copy-paste in a fifty-four-entry
//!   array is invisible by inspection;
//! * the **gradient ramp** — against premultiplication, monotonicity and the dark-halo bug that
//!   interpolating non-premultiplied colours produces;
//! * the **paint kinds** — against the shader's own constants, read out of the very `.wgsl` file
//!   that is compiled, because a painter and its shader that disagreed about what `3` means would
//!   draw a picture where a hatch belongs and nothing would fail to compile.
//!
//! Every case here is arithmetic and file reading. None of it needs a graphics device.

use mjx_paint::{
    backend::{PaintKind, SHADER_SOURCE},
    cell_of, coverage_atlas, mask_of, GradientRamp, PATTERN_MASKS, PATTERN_SIDE, RAMP_TEXELS,
};
use mjx_scene::{Color, GradientStop, PatternPreset, PATTERN_PRESET_COUNT};

#[test]
fn the_dither_matrix_is_a_permutation_and_the_percentages_match_their_names() {
    // The hand-entered part of the percentage family is the Bayer matrix; everything else about
    // those twelve is derived from it. A typo in one cell would silently change one preset's density
    // and leave every other property intact, so the permutation is what is asserted.
    let mut seen = [false; 64];
    for row in mjx_paint::pattern::BAYER_8X8 {
        for value in row {
            let index = usize::from(value);
            assert!(
                index < 64,
                "the matrix holds {value}, which is outside 0..64"
            );
            assert!(!seen[index], "{value} appears twice in the dither matrix");
            seen[index] = true;
        }
    }
    assert!(
        seen.iter().all(|found| *found),
        "the dither matrix is not a permutation of 0..64, so the coverages below are not the \
         coverages the names state"
    );

    // And the relationship the derivation buys: `pct25` sets exactly a quarter of the cells.
    for (position, percent) in mjx_paint::pattern::PERCENT_COVERAGE.iter().enumerate() {
        let preset = PatternPreset::ALL
            .get(position)
            .copied()
            .expect("the twelve percentages come first in wire order");
        let set: u32 = mask_of(preset).iter().map(|row| row.count_ones()).sum();
        let expected = mjx_paint::pattern::cells_for_percent(*percent);
        assert_eq!(
            set, expected,
            "{preset:?} claims {percent}% and covers {set} of 64 cells, where {expected} was the \
             coverage its own name states"
        );
    }
}

#[test]
fn every_preset_has_its_own_mask_and_none_of_them_is_empty_or_solid() {
    assert_eq!(PATTERN_MASKS.len(), PATTERN_PRESET_COUNT as usize);

    // Distinctness. A copy-paste in a fifty-four-entry array of binary literals is invisible to a
    // reader and produces two presets that draw the same picture — which is exactly the failure
    // `mjx-scene` refuses when it says "a painter that mapped an unknown preset onto a near-enough
    // one would draw the wrong page silently".
    for (position, preset) in PatternPreset::ALL.iter().enumerate() {
        let mask = mask_of(*preset);
        let coverage: u32 = mask.iter().map(|row| row.count_ones()).sum();
        assert!(
            coverage > 0,
            "{preset:?} covers nothing, so a shape hatched with it is drawn in its background \
             colour and the hatch is invisible"
        );
        assert!(
            coverage < 64,
            "{preset:?} covers every cell, so a shape hatched with it is drawn in its foreground \
             colour and the hatch is invisible the other way"
        );
        for (other_position, other) in PatternPreset::ALL.iter().enumerate() {
            if other_position <= position {
                continue;
            }
            assert_ne!(
                mask,
                mask_of(*other),
                "{preset:?} and {other:?} are the same eight-by-eight mask; two presets that draw \
                 the same picture is a copy-paste nobody can see by reading"
            );
        }
    }
}

#[test]
fn the_coverage_atlas_is_the_mask_table_and_the_cell_lookup_agrees_with_it() {
    let atlas = coverage_atlas();
    assert_eq!(
        atlas.len(),
        PATTERN_MASKS.len() * PATTERN_SIDE * PATTERN_SIDE,
        "the texture the shader samples must hold every cell of every preset"
    );

    // The texture, the mask table and the cell accessor are three views of one thing, and the
    // painter uses all three: the first is uploaded, the second is the source, the third is what a
    // software painter in R09 will read. They have to agree.
    for (position, preset) in PatternPreset::ALL.iter().enumerate() {
        for y in 0..PATTERN_SIDE {
            for x in 0..PATTERN_SIDE {
                let from_atlas = atlas
                    .get((position * PATTERN_SIDE + y) * PATTERN_SIDE + x)
                    .copied()
                    .expect("every cell is in the atlas");
                let from_cell = cell_of(*preset, x, y);
                assert_eq!(
                    from_atlas != 0,
                    from_cell,
                    "{preset:?} cell ({x}, {y}): the uploaded texture says {from_atlas} and the \
                     accessor says {from_cell}"
                );
            }
        }
    }

    // And the tile repeats, which is what makes it a hatch rather than a stamp.
    let preset = PatternPreset::DiagonalCross;
    assert_eq!(cell_of(preset, 0, 0), cell_of(preset, 8, 8));
    assert_eq!(cell_of(preset, 3, 5), cell_of(preset, 11, 13));
}

#[test]
fn a_gradient_ramp_is_premultiplied_monotonic_and_free_of_the_dark_halo() {
    // A ramp from opaque red to fully transparent. Interpolating **non**-premultiplied colours
    // across this is the classic dark-halo bug: the midpoint becomes half-alpha *dark* red, which
    // reads as a grey edge. Interpolating premultiplied ones does not, and this is the assertion
    // that says which was done.
    let red = Color {
        red: 0xff,
        green: 0,
        blue: 0,
        alpha: 0xff,
    };
    let clear = Color {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    };
    let ramp =
        GradientRamp::from_stops(&[GradientStop::new(0.0, red), GradientStop::new(1.0, clear)]);
    assert_eq!(ramp.texels().len(), RAMP_TEXELS * 4);

    let start = ramp.texel(0).expect("the first texel");
    let middle = ramp.texel(RAMP_TEXELS / 2).expect("the middle texel");
    let end = ramp.texel(RAMP_TEXELS - 1).expect("the last texel");
    assert_eq!(start, [0xff, 0, 0, 0xff]);
    assert_eq!(end, [0, 0, 0, 0]);
    assert!(
        (120..=136).contains(&middle[3]),
        "the midpoint's alpha is {}, not about half",
        middle[3]
    );
    assert_eq!(
        middle[0], middle[3],
        "premultiplied red at alpha a has red exactly a: {middle:?}. Anything else means the \
         interpolation happened before the premultiply, which is the dark halo."
    );

    // Monotonic, and it really moves: a ramp that answered its first stop everywhere would satisfy
    // the endpoints above if the endpoints were the only thing checked.
    let mut previous = 0xffu8;
    let mut distinct = std::collections::BTreeSet::new();
    for index in 0..RAMP_TEXELS {
        let texel = ramp.texel(index).expect("a texel");
        assert!(
            texel[3] <= previous,
            "the ramp's alpha rises at texel {index}: {} after {previous}",
            texel[3]
        );
        previous = texel[3];
        distinct.insert(texel);
    }
    assert!(
        distinct.len() > RAMP_TEXELS / 2,
        "the ramp has {} distinct texels out of {RAMP_TEXELS}, which is a step rather than a ramp",
        distinct.len()
    );
}

#[test]
fn stops_out_of_order_and_a_gradient_with_none_are_both_handled() {
    // `a:gsLst` does not require its stops in order, and a ramp built from an unsorted list runs
    // backwards through part of itself. Real documents contain both this and an empty stop list.
    let black = Color {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0xff,
    };
    let white = Color {
        red: 0xff,
        green: 0xff,
        blue: 0xff,
        alpha: 0xff,
    };
    let forwards =
        GradientRamp::from_stops(&[GradientStop::new(0.0, black), GradientStop::new(1.0, white)]);
    let backwards =
        GradientRamp::from_stops(&[GradientStop::new(1.0, white), GradientStop::new(0.0, black)]);
    assert_eq!(
        forwards, backwards,
        "the same two stops written in the other order produced a different ramp"
    );

    let empty = GradientRamp::from_stops(&[]);
    assert_eq!(
        empty.texel(0),
        Some([0, 0, 0, 0]),
        "a gradient with no stops must resolve to nothing rather than fail the frame: \
         `a:gradFill` with an empty `a:gsLst` is malformed markup that real documents contain"
    );

    // Two stops at the same position are a hard edge, which designers do on purpose — and which is
    // a division by a zero span if it is not handled.
    let hard = GradientRamp::from_stops(&[
        GradientStop::new(0.0, black),
        GradientStop::new(0.5, black),
        GradientStop::new(0.5, white),
        GradientStop::new(1.0, white),
    ]);
    assert_eq!(hard.texel(RAMP_TEXELS / 4), Some([0, 0, 0, 0xff]));
    assert_eq!(
        hard.texel(RAMP_TEXELS * 3 / 4),
        Some([0xff, 0xff, 0xff, 0xff])
    );
}

#[test]
fn every_paint_kind_carries_the_number_its_shader_constant_carries() {
    // The painter writes a number into a uniform and the shader compares against a constant. They
    // are in two files and two languages, nothing links them, and a build in which they disagree
    // compiles perfectly and draws a picture where a hatch belongs. So the constants are read out of
    // the very `.wgsl` that `include_str!` compiles.
    for kind in PaintKind::ALL {
        let name = kind.shader_constant();
        let declaration = format!("const {name}: f32 = ");
        let line = SHADER_SOURCE
            .lines()
            .find(|line| line.trim_start().starts_with(&declaration))
            .unwrap_or_else(|| {
                panic!("the shader declares no `{name}`, which {kind:?} says it compares against")
            });
        let value = line
            .split('=')
            .nth(1)
            .and_then(|rest| rest.trim().trim_end_matches(';').parse::<f32>().ok())
            .unwrap_or_else(|| panic!("`{line}` is not a number the painter can compare against"));
        assert!(
            (value - kind.wire()).abs() < f32::EPSILON,
            "{kind:?} writes {} and the shader's `{name}` is {value}",
            kind.wire()
        );
    }

    // And every kind is distinct, so no two branches can be reached by the same number.
    let mut seen = std::collections::BTreeSet::new();
    for kind in PaintKind::ALL {
        assert!(
            seen.insert(kind.wire().to_bits()),
            "{kind:?} shares a number with another kind"
        );
    }
    assert_eq!(seen.len(), PaintKind::ALL.len());

    // The shader declares no constant the painter does not know about, which is the other direction:
    // a branch nothing can reach is a branch nothing tests.
    let declared = SHADER_SOURCE
        .lines()
        .filter(|line| line.trim_start().starts_with("const KIND_"))
        .count();
    assert_eq!(
        declared,
        PaintKind::ALL.len(),
        "the shader declares {declared} paint kinds and the painter knows {}",
        PaintKind::ALL.len()
    );
}

#[test]
fn the_shader_binds_what_the_painter_binds() {
    // The bind-group layout is built in Rust and consumed by WGSL, and a mismatch is a validation
    // error at pipeline-build time on some backends and a wrong texture on others. Four bindings in
    // group one: two textures and two samplers, and the second sampler exists because a tiled
    // picture must wrap where a gradient ramp must not.
    for binding in [
        "@group(0) @binding(0) var<uniform> draw: Draw;",
        "@group(1) @binding(0) var source: texture_2d<f32>;",
        "@group(1) @binding(1) var second: texture_2d<f32>;",
        "@group(1) @binding(2) var clamped: sampler;",
        "@group(1) @binding(3) var repeating: sampler;",
    ] {
        assert!(
            SHADER_SOURCE.contains(binding),
            "the shader no longer declares `{binding}`, which the painter's bind-group layout does"
        );
    }
    // And the uniform block is the size the painter writes: eleven `vec4`s.
    let vectors = SHADER_SOURCE
        .lines()
        .skip_while(|line| !line.starts_with("struct Draw {"))
        .take_while(|line| !line.starts_with('}'))
        .filter(|line| line.trim_start().contains(": vec4<f32>,"))
        .count();
    assert_eq!(
        vectors, 10,
        "the shader's uniform block holds {vectors} four-component vectors; the painter writes a \
         forty-float block, and a block that is longer than the buffer binding is a validation \
         error rather than a wrong picture"
    );
}
