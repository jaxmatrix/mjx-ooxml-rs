/**
 * The measure grammar: `12 pt`, `2.5 cm`, `1"`, `1,5 cm`.
 *
 * **This module is arithmetic with correct answers, and it is separated from the component for
 * exactly that reason.** MJXOFF-186 names the trap:
 *
 * > A measure input carries units. Parsing and re-formatting is arithmetic with correct answers —
 * > assert them as numbers, including a locale decimal comma if you support one, and say what you
 * > do with an unparseable string rather than silently reverting.
 *
 * So every answer here is a number, `tests/inputs.test.ts` asserts the numbers, and nothing in this
 * file touches the DOM.
 *
 * ## Points are the canonical unit, and everything else is a factor
 *
 * A field that stored *its own displayed unit* would have to convert on every change of unit and
 * would accumulate a rounding error per conversion. Storing points and converting only for display
 * means the stored value never moves when the display unit changes — switching a `12 pt` field to
 * centimetres shows `0.42 cm` and switching it back shows `12 pt`, not `11.99 pt`.
 *
 * Points rather than EMUs because this is a *chrome* measure — what a person types into a spacing
 * box. The document model's EMU is `mjx-ooxml-core`'s business and the conversion between them is
 * one multiplication, in whichever layer needs it.
 *
 * ## Both decimal separators are accepted, always. The attribute governs the *writing*.
 *
 * This grammar has **no thousands separator** — a paragraph indent is never in the thousands — so
 * `1,5` and `1.5` can only ever mean the same quantity. Refusing the comma in a dot locale would
 * therefore reject a keystroke that has exactly one possible meaning, which punishes a person with
 * a numeric keypad and buys no disambiguation at all. `decimalSeparator` decides how a value is
 * *written back*, which is the half that is genuinely a locale question.
 *
 * ## Node-importable
 *
 * Data and pure functions. No DOM, no tokens, no CSS.
 */

/** The units a measure field understands. Points first: it is the one everything converts through. */
export const measureUnitNames = ['pt', 'px', 'pc', 'in', 'cm', 'mm'] as const;

/** One of the six. */
export type MeasureUnit = (typeof measureUnitNames)[number];

/** Whether a value names one of the six. */
export function isMeasureUnit(value: unknown): value is MeasureUnit {
  return (measureUnitNames as readonly string[]).includes(String(value));
}

/** Everything a unit fixes. */
export interface MeasureUnitSpec {
  /** How many points one of this unit is. The one number that defines the unit. */
  readonly points: number;
  /** How many decimal places a value is written back with, before trailing zeros are trimmed. */
  readonly precision: number;
  /** The spellings a person may type, lower-cased. The canonical one is the unit's own name. */
  readonly accepts: readonly string[];
  /** What it is, for the catalogue. */
  readonly use: string;
}

/**
 * The six units, and the four constants that generate the rest.
 *
 * A point is 1/72 of an inch, an inch is 2.54 centimetres, a CSS pixel is 1/96 of an inch and a
 * pica is twelve points. Every factor below is one of those written as arithmetic rather than as a
 * decimal typed out, so a reader can check it without a calculator and a typo cannot hide in the
 * seventh digit.
 */
export const measureUnits: Readonly<Record<MeasureUnit, MeasureUnitSpec>> = {
  pt: {
    points: 1,
    precision: 2,
    accepts: ['pt', 'point', 'points'],
    use: 'The typographic point. Font sizes, paragraph spacing, border widths.',
  },
  px: {
    points: 72 / 96,
    precision: 2,
    accepts: ['px', 'pixel', 'pixels'],
    use: 'The CSS pixel — 1/96 inch, not a device pixel. What a web-authored value arrives in.',
  },
  pc: {
    points: 12,
    precision: 3,
    accepts: ['pc', 'pica', 'picas'],
    use: 'The pica, twelve points. Rare, and accepted because typesetters type it.',
  },
  in: {
    points: 72,
    precision: 3,
    accepts: ['in', 'inch', 'inches', '"', '″', '”'],
    use: 'The inch. Page margins in the United States, and the unit a straight or curly double quote means.',
  },
  cm: {
    points: 72 / 2.54,
    precision: 2,
    accepts: ['cm', 'centimetre', 'centimetres', 'centimeter', 'centimeters'],
    use: 'The centimetre. Page margins nearly everywhere else.',
  },
  mm: {
    points: 72 / 25.4,
    precision: 1,
    accepts: ['mm', 'millimetre', 'millimetres', 'millimeter', 'millimeters'],
    use: 'The millimetre. Bleed, trim, and anything a printer specified.',
  },
};

/** The unit a field uses when it does not say. */
export const defaultMeasureUnit: MeasureUnit = 'pt';

/** How a value is written back. Parsing accepts both — see the module note. */
export const decimalSeparatorNames = ['dot', 'comma'] as const;

/** One of the two. */
export type DecimalSeparator = (typeof decimalSeparatorNames)[number];

