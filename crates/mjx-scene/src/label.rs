//! The lettering a placeholder's label is drawn in: a small built-in block face set as filled rectangles, so every painter draws the same readable label and none of them opens a font.

use crate::geometry::{FillRule, Geometry, PathCommand, ScenePoint, SceneRect};

/// A label set inside a placeholder's box, in the box's own space.
#[derive(Clone, PartialEq, Debug)]
pub struct PlaceholderLettering {
    /// The rectangle the letters sit on, inside the box.
    pub plate: SceneRect,
    /// The letters, as closed rectangles filled non-zero.
    pub ink: Geometry,
}

// How many cells wide a glyph is.
const GLYPH_COLUMNS: usize = 5;
// How many cells a glyph advances the pen by.
const ADVANCE_CELLS: usize = 6;
// How many cell rows a glyph occupies, the last two for descenders.
const GLYPH_ROWS: usize = 9;
// How many cell rows one line of lettering advances by.
const LINE_CELLS: usize = 10;
// The cell sizes a label is tried at, largest first, in device pixels.
const CELL_PIXELS: [f32; 3] = [3.0, 2.0, 1.0];

/// Sets `label` inside `within` at the largest cell size its words fit at, or `None` when the box cannot hold a single letter.
#[must_use]
pub fn placeholder_lettering(label: &str, within: SceneRect) -> Option<PlaceholderLettering> {
    let words: Vec<&str> = label.split_whitespace().collect();
    if words.is_empty() || !within.width().is_finite() || !within.height().is_finite() {
        return None;
    }
    for cell in CELL_PIXELS {
        let columns = cells_across(within.width() - 2.0 * cell, cell);
        let rows = cells_across(within.height() - 2.0 * cell, cell);
        let lines = wrap(&words, columns);
        if lines.iter().all(|line| line_cells(line) <= columns)
            && (lines.len() * LINE_CELLS).saturating_sub(1) <= rows
        {
            return Some(set(&lines, within, cell));
        }
    }
    // Too small for every word at the smallest size: as many lines as fit, each cut to the box's width.
    let cell = 1.0;
    let columns = cells_across(within.width(), cell);
    let rows = cells_across(within.height(), cell);
    let characters = (columns + 1) / ADVANCE_CELLS;
    let line_count = (rows + 1) / LINE_CELLS;
    if characters == 0 || rows < GLYPH_ROWS - 2 {
        return None;
    }
    let lines: Vec<String> = wrap(&words, columns)
        .into_iter()
        .take(line_count.max(1))
        .map(|line| line.chars().take(characters).collect())
        .collect();
    Some(set(&lines, within, cell))
}

// Whole cells of `cell` pixels that fit in `length`.
fn cells_across(length: f32, cell: f32) -> usize {
    if length <= 0.0 {
        0
    } else {
        (length / cell).floor() as usize
    }
}

// How many cells wide a line of lettering is.
fn line_cells(line: &str) -> usize {
    (line.chars().count() * ADVANCE_CELLS).saturating_sub(1)
}

// The words, greedily broken into lines no wider than `columns` cells where a break allows it.
fn wrap(words: &[&str], columns: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in words {
        match lines.last_mut() {
            Some(line) if line_cells(&format!("{line} {word}")) <= columns => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push((*word).to_owned()),
        }
    }
    lines
}

