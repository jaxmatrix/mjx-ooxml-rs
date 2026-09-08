//! The fifty-four hatch tokens: real, distinct, complete — and **in the same order as the mask
//! table they will be compared against.**
//!
//! # Why this crate carries an ordered token list at all
//!
//! `mjx_paint::PATTERN_MASKS` is indexed by a preset's position in `ST_PresetPatternVal`.
//! `mjx_scene::PatternPreset` carries that position as a *number*, because a display list holds no
//! strings. `mjx_ooxml_types::PatternType` carries the *token* and can parse one, but the generated
//! enumerations in this workspace expose no `ALL` — so nothing in the workspace can walk the
//! fifty-four tokens in order, and a swatch deck has to name each preset in a document.
//!
//! A hand-written list is therefore unavoidable, and the whole question is what stops it drifting.
//! Three things do, and only the third checks the **order**:
//!
//! 1. [`every_token_is_a_real_preset_pattern`] — each round-trips through `PatternType::from_wire`
//!    and `to_wire`, so a typo cannot survive.
//! 2. [`there_are_fifty_four_of_them_and_no_two_are_the_same`] — the count is
//!    `PATTERN_PRESET_COUNT`, taken from `mjx-scene` rather than written here, and the set is
//!    distinct. A list with `pct25` twice and `pct30` missing would pass the first case.
//! 3. [`the_twelve_percentages_land_where_their_own_coverage_says`] — the one that catches a
//!    transposition. The twelve percentage presets are the *derived* part of `PATTERN_MASKS`: an
//!    ordered dither realises exactly the coverage the name states, so `pct25`'s row has exactly
//!    sixteen of sixty-four bits set. Checking that the token at index 3 is `pct25` **and** that
//!    row 3 has sixteen bits set ties this list to that table at twelve points spread through it.
//!
//! # And one thing this file found rather than checked
//!
//! `mjx_paint::pattern`'s module documentation says *"Nine are pictorial"* and then lists **ten**
//! names. The list is the authority and the count was the typo.
//! [`the_pictorial_hatches_are_the_ten_the_mask_table_lists`] asserts the length, so the sentence
//! and the list cannot disagree again — and the ten are what the Windows sitting exists to replace
//! with measurements.

use mjx_dml::PatternType;
use mjx_paint::pattern::{cells_for_percent, PERCENT_COVERAGE};
use mjx_paint::{PATTERN_MASKS, PATTERN_SIDE};
use mjx_reference_pack::typography::{swatches, HATCH_TOKENS, PICTORIAL_HATCHES};
use mjx_scene::PATTERN_PRESET_COUNT;

#[test]
fn every_token_is_a_real_preset_pattern() {
    for token in HATCH_TOKENS {
        let preset = PatternType::from_wire(token)
            .unwrap_or_else(|| panic!("`{token}` is not a `ST_PresetPatternVal` value"));
        assert_eq!(
            preset.to_wire(),
            token,
            "`{token}` parsed as a preset that spells itself differently"
        );
    }
}

#[test]
fn there_are_fifty_four_of_them_and_no_two_are_the_same() {
    assert_eq!(
        HATCH_TOKENS.len(),
        PATTERN_PRESET_COUNT as usize,
        "the token list and `mjx-scene`'s own count disagree"
    );
    assert_eq!(
        PATTERN_MASKS.len(),
        HATCH_TOKENS.len(),
        "the mask table and the token list are different lengths, so an index means two things"
    );
    let distinct: std::collections::BTreeSet<&str> = HATCH_TOKENS.into_iter().collect();
    assert_eq!(
        distinct.len(),
        HATCH_TOKENS.len(),
        "the token list repeats a preset, so one is missing"
    );
}

/// The order check, and the only one of the three that would catch a transposition.
#[test]
fn the_twelve_percentages_land_where_their_own_coverage_says() {
    assert_eq!(PERCENT_COVERAGE.len(), 12);
    for (index, percent) in PERCENT_COVERAGE.into_iter().enumerate() {
        let token = HATCH_TOKENS[index];
        assert_eq!(
            token,
            format!("pct{percent}"),
            "index {index} of the token list is `{token}` and the mask table's row {index} is the \
             {percent} % dither"
        );
        let set: u32 = PATTERN_MASKS[index]
            .iter()
            .map(|row| u32::from(row.count_ones()))
            .sum();
        assert_eq!(
            set,
            cells_for_percent(percent),
            "`{token}` is at index {index}, where the mask has {set} of {} cells set and {percent} \
             % of them is {}",
            PATTERN_SIDE * PATTERN_SIDE,
            cells_for_percent(percent)
        );
    }
}

#[test]
fn the_pictorial_hatches_are_the_ten_the_mask_table_lists() {
    // `mjx_paint::pattern`'s own documentation says "Nine are pictorial" and lists ten names.
    assert_eq!(
        PICTORIAL_HATCHES.len(),
        10,
        "the pictorial set has changed size; `mjx_paint::pattern`'s module documentation lists it \
         by name and once said `nine` while naming ten"
    );
    for token in PICTORIAL_HATCHES {
        assert!(
            HATCH_TOKENS.contains(&token),
            "`{token}` is called pictorial and is not a preset pattern"
        );
        // Each is on the sheet, or the sitting cannot answer for it.
        assert!(
            swatches().iter().any(|swatch| swatch.token == token),
            "`{token}` is what the sitting exists to measure and it is not on the swatch sheet"
        );
    }
    // And they are a strict subset: a "pictorial" list that had grown to cover the whole table
    // would mean every hatch was a guess, which is not what the mask table says.
    assert!(PICTORIAL_HATCHES.len() < HATCH_TOKENS.len() / 2);
}

/// Every swatch is on a page, at a place, with a window a crop could be taken from — and the fifty-four
/// of them do not sit on top of each other.
#[test]
fn the_swatches_are_laid_out_where_a_crop_can_find_them() {
    let swatches = swatches();
    assert_eq!(swatches.len(), PATTERN_PRESET_COUNT as usize);
    let mut seen: std::collections::BTreeSet<(usize, i64, i64)> = std::collections::BTreeSet::new();
    for swatch in &swatches {
        assert_eq!(
            swatch.token, HATCH_TOKENS[swatch.index],
            "swatch {} names `{}`",
            swatch.index, swatch.token
        );
        assert!(swatch.bounds.width > 0 && swatch.bounds.height > 0);
        assert!(
            swatch.window.width >= swatch.bounds.width,
            "`{}`'s comparison window is narrower than the swatch it exists to crop",
            swatch.token
        );
        assert!(
            seen.insert((swatch.page, swatch.bounds.x, swatch.bounds.y)),
            "two swatches are in the same place on page {}",
            swatch.page
        );
    }
    // The gate's own spread: fifty-four swatches over more than one page, at more than one column.
    let pages: std::collections::BTreeSet<usize> =
        swatches.iter().map(|swatch| swatch.page).collect();
    let columns: std::collections::BTreeSet<i64> =
        swatches.iter().map(|swatch| swatch.bounds.x).collect();
    assert!(
        pages.len() >= 3 && columns.len() >= 6,
        "the swatches occupy {} page(s) and {} column(s), which is not a grid",
        pages.len(),
        columns.len()
    );
}
