//! DrawingML's fifty-four preset patterns, as eight-by-eight masks a painter can sample.
//!
//! # Where these come from, stated plainly
//!
//! **ECMA-376 names the fifty-four presets and gives a bitmap for none of them.** `ST_PresetPatternVal`
//! is a list of tokens — `pct25`, `dkHorz`, `wave`, `shingle` — and the prose says what each is
//! called, not what it looks like. The images Office draws are not in the specification.
//!
//! R07 met the same shape of problem with `a:headEnd` and `a:tailEnd`, where the specification names
//! six arrowhead shapes and three sizes and states no proportions, and refused to invent them *into
//! the document model* — `Stroke::head` carries the document's intent through the display list
//! untouched, and MJXOFF-88 owns the geometry. That refusal is right and this table does not
//! contradict it: a hatch bitmap is not a document's intent, it is **a painter's rendering of one**.
//! It belongs where a rendering decision belongs, and it is `pub` so that R09's `tiny-skia`, PDF and
//! SVG painters sample the same fifty-four masks — four painters disagreeing about a hatch would be
//! worse than one approximate hatch, and much harder to notice.
//!
//! So the provenance of each family is written down rather than implied:
//!
//! * **The twelve percentages are derived, not drawn.** The name states a coverage and an ordered
//!   (Bayer) dither realises exactly that coverage over sixty-four cells, so
//!   [`PATTERN_MASKS`]`[Percent25]` has exactly sixteen bits set and cannot drift.
//!   `tests/the_pattern_table_is_a_table.rs` asserts the relationship rather than the numbers, and
//!   separately asserts that [`BAYER_8X8`] is a permutation of `0..64`, which is the hand-entered
//!   part.
//! * **The line, grid, checker, diagonal and brick families are derived from what their names
//!   state**: a direction, a period and a weight. `ltHorz` is one row in eight, `horz` one in four,
//!   `dkHorz` two in four, `narHorz` one in two, `dashHorz` one in four broken in half. That is the
//!   whole content of "light", "dark", "narrow" and "dashed", and it is why those thirty are written
//!   as pictures below rather than argued about.
//! * **Ten are pictorial** — `smConfetti`, `lgConfetti`, `plaid`, `sphere`, `weave`, `divot`,
//!   `shingle`, `wave`, `trellis`, `zigZag` — and no naming rule determines them. (This line said
//!   *"nine"* and listed ten until MJXOFF-207 counted them while building the swatch sheet the
//!   Windows sitting will measure. The list was right and the number was not;
//!   `crates/mjx-reference-pack/tests/the_hatch_table_is_the_hatch_table.rs` now asserts the length
//!   so the two cannot disagree again.) They are drawn to
//!   the shape the name describes and are **the approximate part of this table**. Replacing them
//!   with measured bitmaps is a change to one array and to nothing else, which is the reason they
//!   are a table at all.
//!
//! # How a mask becomes pixels
//!
//! One bit per cell, most significant bit leftmost, row zero at the top. A set bit is the
//! foreground colour and a clear bit is the background. The painter uploads the whole table once as
//! a single `R8Unorm` texture eight pixels wide and `54 * 8` tall, and samples row
//! `preset * 8 + (y mod 8)` with `repeat` addressing — one texture, one sampler, no branch in the
//! shader and no per-pattern upload.

use mjx_scene::{PatternPreset, PATTERN_PRESET_COUNT};

/// How many cells on a side. Eight is what the family names imply — "one row in eight" is only a
/// description of `ltHorz` on an eight-row tile — and it is what Office's own hatches are.
pub const PATTERN_SIDE: usize = 8;

/// The ordered-dither threshold matrix the twelve percentage presets are built from.
///
/// A permutation of `0..64`: cell `(x, y)` is set for a coverage of `n` cells exactly when
/// `BAYER_8X8[y][x] < n`, so the coverage a name states is the coverage the mask has. The
/// permutation property is the hand-entered part and is asserted, because a typo here would silently
/// change one preset's density and nothing else would notice.
pub const BAYER_8X8: [[u8; PATTERN_SIDE]; PATTERN_SIDE] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// The coverage each of the twelve percentage presets names, in percent, in wire order.
pub const PERCENT_COVERAGE: [u32; 12] = [5, 10, 20, 25, 30, 40, 50, 60, 70, 75, 80, 90];