// The lines centred in `within` at `cell` pixels a cell, each cell on a whole device pixel.
fn set(lines: &[String], within: SceneRect, cell: f32) -> PlaceholderLettering {
    let block_height = (lines.len() * LINE_CELLS).saturating_sub(1) as f32 * cell;
    let top = (within.top + (within.height() - block_height) / 2.0).round();
    let mut commands = Vec::new();
    let mut extent: Option<SceneRect> = None;
    for (index, line) in lines.iter().enumerate() {
        let width = line_cells(line) as f32 * cell;
        let left = (within.left + (within.width() - width) / 2.0).round();
        let baseline = top + (index * LINE_CELLS) as f32 * cell;
        for (position, character) in line.chars().enumerate() {
            let origin = left + (position * ADVANCE_CELLS) as f32 * cell;
            for (row, bits) in glyph(character).iter().enumerate() {
                let y = baseline + row as f32 * cell;
                let mut column = 0;
                while column < GLYPH_COLUMNS {
                    if bits & (1_u8 << (GLYPH_COLUMNS - 1 - column)) == 0 {
                        column += 1;
                        continue;
                    }
                    let start = column;
                    while column < GLYPH_COLUMNS
                        && bits & (1_u8 << (GLYPH_COLUMNS - 1 - column)) != 0
                    {
                        column += 1;
                    }
                    let rect = SceneRect::new(
                        origin + start as f32 * cell,
                        y,
                        origin + column as f32 * cell,
                        y + cell,
                    );
                    rectangle(&mut commands, rect);
                    extent = Some(extent.map_or(rect, |seen| union(seen, rect)));
                }
            }
        }
    }
    let plate = extent.map_or(SceneRect::EMPTY, |letters| {
        SceneRect::new(
            (letters.left - cell).max(within.left),
            (letters.top - cell).max(within.top),
            (letters.right + cell).min(within.right),
            (letters.bottom + cell).min(within.bottom),
        )
    });
    PlaceholderLettering {
        plate,
        ink: Geometry::path(commands, FillRule::NonZero),
    }
}

// Appends one closed rectangle.
fn rectangle(into: &mut Vec<PathCommand>, rect: SceneRect) {
    into.push(PathCommand::MoveTo(ScenePoint::new(rect.left, rect.top)));
    into.push(PathCommand::LineTo(ScenePoint::new(rect.right, rect.top)));
    into.push(PathCommand::LineTo(ScenePoint::new(
        rect.right,
        rect.bottom,
    )));
    into.push(PathCommand::LineTo(ScenePoint::new(rect.left, rect.bottom)));
    into.push(PathCommand::Close);
}

// The smallest rectangle holding both.
fn union(a: SceneRect, b: SceneRect) -> SceneRect {
    SceneRect::new(
        a.left.min(b.left),
        a.top.min(b.top),
        a.right.max(b.right),
        a.bottom.max(b.bottom),
    )
}

// A character's nine rows of five cells, the leftmost cell the highest bit; an unknown character is a hollow box.
fn glyph(character: char) -> [u8; GLYPH_ROWS] {
    GLYPHS
        .iter()
        .find(|(known, _)| *known == character)
        .map_or(UNKNOWN, |(_, rows)| *rows)
}

const UNKNOWN: [u8; GLYPH_ROWS] = [
    0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111, 0, 0,
];