/** `dot` → `.`, `comma` → `,`. */
export function decimalCharacter(separator: DecimalSeparator): string {
  return separator === 'comma' ? ',' : '.';
}

/** Whether a value names one of the two. */
export function isDecimalSeparator(value: unknown): value is DecimalSeparator {
  return (decimalSeparatorNames as readonly string[]).includes(String(value));
}

/** The unit a spelling means, or `undefined`. Case-insensitive; `IN`, `In` and `in` are one unit. */
export function unitFromSpelling(spelling: string): MeasureUnit | undefined {
  const wanted = spelling.trim().toLowerCase();
  if (wanted === '') return undefined;
  for (const unit of measureUnitNames) {
    if (measureUnits[unit].accepts.includes(wanted)) return unit;
  }
  return undefined;
}

/** A parsed measure: the quantity in points, and the unit it was written in. */
export interface Measure {
  /** The canonical quantity. Everything downstream stores this. */
  readonly points: number;
  /** The unit the text carried, or the field's default when it carried none. */
  readonly unit: MeasureUnit;
  /** The number as written, in `unit`. `2.5` for `2.5 cm`. */
  readonly magnitude: number;
  /** True when the text named no unit and the field's default was used. */
  readonly unitWasImplied: boolean;
}

/**
 * The number, and then optionally a unit. Nothing else, in either order.
 *
 * ⚠ The number may **not** carry a thousands separator and may **not** be bare-signed (`-` alone),
 * bare-pointed (`.` alone) or doubly-pointed (`1.2.3`) — each of those is a distinct way a person
 * mistypes, and each of them lands in the invalid path with the text preserved rather than being
 * silently read as something adjacent.
 */
const grammar = /^([+-]?)(\d+(?:[.,]\d+)?|[.,]\d+)\s*(.*)$/;

/** What `readTypedNumber` found: a number, and whatever followed it. */
export interface TypedNumber {
  readonly magnitude: number;
  /** Everything after the number, trimmed. A unit, a `%`, or the empty string. */
  readonly tail: string;
}

/**
 * Read a number out of what a person typed, and hand back whatever followed it.
 *
 * ⚠ **The one grammar, and MJXOFF-190 is why it is exported.** `<mjx-zoom-control>`'s readout takes
 * a percentage rather than a length, so it cannot be an `<mjx-measure-input>` — there is no `%`
 * among the six units and a percentage has no value in points. What it *must* share is how a number
 * is read: both decimal separators accepted, no thousands separator, and `-`, `.` and `1.2.3`
 * each landing in the invalid path with the text preserved rather than being read as something
 * adjacent. A second regex somewhere else would have been a second answer to *"is `1.2.3` a
 * number"*, and the first defect would have been one field accepting what the other refused.
 *
 * The failure vocabulary is shared for the same reason: `empty` and `notANumber` mean exactly what
 * they mean here, and a caller that wants a third meaning for the tail says so with `unknownUnit`.
 */
export function readTypedNumber(
  text: string,
): { readonly ok: true; readonly value: TypedNumber } | { readonly ok: false; readonly error: MeasureParseError } {
  const trimmed = text.trim();
  if (trimmed === '') return { ok: false, error: { failure: 'empty', offending: text } };

  const match = grammar.exec(trimmed);
  if (match === null) {
    return { ok: false, error: { failure: 'notANumber', offending: trimmed } };
  }
  const [, sign = '', digits = '', tail = ''] = match;

  // Both separators, always. The grammar has no thousands separator, so there is nothing for a
  // comma to be ambiguous *with*.
  const magnitude = Number.parseFloat(`${sign}${digits.replace(',', '.')}`);
  if (!Number.isFinite(magnitude)) {
    return { ok: false, error: { failure: 'notANumber', offending: trimmed } };
  }
  return { ok: true, value: { magnitude, tail: tail.trim() } };
}

/**
 * What a person is told, per failure. One sentence each, and each says what to do.
 *
 * Here rather than in the component for the reason `src/harness/presets.ts` states: **a module
 * that defines a custom element cannot be imported from Node**, and the unit tier runs in Node.
 * A message is data, and data belongs where a test can read it.
 *
 * `{offending}` is the fragment that defeated the parse and `{units}` the list of spellings, both
 * substituted by the component — so a failure never says only *that* it failed.
 */
export const measureFailureMessages = {
  empty: 'Type a measurement, such as 12 pt.',
  notANumber: '“{offending}” is not a number. Type a number, optionally followed by a unit.',
  unknownUnit: '“{offending}” is not a unit this field knows. Use {units}.',
} as const;

/** How a parse can fail, so a field can say *which* thing it did not understand. */
export const measureParseFailures = ['empty', 'notANumber', 'unknownUnit'] as const;

/** One of the three. */
export type MeasureParseFailure = (typeof measureParseFailures)[number];

/** A parse that did not produce a measure, and why. */
export interface MeasureParseError {
  readonly failure: MeasureParseFailure;
  /** The fragment that was not understood — the whole text, or just the unit. */
  readonly offending: string;
}