/// How many of the sixty-four cells a coverage of `percent` sets.
///
/// Rounded to the nearest cell: five percent of sixty-four is 3.2, and a `pct5` that set no cells at
/// all would be indistinguishable from no fill.
#[must_use]
pub const fn cells_for_percent(percent: u32) -> u32 {
    (percent * (PATTERN_SIDE as u32) * (PATTERN_SIDE as u32) + 50) / 100
}

/// One percentage preset's rows.
const fn dithered(percent: u32) -> [u8; PATTERN_SIDE] {
    let cells = cells_for_percent(percent);
    let mut rows = [0u8; PATTERN_SIDE];
    let mut y = 0;
    while y < PATTERN_SIDE {
        let mut x = 0;
        while x < PATTERN_SIDE {
            if (BAYER_8X8[y][x] as u32) < cells {
                rows[y] |= 0b1000_0000 >> x;
            }
            x += 1;
        }
        y += 1;
    }
    rows
}

/// Every preset's mask, in wire order — the order of [`PatternPreset::ALL`].
///
/// Row zero is the top; within a row the most significant bit is the leftmost cell.
pub const PATTERN_MASKS: [[u8; PATTERN_SIDE]; PATTERN_PRESET_COUNT as usize] = [
    // --- the twelve percentages, dithered to exactly the coverage they name ---
    dithered(PERCENT_COVERAGE[0]),
    dithered(PERCENT_COVERAGE[1]),
    dithered(PERCENT_COVERAGE[2]),
    dithered(PERCENT_COVERAGE[3]),
    dithered(PERCENT_COVERAGE[4]),
    dithered(PERCENT_COVERAGE[5]),
    dithered(PERCENT_COVERAGE[6]),
    dithered(PERCENT_COVERAGE[7]),
    dithered(PERCENT_COVERAGE[8]),
    dithered(PERCENT_COVERAGE[9]),
    dithered(PERCENT_COVERAGE[10]),
    dithered(PERCENT_COVERAGE[11]),
    // --- horizontal rules: one row in four ---
    [
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    // Vertical: one column in four.
    [
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
    ],
    // Light horizontal: one row in eight.
    [
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    // Light vertical: one column in eight.
    [
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
    ],
    // Dark horizontal: two rows in four.
    [
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
    ],
    // Dark vertical: two columns in four.
    [
        0b1100_1100,
        0b1100_1100,
        0b1100_1100,
        0b1100_1100,
        0b1100_1100,
        0b1100_1100,
        0b1100_1100,
        0b1100_1100,
    ],
    // Narrow horizontal: one row in two.
    [
        0b1111_1111,
        0b0000_0000,
        0b1111_1111,
        0b0000_0000,
        0b1111_1111,
        0b0000_0000,
        0b1111_1111,
        0b0000_0000,
    ],
    // Narrow vertical: one column in two.
    [
        0b1010_1010,
        0b1010_1010,
        0b1010_1010,
        0b1010_1010,
        0b1010_1010,
        0b1010_1010,
        0b1010_1010,
        0b1010_1010,
    ],
    // Dashed horizontal: a horizontal rule broken in half, alternate courses offset.
    [
        0b1111_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    // Dashed vertical: the same, turned.
    [
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b0000_1000,
        0b0000_1000,
        0b0000_1000,
        0b0000_1000,
    ],
    // Cross: the horizontal and vertical rules together.
    [
        0b1111_1111,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
        0b1111_1111,
        0b1000_1000,
        0b1000_1000,
        0b1000_1000,
    ],
    // Downward diagonal: `\`, one line in four.
    [
        0b1000_1000,
        0b0100_0100,
        0b0010_0010,
        0b0001_0001,
        0b1000_1000,
        0b0100_0100,
        0b0010_0010,
        0b0001_0001,
    ],
    // Upward diagonal: `/`, one line in four.
    [
        0b0001_0001,
        0b0010_0010,
        0b0100_0100,
        0b1000_1000,
        0b0001_0001,
        0b0010_0010,
        0b0100_0100,
        0b1000_1000,
    ],
    // Light downward diagonal: one line in eight.
    [
        0b1000_0000,
        0b0100_0000,
        0b0010_0000,
        0b0001_0000,
        0b0000_1000,
        0b0000_0100,
        0b0000_0010,
        0b0000_0001,
    ],
    // Light upward diagonal: one line in eight.
    [
        0b0000_0001,
        0b0000_0010,
        0b0000_0100,
        0b0000_1000,
        0b0001_0000,
        0b0010_0000,
        0b0100_0000,
        0b1000_0000,
    ],
    // Dark downward diagonal: two cells thick, one line in four.
    [
        0b1100_1100,
        0b0110_0110,
        0b0011_0011,
        0b1001_1001,
        0b1100_1100,
        0b0110_0110,
        0b0011_0011,
        0b1001_1001,
    ],
    // Dark upward diagonal.
    [
        0b0011_0011,
        0b0110_0110,
        0b1100_1100,
        0b1001_1001,
        0b0011_0011,
        0b0110_0110,
        0b1100_1100,
        0b1001_1001,
    ],
    // Wide downward diagonal: two cells thick, one line in eight.
    [
        0b1100_0000,
        0b0110_0000,
        0b0011_0000,
        0b0001_1000,
        0b0000_1100,
        0b0000_0110,
        0b0000_0011,
        0b1000_0001,
    ],
    // Wide upward diagonal.
    [
        0b0000_0011,
        0b0000_0110,
        0b0000_1100,
        0b0001_1000,
        0b0011_0000,
        0b0110_0000,
        0b1100_0000,
        0b1000_0001,
    ],
    // Dashed downward diagonal: the downward line, broken.
    [
        0b1000_1000,
        0b0100_0100,
        0b0000_0000,
        0b0000_0000,
        0b1000_1000,
        0b0100_0100,
        0b0000_0000,
        0b0000_0000,
    ],
    // Dashed upward diagonal.
    [
        0b0001_0001,
        0b0010_0010,
        0b0000_0000,
        0b0000_0000,
        0b0001_0001,
        0b0010_0010,
        0b0000_0000,
        0b0000_0000,
    ],
    // Diagonal cross: both diagonals.
    [
        0b1001_1001,
        0b0110_0110,
        0b0110_0110,
        0b1001_1001,
        0b1001_1001,
        0b0110_0110,
        0b0110_0110,
        0b1001_1001,
    ],
    // Small checkerboard: two-cell squares.
    [
        0b1100_1100,
        0b1100_1100,
        0b0011_0011,
        0b0011_0011,
        0b1100_1100,
        0b1100_1100,
        0b0011_0011,
        0b0011_0011,
    ],
    // Large checkerboard: four-cell squares.
    [
        0b1111_0000,
        0b1111_0000,
        0b1111_0000,
        0b1111_0000,
        0b0000_1111,
        0b0000_1111,
        0b0000_1111,
        0b0000_1111,
    ],
    // Small grid: rules every two cells.
    [
        0b1111_1111,
        0b1010_1010,
        0b1111_1111,
        0b1010_1010,
        0b1111_1111,
        0b1010_1010,
        0b1111_1111,
        0b1010_1010,
    ],
    // Large grid: rules every eight cells — one square to the tile, where `cross` above rules
    // every four. Written this way rather than at the same pitch as `cross` because the two would
    // otherwise be the same eight-by-eight picture, which
    // `tests/the_tables_are_tables.rs` refuses: a fifty-four-entry table of binary literals is
    // exactly where a duplicate hides from a reader, and it found this one on its first run.
    [
        0b1111_1111,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
        0b1000_0000,
    ],
    // Dotted grid: the large grid's rules reduced to dots.
    [
        0b1010_1010,
        0b0000_0000,
        0b1000_1000,
        0b0000_0000,
        0b1010_1010,
        0b0000_0000,
        0b1000_1000,
        0b0000_0000,
    ],
    // Small confetti: single cells, scattered.
    [
        0b1000_0100,
        0b0000_0000,
        0b0010_0000,
        0b0000_1001,
        0b0100_0000,
        0b0000_0010,
        0b0001_0100,
        0b0000_0000,
    ],
    // Large confetti: two-cell flecks, scattered.
    [
        0b1100_0000,
        0b1100_0110,
        0b0000_0110,
        0b0011_0000,
        0b0011_0000,
        0b0000_1100,
        0b0110_1100,
        0b0110_0000,
    ],
    // Horizontal brick: courses of bricks, alternate courses offset by half.
    [
        0b1111_1111,
        0b0001_0000,
        0b0001_0000,
        0b0001_0000,
        0b1111_1111,
        0b0000_0001,
        0b0000_0001,
        0b0000_0001,
    ],
    // Diagonal brick: the same coursing, laid on the diagonal.
    [
        0b1000_0001,
        0b0100_0010,
        0b0010_0100,
        0b0001_1000,
        0b0010_0100,
        0b0100_0010,
        0b1000_0001,
        0b1100_0011,
    ],
    // Solid diamond: filled diamonds on a four-cell pitch.
    [
        0b0001_0000,
        0b0011_1000,
        0b0111_1100,
        0b1111_1110,
        0b0111_1100,
        0b0011_1000,
        0b0001_0000,
        0b0000_0000,
    ],
    // Open diamond: their outlines.
    [
        0b0001_0000,
        0b0010_1000,
        0b0100_0100,
        0b1000_0010,
        0b0100_0100,
        0b0010_1000,
        0b0001_0000,
        0b0000_0000,
    ],
    // Dotted diamond: the outline, dotted.
    [
        0b0001_0000,
        0b0000_0000,
        0b0100_0100,
        0b0000_0000,
        0b0100_0100,
        0b0000_0000,
        0b0001_0000,
        0b0000_0000,
    ],
    // Plaid: broad bands crossing narrow ones.
    [
        0b1111_0000,
        0b1111_0000,
        0b1111_1010,
        0b1111_0101,
        0b1111_0000,
        0b1111_0000,
        0b1010_1111,
        0b0101_1111,
    ],
    // Sphere: a ring with a highlight, on a four-cell pitch.
    [
        0b0111_0111,
        0b1000_1000,
        0b1001_1001,
        0b1000_1000,
        0b0111_0111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    // Weave: over-and-under strands.
    [
        0b1100_0011,
        0b1000_0111,
        0b0000_1110,
        0b0001_1100,
        0b0011_1000,
        0b0111_0000,
        0b1110_0000,
        0b1100_0001,
    ],
    // Divot: a scattering of small chevrons.
    [
        0b0010_0000,
        0b0101_0000,
        0b0000_0000,
        0b0000_0010,
        0b0000_0101,
        0b0000_0000,
        0b0010_0000,
        0b0101_0000,
    ],
    // Shingle: overlapping scallops in courses.
    [
        0b1000_0001,
        0b0100_0010,
        0b0011_1100,
        0b0000_0000,
        0b0001_1000,
        0b0010_0100,
        0b1100_0011,
        0b0000_0000,
    ],
    // Wave: a horizontal sine, repeated.
    [
        0b0000_0000,
        0b0110_0000,
        0b1001_0000,
        0b0000_1001,
        0b0000_0110,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ],
    // Trellis: a lattice.
    [
        0b1111_1111,
        0b0110_0110,
        0b1111_1111,
        0b1001_1001,
        0b1111_1111,
        0b0110_0110,
        0b1111_1111,
        0b1001_1001,
    ],
    // Zig zag: a continuous chevron rule.
    [
        0b1000_1000,
        0b0101_0101,
        0b0010_0010,
        0b0000_0000,
        0b1000_1000,
        0b0101_0101,
        0b0010_0010,
        0b0000_0000,
    ],
];

/// The mask a preset is drawn with.
#[must_use]
pub fn mask_of(preset: PatternPreset) -> [u8; PATTERN_SIDE] {
    let index = preset.wire_value() as usize;
    match PATTERN_MASKS.get(index) {
        Some(mask) => *mask,
        // Unreachable: `wire_value` is the preset's own position in a table of the same length.
        // Written as a fallback rather than an index because nothing on a painting path may panic,
        // and a solid fill is the least surprising thing to draw for a preset that got away.
        None => [0xff; PATTERN_SIDE],
    }
}

/// Whether the cell at `(x, y)` of `preset`'s tile is foreground.
#[must_use]
pub fn cell_of(preset: PatternPreset, x: usize, y: usize) -> bool {
    let mask = mask_of(preset);
    let row = mask.get(y % PATTERN_SIDE).copied().unwrap_or_default();
    row & (0b1000_0000 >> (x % PATTERN_SIDE)) != 0
}

/// The whole table as one 8-wide, `54 * 8`-tall coverage image: `0xff` for foreground, `0x00` for
/// background, row by row from the top, preset by preset in wire order.
///
/// One texture rather than fifty-four: a page that hatches three shapes three different ways then
/// costs one binding and no upload, and the shader's row index is arithmetic rather than a branch.
#[must_use]
pub fn coverage_atlas() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(PATTERN_MASKS.len() * PATTERN_SIDE * PATTERN_SIDE);
    for mask in &PATTERN_MASKS {
        for row in mask {
            for bit in 0..PATTERN_SIDE {
                pixels.push(if row & (0b1000_0000 >> bit) != 0 {
                    0xff
                } else {
                    0x00
                });
            }
        }
    }
    pixels
}