// The block face: capitals, small letters, digits and the punctuation a label uses.
const GLYPHS: [(char, [u8; GLYPH_ROWS]); 71] = [
    (' ', [0, 0, 0, 0, 0, 0, 0, 0, 0]),
    (
        'A',
        [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001, 0, 0,
        ],
    ),
    (
        'B',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110, 0, 0,
        ],
    ),
    (
        'C',
        [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        'D',
        [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110, 0, 0,
        ],
    ),
    (
        'E',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111, 0, 0,
        ],
    ),
    (
        'F',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0, 0,
        ],
    ),
    (
        'G',
        [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111, 0, 0,
        ],
    ),
    (
        'H',
        [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001, 0, 0,
        ],
    ),
    (
        'I',
        [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0, 0,
        ],
    ),
    (
        'J',
        [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100, 0, 0,
        ],
    ),
    (
        'K',
        [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001, 0, 0,
        ],
    ),
    (
        'L',
        [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111, 0, 0,
        ],
    ),
    (
        'M',
        [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001, 0, 0,
        ],
    ),
    (
        'N',
        [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001, 0, 0,
        ],
    ),
    (
        'O',
        [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        'P',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000, 0, 0,
        ],
    ),
    (
        'Q',
        [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101, 0, 0,
        ],
    ),
    (
        'R',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001, 0, 0,
        ],
    ),
    (
        'S',
        [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110, 0, 0,
        ],
    ),
    (
        'T',
        [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0,
        ],
    ),
    (
        'U',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        'V',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0, 0,
        ],
    ),
    (
        'W',
        [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010, 0, 0,
        ],
    ),
    (
        'X',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001, 0, 0,
        ],
    ),
    (
        'Y',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0,
        ],
    ),
    (
        'Z',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111, 0, 0,
        ],
    ),
    (
        'a',
        [0, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111, 0, 0],
    ),
    (
        'b',
        [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110, 0, 0,
        ],
    ),
    (
        'c',
        [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110, 0, 0],
    ),
    (
        'd',
        [
            0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111, 0, 0,
        ],
    ),
    (
        'e',
        [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110, 0, 0],
    ),
    (
        'f',
        [
            0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000, 0, 0,
        ],
    ),
    (
        'g',
        [
            0, 0, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110,
        ],
    ),
    (
        'h',
        [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001, 0, 0,
        ],
    ),
    (
        'i',
        [
            0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110, 0, 0,
        ],
    ),
    (
        'j',
        [
            0b00010, 0, 0b00110, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
    ),
    (
        'k',
        [
            0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0, 0,
        ],
    ),
    (
        'l',
        [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0, 0,
        ],
    ),
    (
        'm',
        [0, 0, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101, 0, 0],
    ),
    (
        'n',
        [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001, 0, 0],
    ),
    (
        'o',
        [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110, 0, 0],
    ),
    (
        'p',
        [
            0, 0, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000,
        ],
    ),
    (
        'q',
        [
            0, 0, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001,
        ],
    ),
    (
        'r',
        [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000, 0, 0],
    ),
    (
        's',
        [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110, 0, 0],
    ),
    (
        't',
        [
            0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110, 0, 0,
        ],
    ),
    (
        'u',
        [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101, 0, 0],
    ),
    (
        'v',
        [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0, 0],
    ),
    (
        'w',
        [0, 0, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010, 0, 0],
    ),
    (
        'x',
        [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0, 0],
    ),
    (
        'y',
        [
            0, 0, 0b10001, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110,
        ],
    ),
    (
        'z',
        [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111, 0, 0],
    ),
    (
        '0',
        [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        '1',
        [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0, 0,
        ],
    ),
    (
        '2',
        [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111, 0, 0,
        ],
    ),
    (
        '3',
        [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        '4',
        [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010, 0, 0,
        ],
    ),
    (
        '5',
        [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        '6',
        [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        '7',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0, 0,
        ],
    ),
    (
        '8',
        [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110, 0, 0,
        ],
    ),
    (
        '9',
        [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100, 0, 0,
        ],
    ),
    ('.', [0, 0, 0, 0, 0, 0b01100, 0b01100, 0, 0]),
    (',', [0, 0, 0, 0, 0, 0b01100, 0b00100, 0b01000, 0]),
    ('-', [0, 0, 0, 0b01110, 0, 0, 0, 0, 0]),
    ('\'', [0b00100, 0b00100, 0b01000, 0, 0, 0, 0, 0, 0]),
    (':', [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0, 0, 0]),
    (
        '/',
        [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000, 0, 0,
        ],
    ),
    (
        '(',
        [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010, 0, 0,
        ],
    ),
    (
        ')',
        [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000, 0, 0,
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_set_inside_its_box_and_wraps_rather_than_overflowing() {
        let within = SceneRect::new(20.0, 20.0, 180.0, 100.0);
        let lettering = placeholder_lettering("Picture not available", within).expect("a label");
        let letters = lettering.ink.bounds();
        assert!(letters.left >= within.left && letters.right <= within.right);
        assert!(letters.top >= within.top && letters.bottom <= within.bottom);
        assert!(lettering.plate.left <= letters.left && lettering.plate.right >= letters.right);
        assert_eq!(
            letters.height(),
            34.0,
            "a capital on line one down to the baseline of line two, at two pixels a cell"
        );
    }

    #[test]
    fn every_character_of_every_loss_label_has_a_glyph_of_its_own() {
        for character in "Chart Diagram Embedded object Ink Picture Text shaped Content read Approximated Colour resolved Fill picture available colour Paint approximated embedded Effect drawn Arrowhead Shape outline".chars() {
            assert!(
                character == ' ' || glyph(character) != UNKNOWN,
                "{character:?} has no glyph"
            );
        }
    }

    #[test]
    fn a_box_too_small_for_one_letter_has_no_label() {
        assert_eq!(
            placeholder_lettering("Chart not rendered", SceneRect::new(0.0, 0.0, 4.0, 40.0)),
            None
        );
    }
}