/** What `parseMeasure` returns: a measure, or the reason there is not one. */
export type MeasureParse =
  | { readonly ok: true; readonly measure: Measure }
  | { readonly ok: false; readonly error: MeasureParseError };

/**
 * Read a measure out of what a person typed.
 *
 * **It never guesses.** A string it does not understand comes back as an error carrying the
 * fragment that defeated it, and `<mjx-measure-input>` keeps the person's text, marks the field
 * `aria-invalid` and commits nothing — see that component's note for why silently reverting is the
 * one behaviour this must not have.
 */
export function parseMeasure(
  text: string,
  defaultUnit: MeasureUnit = defaultMeasureUnit,
): MeasureParse {
  const read = readTypedNumber(text);
  if (!read.ok) return { ok: false, error: read.error };
  const { magnitude, tail: spelling } = read.value;

  if (spelling === '') {
    return {
      ok: true,
      measure: {
        points: magnitude * measureUnits[defaultUnit].points,
        unit: defaultUnit,
        magnitude,
        unitWasImplied: true,
      },
    };
  }

  const unit = unitFromSpelling(spelling);
  if (unit === undefined) {
    return { ok: false, error: { failure: 'unknownUnit', offending: spelling } };
  }
  return {
    ok: true,
    measure: {
      points: magnitude * measureUnits[unit].points,
      unit,
      magnitude,
      unitWasImplied: false,
    },
  };
}

/** A quantity in points, expressed in a unit. `72 pt` → `2.54` centimetres. */
export function pointsIn(points: number, unit: MeasureUnit): number {
  return points / measureUnits[unit].points;
}

/** The reverse. `2.54` centimetres → `72` points. */
export function pointsFrom(magnitude: number, unit: MeasureUnit): number {
  return magnitude * measureUnits[unit].points;
}

/**
 * Round to a unit's precision and drop the zeros the rounding produced.
 *
 * `12.00` is written `12` and `2.50` is written `2.5`, because a spacing box that says `12.00 pt`
 * reads as a field that has been *computed at* rather than typed into.
 */
export function roundToPrecision(magnitude: number, precision: number): number {
  const scale = 10 ** precision;
  // `Math.round` on a negative half rounds toward positive infinity, so a value of exactly −0.005
  // would go to −0.00 rather than −0.01. Rounding the magnitude and restoring the sign is what
  // makes the two directions symmetric, which is what a person expects of a spacing box that
  // accepts negatives.
  const sign = magnitude < 0 ? -1 : 1;
  return (sign * Math.round(Math.abs(magnitude) * scale)) / scale;
}

/**
 * Write a quantity back, in a unit, with a separator.
 *
 * `formatMeasure(70.866…, 'cm', 'dot')` → `2.5 cm`; with `'comma'` → `2,5 cm`.
 */
export function formatMeasure(
  points: number,
  unit: MeasureUnit,
  separator: DecimalSeparator = 'dot',
): string {
  const rounded = roundToPrecision(pointsIn(points, unit), measureUnits[unit].precision);
  // `toFixed` then trim, rather than `String(rounded)`, because a value that rounded to an integer
  // must not come back in exponential notation and a value with a long binary tail must not come
  // back with it. Both are real: `1e-7` and `0.30000000000000004`.
  const fixed = rounded.toFixed(measureUnits[unit].precision);
  const trimmed = fixed.includes('.') ? fixed.replace(/\.?0+$/, '') : fixed;
  // `-0` is a number a person never means and never typed.
  const text = trimmed === '-0' ? '0' : trimmed;
  return `${text.replace('.', decimalCharacter(separator))} ${unit}`;
}

/** A range a measure must stay inside, in points. Either end may be absent. */
export interface MeasureRange {
  readonly minimumPoints?: number;
  readonly maximumPoints?: number;
}

/** Bring a quantity inside its range. A range with no ends returns the quantity unchanged. */
export function clampMeasure(points: number, range: MeasureRange): number {
  let value = points;
  if (range.minimumPoints !== undefined) value = Math.max(value, range.minimumPoints);
  if (range.maximumPoints !== undefined) value = Math.min(value, range.maximumPoints);
  return value;
}

/** Whether a quantity is inside its range. */
export function withinRange(points: number, range: MeasureRange): boolean {
  return clampMeasure(points, range) === points;
}

/**
 * Move a measure by one step, in the unit it is displayed in.
 *
 * Stepping in **points** would make Arrow Up in a centimetres field move by 1/28th of a
 * centimetre, which is a number no field should ever show. Stepping in the displayed unit and
 * converting back is what makes `2.5 cm` become `3.5 cm` rather than `2.54 cm`.
 */
export function stepMeasure(
  points: number,
  unit: MeasureUnit,
  step: number,
  direction: 1 | -1,
  range: MeasureRange = {},
): number {
  const magnitude = pointsIn(points, unit);
  const moved = roundToPrecision(magnitude + step * direction, measureUnits[unit].precision);
  return clampMeasure(pointsFrom(moved, unit), range);
}
