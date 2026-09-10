/**
 * The formula bar's model — **every hard behaviour in this child is caret-relative, and all of it
 * is here**, in pure functions a Node test can drive without a browser.
 *
 * MJXOFF-192 states the trap in one sentence and it decided this file's whole shape:
 *
 * > Reference colouring, bracket matching and above all the argument tooltip's *"which argument am
 * > I in"* all depend on caret position across nested calls and quoted strings containing commas.
 * > **So the gate is a caret-position table.** A naive comma-counting implementation passes the
 * > simple case and fails the quoted one.
 *
 * So [`naiveActiveArgument`] is **shipped beside** [`activeArgument`], deliberately, as the
 * positive control the gate needs — U11's rule and U12's, applied to a parser: without the naive
 * answer computed beside the real one, *"the table passes"* is a statement about a table nobody has
 * seen reject anything. `tests/formula.test.ts` runs one table through both and asserts they
 * **disagree** at the offsets a quoted comma moves.
 *
 * ## What `mjx-sml` already owns, and what it deliberately does not
 *
 * `crates/mjx-sml/src/formula/mod.rs` is unambiguous, in its own words:
 *
 * > There is **no expression tree, no tokeniser** and no dependency graph — `CellFormula` is a
 * > *reader* over the bytes the file wrote and has no way to produce different ones.
 *
 * That is a settled position rather than a gap (`PLAN.md`, MJXOFF-21): formulas are carried as the
 * text their producer wrote, and nothing in the workspace acts on one. **So there was no
 * tokenisation to mirror, and this file's tokeniser is not a duplicate of anything.**
 *
 * `crates/mjx-sml/src/address.rs` is the opposite case: it owns a complete, authoritative **address
 * grammar** — `CellReference::parse`, `CellRange::parse`, `Anchoring`, `column_index_from_letters`,
 * `GridBounds::normalized_bounds`, `COLUMN_COUNT`, `ROW_COUNT` and a named error for every way an
 * address can be wrong. A second, subtly different address grammar in TypeScript is exactly the
 * duplication the ticket warns about, so this file **mirrors that one**: the same two grid limits,
 * the same four range shapes, the same anchoring vocabulary, the same normalisation rule (*ordering
 * happens in `normalizedBounds` and nowhere else*) and the same problem names, one per
 * `AddressError` variant. `tests/formula.test.ts` restates the limits as literals so a drift is a
 * failing assertion rather than a silent divergence.
 *
 * ⚠ **One deliberate difference, and it is the UI's rather than the grammar's.** `address.rs`
 * refuses `"a1"` outright (`MissingColumnLetters`) because folding case in a *file reader* would
 * accept bytes Excel would not have written. A person typing into the name box does expect `a1` to
 * work, so [`normaliseTypedAddress`] upper-cases the column letters **before** the grammar sees
 * them, and the grammar stays as strict as the Rust one. The normalisation is the name box's, it is
 * named, and it is asserted in both directions.
 *
 * ## Node-importable
 *
 * Strings and numbers. No DOM, no tokens, no CSS — `src/foundations/splitter.ts`'s rule, for the
 * same reason: the unit tier runs in Node and a module that defines a custom element cannot be
 * imported from one.
 */

import { clampToBounds, type SplitterBounds } from '../foundations/splitter.ts';
import type { TokenPath } from '../tokens/resolver.ts';

// ── the grid, mirrored from crates/mjx-sml/src/address.rs ──────────────────────────────────────

/** Columns in the grid: A to XFD. `mjx_sml::COLUMN_COUNT`. */
export const columnCount = 16_384;

/** The last column's zero-based index. `mjx_sml::LAST_COLUMN_INDEX`. */
export const lastColumnIndex = columnCount - 1;

/** Rows in the grid. `mjx_sml::ROW_COUNT`. */
export const rowCount = 1_048_576;

/** The last row's zero-based index. `mjx_sml::LAST_ROW_INDEX`. */
export const lastRowIndex = rowCount - 1;

/**
 * Whether a bound was written with a `$`.
 *
 * `mjx_sml::Anchoring`, with its two members and its marker. A reference's anchoring never changes
 * which cells it names — it changes what happens when the formula is copied — so it is preserved
 * as written and plays no part in [`referenceIdentity`].
 */
export const anchoringNames = ['relative', 'absolute'] as const;

/** One of the two. */
export type Anchoring = (typeof anchoringNames)[number];

/** The character that makes a bound absolute. */
export const anchoringMarker = '$';

/**
 * Every way an address can be wrong, one name per `mjx_sml::AddressError` variant.
 *
 * The names are this catalogue's spelling of the Rust ones — a reader who knows one knows the
 * other — and the set is closed on purpose: a twelfth kind of failure means the grammar diverged
 * from the crate's, which is the thing this vocabulary exists to make visible.
 */
export const addressProblemNames = [
  'empty',
  'missingColumnLetters',
  'missingRowNumber',
  'unexpectedCharacter',
  'columnOutOfGrid',
  'rowOutOfGrid',
  'tooManyRangeEnds',
  'mismatchedRangeEnds',
  'unterminatedSheetName',
  'emptySheetName',
  'missingSheetSeparator',
] as const;

/** One of the eleven. */
export type AddressProblem = (typeof addressProblemNames)[number];

/** What a person is told when an address will not parse. */
export const addressProblemMessages: Readonly<Record<AddressProblem, string>> = {
  empty: 'That reference is empty.',
  missingColumnLetters: 'A column is spelled with letters, A to XFD.',
  missingRowNumber: 'A cell reference needs a row number.',
  unexpectedCharacter: 'That is not a reference this grid understands.',
  columnOutOfGrid: `A column past XFD: the grid has ${String(columnCount)} columns.`,
  rowOutOfGrid: `A row outside the grid: rows are numbered 1 to ${String(rowCount)}.`,
  tooManyRangeEnds: 'A range has two ends, not three.',
  mismatchedRangeEnds: 'A range mixes kinds of end — a cell, a whole column, a whole row.',
  unterminatedSheetName: 'A quoted sheet name has no closing apostrophe.',
  emptySheetName: 'That reference names no sheet.',
  missingSheetSeparator: 'A sheet-qualified reference needs a `!`.',
};

/** The ordered rectangle a range covers. `mjx_sml::GridBounds`. */
export interface GridBounds {
  readonly firstColumn: number;
  readonly lastColumn: number;
  readonly firstRow: number;
  readonly lastRow: number;
}

/** One end of a reference: a column index, a row index, and how each was anchored. */
export interface CellReference {
  readonly column: number;
  readonly row: number;
  readonly columnAnchoring: Anchoring;
  readonly rowAnchoring: Anchoring;
}

/** The four shapes a range can take. `mjx_sml::CellRange`'s four variants. */
export const rangeShapeNames = ['cell', 'cells', 'columns', 'rows'] as const;

/** One of the four. */
export type RangeShape = (typeof rangeShapeNames)[number];

/** A parsed range, in whatever shape it was written. */
export interface CellRange {
  readonly shape: RangeShape;
  /** The two ends' columns and rows, in the order they were written. */
  readonly start: CellReference;
  readonly end: CellReference;
}

/** Either a parsed value or a named problem. Never a throw: this grammar reads untrusted text. */
export type AddressResult<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly problem: AddressProblem };

function fail<T>(problem: AddressProblem): AddressResult<T> {
  return { ok: false, problem };
}

function ok<T>(value: T): AddressResult<T> {
  return { ok: true, value };
}

/**
 * `A` → 0, `XFD` → 16383.
 *
 * ⚠ **Case is not folded**, exactly as `mjx_sml::column_index_from_letters` does not fold it. See
 * this module's note: the name box up-cases before it asks, and the grammar stays strict.
 */
export function columnIndexFromLetters(letters: string): AddressResult<number> {
  if (letters === '') return fail('missingColumnLetters');
  let index = 0;
  for (const character of letters) {
    if (character < 'A' || character > 'Z') return fail('missingColumnLetters');
    index = index * 26 + (character.charCodeAt(0) - 64);
    if (index > columnCount) return fail('columnOutOfGrid');
  }
  if (index > columnCount) return fail('columnOutOfGrid');
  return ok(index - 1);
}

/** 0 → `A`, 16383 → `XFD`. The inverse of the above, and the name box's canonical spelling. */
export function columnLetters(column: number): string {
  if (!Number.isInteger(column) || column < 0 || column > lastColumnIndex) return '';
  let remaining = column + 1;
  let letters = '';
  while (remaining > 0) {
    const digit = (remaining - 1) % 26;
    letters = String.fromCharCode(65 + digit) + letters;
    remaining = Math.floor((remaining - 1) / 26);
  }
  return letters;
}

/** `$B$7` split into its optional marker and its body. */
function takeAnchoring(text: string): { readonly anchoring: Anchoring; readonly rest: string } {
  if (text.startsWith(anchoringMarker)) return { anchoring: 'absolute', rest: text.slice(1) };
  return { anchoring: 'relative', rest: text };
}

/** `A1`, `$A$1`, `A$1`. `mjx_sml::CellReference::parse`. */
export function parseCellReference(text: string): AddressResult<CellReference> {
  if (text === '') return fail('empty');
  const column = takeAnchoring(text);
  const letters = /^[A-Za-z]*/.exec(column.rest)?.[0] ?? '';
  if (letters === '') return fail('missingColumnLetters');
  const afterLetters = column.rest.slice(letters.length);
  const row = takeAnchoring(afterLetters);
  if (row.rest === '') return fail('missingRowNumber');
  if (!/^\d+$/.test(row.rest)) {
    const offending = /[^\d]/.exec(row.rest)?.[0];
    return fail(offending === undefined ? 'missingRowNumber' : 'unexpectedCharacter');
  }
  const columnIndex = columnIndexFromLetters(letters);
  if (!columnIndex.ok) return columnIndex;
  const rowNumber = Number(row.rest);
  if (rowNumber < 1 || rowNumber > rowCount) return fail('rowOutOfGrid');
  return ok({
    column: columnIndex.value,
    row: rowNumber - 1,
    columnAnchoring: column.anchoring,
    rowAnchoring: row.anchoring,
  });
}

type RangeEnd =
  | { readonly kind: 'cell'; readonly value: CellReference }
  | { readonly kind: 'column'; readonly column: number; readonly anchoring: Anchoring }
  | { readonly kind: 'row'; readonly row: number; readonly anchoring: Anchoring };

function parseRangeEnd(text: string): AddressResult<RangeEnd> {
  if (text === '') return fail('empty');
  const first = takeAnchoring(text);
  if (/^[A-Za-z]+$/.test(first.rest)) {
    const column = columnIndexFromLetters(first.rest);
    if (!column.ok) return column;
    return ok({ kind: 'column', column: column.value, anchoring: first.anchoring });
  }
  if (/^\d+$/.test(first.rest)) {
    const rowNumber = Number(first.rest);
    if (rowNumber < 1 || rowNumber > rowCount) return fail('rowOutOfGrid');
    return ok({ kind: 'row', row: rowNumber - 1, anchoring: first.anchoring });
  }
  const cell = parseCellReference(text);
  if (!cell.ok) return cell;
  return ok({ kind: 'cell', value: cell.value });
}

/**
 * `A1`, `A1:C3`, `A:C` or `1:3`, with any pattern of `$`. `mjx_sml::CellRange::parse`.
 *
 * The order of the two ends is **preserved**, never sorted: `C3:A1` comes back as it was written,
 * for the reason the Rust doc gives — normalising one would change bytes in a part nobody asked to
 * edit. Ordering happens in [`normalizedBounds`] and nowhere else.
 */
export function parseCellRange(text: string): AddressResult<CellRange> {
  const colon = text.indexOf(':');
  if (colon < 0) {
    const cell = parseCellReference(text);
    if (!cell.ok) return cell;
    return ok({ shape: 'cell', start: cell.value, end: cell.value });
  }
  const startText = text.slice(0, colon);
  const endText = text.slice(colon + 1);
  if (endText.includes(':')) return fail('tooManyRangeEnds');
  const start = parseRangeEnd(startText);
  if (!start.ok) return start;
  const end = parseRangeEnd(endText);
  if (!end.ok) return end;
  if (start.value.kind !== end.value.kind) return fail('mismatchedRangeEnds');
  if (start.value.kind === 'cell' && end.value.kind === 'cell') {
    return ok({ shape: 'cells', start: start.value.value, end: end.value.value });
  }
  if (start.value.kind === 'column' && end.value.kind === 'column') {
    return ok({
      shape: 'columns',
      start: { column: start.value.column, row: 0, columnAnchoring: start.value.anchoring, rowAnchoring: 'relative' },
      end: { column: end.value.column, row: lastRowIndex, columnAnchoring: end.value.anchoring, rowAnchoring: 'relative' },
    });
  }
  if (start.value.kind === 'row' && end.value.kind === 'row') {
    return ok({
      shape: 'rows',
      start: { column: 0, row: start.value.row, columnAnchoring: 'relative', rowAnchoring: start.value.anchoring },
      end: { column: lastColumnIndex, row: end.value.row, columnAnchoring: 'relative', rowAnchoring: end.value.anchoring },
    });
  }
  return fail('mismatchedRangeEnds');
}

/**
 * The ordered rectangle a range covers, with a whole-column or whole-row form widened to the
 * grid's full extent on the other axis. `mjx_sml::CellRange::normalized_bounds`.
 *
 * ⚠ **Ordering happens here and only here.** The parsed value keeps whatever order its author
 * wrote, which is what lets the bar echo a formula back unchanged.
 */
export function normalizedBounds(range: CellRange): GridBounds {
  return {
    firstColumn: Math.min(range.start.column, range.end.column),
    lastColumn: Math.max(range.start.column, range.end.column),
    firstRow: Math.min(range.start.row, range.end.row),
    lastRow: Math.max(range.start.row, range.end.row),
  };
}

/** How many cells a rectangle covers. `mjx_sml::GridBounds::cell_count`. */
export function cellCount(bounds: GridBounds): number {
  return (bounds.lastColumn - bounds.firstColumn + 1) * (bounds.lastRow - bounds.firstRow + 1);
}

/** `{ column: 1, row: 6 }` → `B7`. The name box's canonical display spelling. */
export function cellAddressText(cell: CellReference): string {
  const columnMark = cell.columnAnchoring === 'absolute' ? anchoringMarker : '';
  const rowMark = cell.rowAnchoring === 'absolute' ? anchoringMarker : '';
  return `${columnMark}${columnLetters(cell.column)}${rowMark}${String(cell.row + 1)}`;
}

/** A range as text, in the shape it was written. */
export function rangeText(range: CellRange): string {
  switch (range.shape) {
    case 'cell':
      return cellAddressText(range.start);
    case 'cells':
      return `${cellAddressText(range.start)}:${cellAddressText(range.end)}`;
    case 'columns': {
      const first = range.start.columnAnchoring === 'absolute' ? anchoringMarker : '';
      const second = range.end.columnAnchoring === 'absolute' ? anchoringMarker : '';
      return `${first}${columnLetters(range.start.column)}:${second}${columnLetters(range.end.column)}`;
    }
    case 'rows': {
      const first = range.start.rowAnchoring === 'absolute' ? anchoringMarker : '';
      const second = range.end.rowAnchoring === 'absolute' ? anchoringMarker : '';
      return `${first}${String(range.start.row + 1)}:${second}${String(range.end.row + 1)}`;
    }
  }
}

/**
 * What a person typed, in the spelling the grammar accepts.
 *
 * **The one difference between this catalogue's address grammar and `mjx-sml`'s**, and it lives
 * here rather than inside the parser so the parser stays as strict as the Rust one. Trimming and
 * up-casing, and nothing else: `  b7 ` is `B7`, and `Sheet1!b7` keeps its sheet name's case,
 * because a sheet name is a *name* and Excel's are case-preserving.
 */
export function normaliseTypedAddress(text: string): string {
  const trimmed = text.trim();
  const bang = trimmed.lastIndexOf('!');
  if (bang < 0) return trimmed.toUpperCase();
  return trimmed.slice(0, bang + 1) + trimmed.slice(bang + 1).toUpperCase();
}

// ── the tokeniser ──────────────────────────────────────────────────────────────────────────────

/**
 * What a run of characters is.
 *
 * ⚠ **`string` is the kind the whole child turns on.** A comma inside one is not a separator, a
 * parenthesis inside one does not open a call, and a `#REF!` inside one is not an error value.
 * Everything caret-relative below reads these tokens rather than the characters, which is the
 * single structural difference between this and the naive implementation the gate rejects.
 */
export const formulaTokenKindNames = [
  'equals',
  'whitespace',
  'number',
  'string',
  'errorValue',
  'boolean',
  'operator',
  'openParen',
  'closeParen',
  'separator',
  'reference',
  'structured',
  'functionName',
  'name',
  'unknown',
] as const;

/** One of the fifteen. */
export type FormulaTokenKind = (typeof formulaTokenKindNames)[number];

/** A parsed reference, with everything the grid needs to draw a box around it. */
export interface ReferenceParts {
  readonly shape: RangeShape;
  /** The sheet as written, without its quotes, when the reference was qualified. */
  readonly sheet?: string;
  /** The external book index as written, when the reference came from `[1]`. */
  readonly book?: string;
  /** The ordered rectangle, from [`normalizedBounds`]. */
  readonly bounds: GridBounds;
}

/** One run of characters, and where it sits in the source. Offsets are half-open: `[from, to)`. */
export interface FormulaToken {
  readonly kind: FormulaTokenKind;
  readonly text: string;
  readonly from: number;
  readonly to: number;
  /** Present on a `reference` token, and only there. */
  readonly reference?: ReferenceParts;
  /** Set on a `string` token whose closing quote is missing — a formula being typed. */
  readonly unterminated?: true;
}

/** The error values a formula may carry as a literal. Longest first, so `#N/A` cannot eat `#NAME?`. */
const errorLiterals = [
  '#GETTING_DATA',
  '#DIV/0!',
  '#VALUE!',
  '#NAME?',
  '#NULL!',
  '#SPILL!',
  '#CALC!',
  '#NUM!',
  '#REF!',
  '#N/A',
];

/** The two-character comparisons, checked before the one-character ones. */
const longOperators = ['<=', '>=', '<>'];

const shortOperators = new Set(['+', '-', '*', '/', '^', '&', '%', '=', '<', '>', ':']);

function isIdentifierStart(character: string): boolean {
  return /[A-Za-z_\\]/.test(character);
}

function isIdentifierPart(character: string): boolean {
  return /[A-Za-z0-9_.?\\]/.test(character);
}

/** How far a reference body runs from `start`, or `-1` when there is not one there. */
function scanReferenceBody(text: string, start: number): number {
  const rest = text.slice(start);
  const cells = /^\$?[A-Za-z]{1,3}\$?\d{1,7}(?::\$?[A-Za-z]{1,3}\$?\d{1,7})?/.exec(rest);
  const columns = /^\$?[A-Za-z]{1,3}:\$?[A-Za-z]{1,3}/.exec(rest);
  const rows = /^\$?\d{1,7}:\$?\d{1,7}/.exec(rest);
  const match = [cells?.[0], columns?.[0], rows?.[0]]
    .filter((candidate): candidate is string => candidate !== undefined)
    .sort((first, second) => second.length - first.length)[0];
  if (match === undefined) return -1;
  const end = start + match.length;
  const next = text[end];
  // `A1B` is a name, not a reference followed by a letter — and `A1(` is a call.
  if (next !== undefined && (isIdentifierPart(next) || next === '(' || next === '[')) return -1;
  return end;
}

/** A quoted sheet name, `''` being an escaped apostrophe. Returns the end offset, or `-1`. */
function scanQuotedName(text: string, start: number): number {
  let index = start + 1;
  while (index < text.length) {
    if (text[index] === "'") {
      if (text[index + 1] === "'") {
        index += 2;
        continue;
      }
      return index + 1;
    }
    index += 1;
  }
  return -1;
}

interface Qualifier {
  readonly end: number;
  readonly sheet: string;
  readonly book?: string;
}

/**
 * `Sheet1!`, `'My Sheet'!`, `[1]Sheet1!`, `Sheet1:Sheet3!` — the part before a reference body.
 *
 * `undefined` when there is no qualifier here, which is the ordinary case and not a failure.
 */
function scanQualifier(text: string, start: number): Qualifier | undefined {
  let index = start;
  let book: string | undefined;
  if (text[index] === '[') {
    const close = text.indexOf(']', index);
    if (close < 0) return undefined;
    book = text.slice(index + 1, close);
    index = close + 1;
  }
  let sheet: string;
  if (text[index] === "'") {
    const end = scanQuotedName(text, index);
    if (end < 0) return undefined;
    sheet = text.slice(index + 1, end - 1).replaceAll("''", "'");
    index = end;
  } else {
    const match = /^[A-Za-z0-9_.]+(?::[A-Za-z0-9_.]+)?/.exec(text.slice(index));
    if (match === null || match[0] === '') return undefined;
    sheet = match[0];
    index += match[0].length;
  }
  if (text[index] !== '!') return undefined;
  return book === undefined ? { end: index + 1, sheet } : { end: index + 1, sheet, book };
}

/** A structured reference's bracketed part: `[Column]`, `[[#Data],[Amount]]`. */
function scanBrackets(text: string, start: number): number {
  let depth = 0;
  let index = start;
  while (index < text.length) {
    const character = text[index];
    if (character === '[') depth += 1;
    else if (character === ']') {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
    index += 1;
  }
  return -1;
}

function referenceToken(
  text: string,
  from: number,
  to: number,
  bodyFrom: number,
  qualifier: Qualifier | undefined,
): FormulaToken {
  const parsed = parseCellRange(text.slice(bodyFrom, to));
  if (!parsed.ok) return { kind: 'name', text: text.slice(from, to), from, to };
  const parts: ReferenceParts = {
    shape: parsed.value.shape,
    bounds: normalizedBounds(parsed.value),
    ...(qualifier === undefined ? {} : { sheet: qualifier.sheet }),
    ...(qualifier?.book === undefined ? {} : { book: qualifier.book }),
  };
  return { kind: 'reference', text: text.slice(from, to), from, to, reference: parts };
}

/**
 * The whole formula, as runs.
 *
 * One left-to-right pass, no backtracking beyond a token's own body, and **it never throws**: this
 * reads a half-typed expression on every keystroke, so every malformed shape has a token rather
 * than an exception. An unterminated string runs to the end and says so; anything the scanner
 * cannot name at all is one `unknown` character, so the offsets always tile the source exactly.
 */
export function tokeniseFormula(source: string): readonly FormulaToken[] {
  const tokens: FormulaToken[] = [];
  let index = 0;

  if (source.startsWith('=')) {
    tokens.push({ kind: 'equals', text: '=', from: 0, to: 1 });
    index = 1;
  }

  while (index < source.length) {
    const character = source[index] ?? '';
    const start = index;

    // Whitespace. Excel's intersection operator is also a space; it is drawn as whitespace here
    // because a formula bar's job is to show what was typed, and calling one space an operator and
    // the next one padding would be a claim this child cannot check without evaluating.
    if (/\s/.test(character)) {
      while (index < source.length && /\s/.test(source[index] ?? '')) index += 1;
      tokens.push({ kind: 'whitespace', text: source.slice(start, index), from: start, to: index });
      continue;
    }

    // A string. THE token this child turns on.
    if (character === '"') {
      index += 1;
      let closed = false;
      while (index < source.length) {
        if (source[index] === '"') {
          if (source[index + 1] === '"') {
            index += 2;
            continue;
          }
          index += 1;
          closed = true;
          break;
        }
        index += 1;
      }
      const token: FormulaToken = closed
        ? { kind: 'string', text: source.slice(start, index), from: start, to: index }
        : { kind: 'string', text: source.slice(start, index), from: start, to: index, unterminated: true };
      tokens.push(token);
      continue;
    }

    if (character === '#') {
      const literal = errorLiterals.find((candidate) => source.startsWith(candidate, index));
      if (literal !== undefined) {
        index += literal.length;
        tokens.push({ kind: 'errorValue', text: literal, from: start, to: index });
        continue;
      }
    }

    if (character === '(') {
      index += 1;
      tokens.push({ kind: 'openParen', text: '(', from: start, to: index });
      continue;
    }
    if (character === ')') {
      index += 1;
      tokens.push({ kind: 'closeParen', text: ')', from: start, to: index });
      continue;
    }
    if (character === ',' || character === ';') {
      index += 1;
      tokens.push({ kind: 'separator', text: character, from: start, to: index });
      continue;
    }

    if (/[0-9]/.test(character) || (character === '.' && /[0-9]/.test(source[index + 1] ?? ''))) {
      const rows = scanReferenceBody(source, index);
      if (rows > 0 && source.slice(index, rows).includes(':')) {
        index = rows;
        tokens.push(referenceToken(source, start, index, start, undefined));
        continue;
      }
      const match = /^\d*\.?\d+(?:[eE][+-]?\d+)?/.exec(source.slice(index));
      const digits = match?.[0] ?? character;
      index += digits.length;
      tokens.push({ kind: 'number', text: digits, from: start, to: index });
      continue;
    }

    if (character === '$' || character === "'" || character === '[' || isIdentifierStart(character)) {
      const qualifier = scanQualifier(source, index);
      const bodyFrom = qualifier?.end ?? index;
      const body = scanReferenceBody(source, bodyFrom);
      if (body > 0) {
        index = body;
        tokens.push(referenceToken(source, start, index, bodyFrom, qualifier));
        continue;
      }
      if (qualifier !== undefined) {
        // A qualified thing that is not a range: `Sheet1!Total`, a sheet-scoped defined name.
        const match = /^[A-Za-z0-9_.\\]+/.exec(source.slice(qualifier.end));
        index = qualifier.end + (match?.[0].length ?? 0);
        tokens.push({ kind: 'name', text: source.slice(start, index), from: start, to: index });
        continue;
      }
      if (isIdentifierStart(character)) {
        index += 1;
        while (index < source.length && isIdentifierPart(source[index] ?? '')) index += 1;
        const word = source.slice(start, index);
        if (source[index] === '[') {
          const close = scanBrackets(source, index);
          index = close < 0 ? source.length : close;
          tokens.push({ kind: 'structured', text: source.slice(start, index), from: start, to: index });
          continue;
        }
        if (source[index] === '(') {
          tokens.push({ kind: 'functionName', text: word, from: start, to: index });
          continue;
        }
        const upper = word.toUpperCase();
        if (upper === 'TRUE' || upper === 'FALSE') {
          tokens.push({ kind: 'boolean', text: word, from: start, to: index });
          continue;
        }
        tokens.push({ kind: 'name', text: word, from: start, to: index });
        continue;
      }
    }

    const long = longOperators.find((candidate) => source.startsWith(candidate, index));
    if (long !== undefined) {
      index += long.length;
      tokens.push({ kind: 'operator', text: long, from: start, to: index });
      continue;
    }
    if (shortOperators.has(character)) {
      index += 1;
      tokens.push({ kind: 'operator', text: character, from: start, to: index });
      continue;
    }

    index += 1;
    tokens.push({ kind: 'unknown', text: character, from: start, to: index });
  }

  return tokens;
}

/** Whether the text is a formula at all — Excel's rule, one character long. */
export function isFormula(source: string): boolean {
  return source.startsWith('=');
}

// ── the reference-colouring contract ───────────────────────────────────────────────────────────

/**
 * A colour a reference can be drawn in, named once and resolved per scheme.
 *
 * ⚠ **Two tokens per slot, not one, and it is not a convenience.** This palette is warm and small,
 * and a colour that clears 4.5 : 1 on the light surface is usually the same colour that fails on
 * the dark one — `color.clay` is 5.23 : 1 light and 2.27 : 1 dark. A single token per slot would
 * therefore have made half the ring illegible in whichever scheme it was not chosen for, silently,
 * because the a11y sweep runs in the light scheme. So a slot is an **identity** — *the first
 * reference*, *the second* — and each scheme spells it with the token that is legible there.
 * `tests/formula.test.ts` measures every one of the eight against its own scheme's surface.
 */
export interface ReferenceColourSlot {
  /** What this slot is called, in the contract and in a story's readout. */
  readonly name: string;
  /** The token drawn on a light surface. */
  readonly light: TokenPath;
  /** The token drawn on a dark one. */
  readonly dark: TokenPath;
}

/**
 * The ring, and **four is a measurement rather than a taste.**
 *
 * Excel cycles about seven colours. Four is what this palette can spell at 4.5 : 1 *in both
 * schemes at once* — two chromatic pairs and two neutral ones — and a fifth slot would have had to
 * be a colour nobody had measured. A formula with more than four distinct references reuses the
 * ring from the beginning, which the contract states rather than hides: two references that share a
 * slot are drawn alike, and [`referenceHighlights`] says which.
 */
export const referenceColourSlots: readonly ReferenceColourSlot[] = [
  { name: 'first', light: 'color.greenDeep', dark: 'color.greenLifted' },
  { name: 'second', light: 'color.clay', dark: 'color.honey' },
  { name: 'third', light: 'theme.light.textSecondary', dark: 'theme.dark.textSecondary' },
  { name: 'fourth', light: 'color.ink', dark: 'theme.dark.textPrimary' },
];

/** The custom property a slot's colour is read from. Written by the component, read by the gate. */
export function referenceSlotProperty(slot: number): string {
  return `--mjx-formula-reference-${String(slot)}`;
}

/**
 * **The contract loop 2 binds to.** One entry per reference *occurrence* in the formula.
 *
 * The in-canvas grid highlight is not in this child's scope; what is in scope is making that
 * binding trivial, which means the component must say — for each reference, in source order —
 * where it sits in the text, which cells it names, and which colour it was drawn in. A grid that
 * had to re-tokenise the formula to find that out would be a second tokeniser, and the first defect
 * would be the two disagreeing about a comma inside a string.
 */
export interface ReferenceHighlight {
  /** The reference exactly as written, `$` and sheet name included. */
  readonly text: string;
  /** Where it sits in the source, half-open. */
  readonly from: number;
  readonly to: number;
  /** Which colour slot it was drawn in — an index into [`referenceColourSlots`]. */
  readonly slot: number;
  /** The sheet it named, when it named one. */
  readonly sheet?: string;
  /** The external book index, when it came from one. */
  readonly book?: string;
  /** The cells it covers, ordered. What the grid draws a box around. */
  readonly bounds: GridBounds;
  /** Which of the four shapes it was written in. */
  readonly shape: RangeShape;
}

/**
 * The key that decides whether two references are *the same range* and therefore one colour.
 *
 * ⚠ **Anchoring is deliberately not part of it.** `A1` and `$A$1` name the same cell and differ
 * only in what happens when the formula is copied, so a grid drawing one box for each would draw
 * two boxes around one cell in two colours. The sheet **is** part of it, because `Sheet1!A1` and
 * `Sheet2!A1` are different cells.
 */
export function referenceIdentity(parts: ReferenceParts): string {
  const sheet = parts.sheet === undefined ? '' : parts.sheet.toUpperCase();
  const book = parts.book === undefined ? '' : parts.book;
  const { firstColumn, lastColumn, firstRow, lastRow } = parts.bounds;
  return `${book}|${sheet}|${String(firstColumn)}:${String(lastColumn)}:${String(firstRow)}:${String(lastRow)}`;
}

/**
 * Every reference in the formula, in source order, with the colour each carries.
 *
 * Two occurrences of the same range get the **same** slot however they were spelled; a fifth
 * distinct range wraps to the first slot.
 */
export function referenceHighlights(source: string): readonly ReferenceHighlight[] {
  const assigned = new Map<string, number>();
  const highlights: ReferenceHighlight[] = [];
  for (const token of tokeniseFormula(source)) {
    const parts = token.reference;
    if (token.kind !== 'reference' || parts === undefined) continue;
    const identity = referenceIdentity(parts);
    let slot = assigned.get(identity);
    if (slot === undefined) {
      slot = assigned.size % referenceColourSlots.length;
      assigned.set(identity, slot);
    }
    highlights.push({
      text: token.text,
      from: token.from,
      to: token.to,
      slot,
      bounds: parts.bounds,
      shape: parts.shape,
      ...(parts.sheet === undefined ? {} : { sheet: parts.sheet }),
      ...(parts.book === undefined ? {} : { book: parts.book }),
    });
  }
  return highlights;
}

// ── bracket matching ───────────────────────────────────────────────────────────────────────────

/** A matched pair of parentheses, by offset. */
export interface BracketPair {
  /** The offset of the `(`. */
  readonly open: number;
  /** The offset of the `)`. */
  readonly close: number;
}

/** Every matched pair in the formula, and every parenthesis that has no partner. */
export interface BracketReport {
  readonly pairs: readonly BracketPair[];
  /** Offsets of parentheses with nothing to match — an unclosed `(` or a stray `)`. */
  readonly unmatched: readonly number[];
}

/**
 * Which parentheses pair with which.
 *
 * Token-driven, so `="("` has no open parenthesis in it at all. The naive character scan gets that
 * wrong in the same way it gets the argument wrong, which is why [`naiveBracketReport`] exists
 * beside it.
 */
export function bracketReport(source: string): BracketReport {
  const stack: number[] = [];
  const pairs: BracketPair[] = [];
  const unmatched: number[] = [];
  for (const token of tokeniseFormula(source)) {
    if (token.kind === 'openParen') stack.push(token.from);
    else if (token.kind === 'closeParen') {
      const open = stack.pop();
      if (open === undefined) unmatched.push(token.from);
      else pairs.push({ open, close: token.from });
    }
  }
  unmatched.push(...stack);
  return { pairs, unmatched: [...unmatched].sort((first, second) => first - second) };
}

/**
 * The pair the caret is *on*, or `undefined`.
 *
 * Excel's rule and it is narrower than *"the pair I am inside"*: a caret immediately after a `(`
 * or immediately before or after its `)` lights both, and a caret in the middle of the arguments
 * lights nothing. A bar that emboldened a pair for the whole time the caret was anywhere inside it
 * would be a bar with a permanently emboldened outer pair.
 */
export function bracketAt(source: string, caret: number): BracketPair | undefined {
  const { pairs } = bracketReport(source);
  return pairs.find(
    (pair) =>
      caret === pair.open ||
      caret === pair.open + 1 ||
      caret === pair.close ||
      caret === pair.close + 1,
  );
}

/**
 * ⚠ **A positive control, not an implementation.** Character-by-character, string-blind.
 *
 * Kept beside the real one so `tests/formula.test.ts` can show the table rejecting something. A
 * gate whose failing case is not in the repository is a gate nobody has watched fail.
 */
export function naiveBracketReport(source: string): BracketReport {
  const stack: number[] = [];
  const pairs: BracketPair[] = [];
  const unmatched: number[] = [];
  for (let index = 0; index < source.length; index += 1) {
    if (source[index] === '(') stack.push(index);
    else if (source[index] === ')') {
      const open = stack.pop();
      if (open === undefined) unmatched.push(index);
      else pairs.push({ open, close: index });
    }
  }
  unmatched.push(...stack);
  return { pairs, unmatched: [...unmatched].sort((first, second) => first - second) };
}

// ── the argument tooltip: which argument is the caret in ───────────────────────────────────────

/** Where the caret is, in a function call's terms. */
export interface ActiveArgument {
  /** The function whose parentheses the caret is inside, as written. */
  readonly functionName: string;
  /** The offset of that call's `(`. */
  readonly openParen: number;
  /** Which argument, counting from zero. */
  readonly argumentIndex: number;
  /** How many *named* calls enclose this one. Zero for the outermost. */
  readonly depth: number;
  /** Where this argument's text begins and ends, half-open. */
  readonly from: number;
  readonly to: number;
}

interface Frame {
  name: string;
  open: number;
  argumentIndex: number;
  argumentFrom: number;
}

/**
 * **The child's central function.** Which argument of which call the caret sits in.
 *
 * Three things it gets right that a comma count does not, and each of them is a row in the gate's
 * table:
 *
 * 1. **A comma inside a quoted string is not a separator.** `TEXT(B2,"#,##0")` has two arguments,
 *    and a caret after the format string is still in the second one.
 * 2. **Nesting is a stack.** A caret inside `COUNTIF`'s parentheses is in `COUNTIF`'s second
 *    argument and *not* in `IF`'s — the innermost call wins, and `depth` says how deep it is.
 * 3. **A bare grouping parenthesis is not a call.** `SUM((A1,B1))`'s inner comma is the union
 *    operator; it belongs to the group, not to `SUM`, so `SUM` still shows argument one. The
 *    innermost **named** frame is the answer, which is what makes that fall out rather than need a
 *    special case.
 *
 * `undefined` when the caret is not inside any named call — outside the parentheses, or in a
 * formula with none.
 */
export function activeArgument(source: string, caret: number): ActiveArgument | undefined {
  const tokens = tokeniseFormula(source);
  const position = Math.max(0, Math.min(caret, source.length));
  const stack: Frame[] = [];
  let pending: string | undefined;

  for (const token of tokens) {
    // A token whose first character is at or after the caret is entirely ahead of it: the caret
    // sits BETWEEN characters, so a `(` at the caret's own offset has not been passed yet.
    if (token.from >= position) break;
    switch (token.kind) {
      case 'functionName':
        pending = token.text;
        break;
      case 'openParen':
        stack.push({ name: pending ?? '', open: token.from, argumentIndex: 0, argumentFrom: token.to });
        pending = undefined;
        break;
      case 'closeParen':
        stack.pop();
        break;
      case 'separator': {
        const top = stack[stack.length - 1];
        if (top !== undefined) {
          top.argumentIndex += 1;
          top.argumentFrom = token.to;
        }
        break;
      }
      default:
        break;
    }
  }

  let depth = -1;
  let frame: Frame | undefined;
  for (let index = stack.length - 1; index >= 0; index -= 1) {
    const candidate = stack[index];
    if (candidate !== undefined && candidate.name !== '') {
      frame = candidate;
      depth = index;
      break;
    }
  }
  if (frame === undefined) return undefined;

  // How far the argument runs: forward to the next separator or close at this frame's own depth.
  let relative = 0;
  let end = source.length;
  for (const token of tokens) {
    if (token.from < position) continue;
    if (token.kind === 'openParen') relative += 1;
    else if (token.kind === 'closeParen') {
      if (relative === 0) {
        end = token.from;
        break;
      }
      relative -= 1;
    } else if (token.kind === 'separator' && relative === 0) {
      end = token.from;
      break;
    }
  }

  return {
    functionName: frame.name,
    openParen: frame.open,
    argumentIndex: frame.argumentIndex,
    depth,
    from: frame.argumentFrom,
    to: Math.max(frame.argumentFrom, end),
  };
}

/**
 * ⚠ **A positive control, not an implementation.** The comma count the ticket says passes the
 * simple case and fails the quoted one.
 *
 * It reads *characters*: every `(` opens a call, every `,` separates an argument, and a quotation
 * mark is a character like any other. Shipped beside [`activeArgument`] on purpose — the caret
 * table drives **both**, asserts the real one against the expected column, and asserts that this
 * one **differs** at the offsets a quoted comma moves. Without it, the table is twelve assertions
 * that a correct implementation satisfies and so does any other.
 */
export function naiveActiveArgument(
  source: string,
  caret: number,
): { readonly functionName: string; readonly argumentIndex: number } | undefined {
  const position = Math.max(0, Math.min(caret, source.length));
  const stack: { name: string; argumentIndex: number }[] = [];
  for (let index = 0; index < position; index += 1) {
    const character = source[index];
    if (character === '(') {
      const before = /[A-Za-z_][A-Za-z0-9_.]*$/.exec(source.slice(0, index));
      stack.push({ name: before?.[0] ?? '', argumentIndex: 0 });
    } else if (character === ')') stack.pop();
    else if (character === ',') {
      const top = stack[stack.length - 1];
      if (top !== undefined) top.argumentIndex += 1;
    }
  }
  for (let index = stack.length - 1; index >= 0; index -= 1) {
    const candidate = stack[index];
    if (candidate !== undefined && candidate.name !== '') {
      return { functionName: candidate.name, argumentIndex: candidate.argumentIndex };
    }
  }
  return undefined;
}

// ── the function catalogue ─────────────────────────────────────────────────────────────────────

/** One parameter of a function. */
export interface FunctionParameter {
  readonly name: string;
  /** Whether the function can be called without it. */
  readonly optional?: true;
  /**
   * Whether it stands for *"and as many more as you like"*.
   *
   * ⚠ The reason the tooltip needs this: `SUM`'s seventh argument must emphasise `number2`, not
   * run off the end of the list and emphasise nothing. See [`emphasisedParameter`].
   */
  readonly repeating?: true;
}

/** A function's signature and what it is for. Data, not behaviour — see [`formulaFunctions`]. */
export interface FunctionSignature {
  readonly name: string;
  readonly parameters: readonly FunctionParameter[];
  readonly summary: string;
  readonly category: string;
}

/**
 * A starter catalogue, and **it is data a host replaces**.
 *
 * `GUESS:` the summaries are this catalogue's own wording, not Microsoft's. Excel ships around five
 * hundred functions; shipping a transcription of all of them here would be a large, stale copy of
 * somebody else's reference material inside a component library. `<mjx-formula-bar>` takes any
 * catalogue through its `functions` property, so a host with the real list loses nothing — and this
 * one is large enough that the autocomplete's filtering, its keyboard and the tooltip's argument
 * emphasis are exercised against something real.
 *
 * **No function here computes anything.** There is no calculation engine in this loop and this
 * component must not imply one: a signature is a *label*, and the bar never evaluates a formula,
 * never reports a result, and has nowhere to put one.
 */
export const formulaFunctions: readonly FunctionSignature[] = [
  {
    name: 'AVERAGE',
    parameters: [{ name: 'number1' }, { name: 'number2', optional: true, repeating: true }],
    summary: 'The arithmetic mean of its arguments.',
    category: 'Statistical',
  },
  {
    name: 'AVERAGEIF',
    parameters: [{ name: 'range' }, { name: 'criteria' }, { name: 'average_range', optional: true }],
    summary: 'The mean of the cells that meet one condition.',
    category: 'Statistical',
  },
  {
    name: 'CONCAT',
    parameters: [{ name: 'text1' }, { name: 'text2', optional: true, repeating: true }],
    summary: 'Joins its arguments into one piece of text.',
    category: 'Text',
  },
  {
    name: 'COUNT',
    parameters: [{ name: 'value1' }, { name: 'value2', optional: true, repeating: true }],
    summary: 'How many of its arguments are numbers.',
    category: 'Statistical',
  },
  {
    name: 'COUNTA',
    parameters: [{ name: 'value1' }, { name: 'value2', optional: true, repeating: true }],
    summary: 'How many of its arguments are not empty.',
    category: 'Statistical',
  },
  {
    name: 'COUNTIF',
    parameters: [{ name: 'range' }, { name: 'criteria' }],
    summary: 'How many cells in a range meet one condition.',
    category: 'Statistical',
  },
  {
    name: 'IF',
    parameters: [
      { name: 'logical_test' },
      { name: 'value_if_true', optional: true },
      { name: 'value_if_false', optional: true },
    ],
    summary: 'One value when a test holds, another when it does not.',
    category: 'Logical',
  },
  {
    name: 'IFERROR',
    parameters: [{ name: 'value' }, { name: 'value_if_error' }],
    summary: 'A stand-in for a value that would otherwise be an error.',
    category: 'Logical',
  },
  {
    name: 'INDEX',
    parameters: [{ name: 'array' }, { name: 'row_num' }, { name: 'column_num', optional: true }],
    summary: 'The value at a position within a range.',
    category: 'Lookup',
  },
  {
    name: 'INDIRECT',
    parameters: [{ name: 'ref_text' }, { name: 'a1', optional: true }],
    summary: 'The range a piece of text names.',
    category: 'Lookup',
  },
  {
    name: 'LEFT',
    parameters: [{ name: 'text' }, { name: 'num_chars', optional: true }],
    summary: 'Characters from the start of a piece of text.',
    category: 'Text',
  },
  {
    name: 'LEN',
    parameters: [{ name: 'text' }],
    summary: 'How many characters a piece of text has.',
    category: 'Text',
  },
  {
    name: 'MATCH',
    parameters: [{ name: 'lookup_value' }, { name: 'lookup_array' }, { name: 'match_type', optional: true }],
    summary: 'The position of a value within a range.',
    category: 'Lookup',
  },
  {
    name: 'MAX',
    parameters: [{ name: 'number1' }, { name: 'number2', optional: true, repeating: true }],
    summary: 'The largest of its arguments.',
    category: 'Statistical',
  },
  {
    name: 'MIN',
    parameters: [{ name: 'number1' }, { name: 'number2', optional: true, repeating: true }],
    summary: 'The smallest of its arguments.',
    category: 'Statistical',
  },
  {
    name: 'ROUND',
    parameters: [{ name: 'number' }, { name: 'num_digits' }],
    summary: 'A number rounded to a given number of digits.',
    category: 'Mathematical',
  },
  {
    name: 'SUM',
    parameters: [{ name: 'number1' }, { name: 'number2', optional: true, repeating: true }],
    summary: 'Adds its arguments together.',
    category: 'Mathematical',
  },
  {
    name: 'SUMIF',
    parameters: [{ name: 'range' }, { name: 'criteria' }, { name: 'sum_range', optional: true }],
    summary: 'Adds the cells that meet one condition.',
    category: 'Mathematical',
  },
  {
    name: 'SUMIFS',
    parameters: [
      { name: 'sum_range' },
      { name: 'criteria_range1' },
      { name: 'criteria1' },
      { name: 'criteria_range2', optional: true, repeating: true },
    ],
    summary: 'Adds the cells that meet every one of several conditions.',
    category: 'Mathematical',
  },
  {
    name: 'TEXT',
    parameters: [{ name: 'value' }, { name: 'format_text' }],
    summary: 'A value written out in a given number format.',
    category: 'Text',
  },
  {
    name: 'TEXTJOIN',
    parameters: [
      { name: 'delimiter' },
      { name: 'ignore_empty' },
      { name: 'text1' },
      { name: 'text2', optional: true, repeating: true },
    ],
    summary: 'Joins pieces of text with a delimiter between them.',
    category: 'Text',
  },
  {
    name: 'VLOOKUP',
    parameters: [
      { name: 'lookup_value' },
      { name: 'table_array' },
      { name: 'col_index_num' },
      { name: 'range_lookup', optional: true },
    ],
    summary: 'Looks a value up in the first column of a range.',
    category: 'Lookup',
  },
  {
    name: 'XLOOKUP',
    parameters: [
      { name: 'lookup_value' },
      { name: 'lookup_array' },
      { name: 'return_array' },
      { name: 'if_not_found', optional: true },
      { name: 'match_mode', optional: true },
    ],
    summary: 'Looks a value up in one range and returns from another.',
    category: 'Lookup',
  },
];

/** A signature as one line: `SUM(number1, [number2], …)`. */
export function signatureText(signature: FunctionSignature): string {
  const parts = signature.parameters.map((parameter) => {
    const name = parameter.optional === true ? `[${parameter.name}]` : parameter.name;
    return parameter.repeating === true ? `${name}, …` : name;
  });
  return `${signature.name}(${parts.join(', ')})`;
}

/**
 * Which parameter the tooltip emphasises for a given argument index.
 *
 * ⚠ **The repeating tail is why this is a function and not an array index.** `SUM`'s eighth
 * argument is still `number2`; a tooltip that indexed straight into the list would emphasise
 * nothing at all from the third argument onward, which is exactly when a person most needs it.
 * Returns `-1` when the call has more arguments than the signature allows and none of them repeat —
 * a real state, and the honest way to say *this call has too many arguments* without evaluating it.
 */
export function emphasisedParameter(signature: FunctionSignature, argumentIndex: number): number {
  const count = signature.parameters.length;
  if (argumentIndex < 0 || count === 0) return -1;
  if (argumentIndex < count) return argumentIndex;
  const last = signature.parameters[count - 1];
  return last?.repeating === true ? count - 1 : -1;
}

/** The signature by name, case-insensitively — a person may type `sum(`. */
export function signatureFor(
  name: string,
  catalogue: readonly FunctionSignature[] = formulaFunctions,
): FunctionSignature | undefined {
  const wanted = name.trim().toUpperCase();
  return catalogue.find((candidate) => candidate.name === wanted);
}

/**
 * The word the caret is in the middle of typing, if it could still become a function name.
 *
 * `undefined` in every state where an autocomplete would be an interruption: outside a formula, in
 * the middle of a string, inside a reference, or immediately after a `(` that has already been
 * typed. What is left is exactly *a bare identifier run ending at the caret*.
 */
export function completionPrefix(source: string, caret: number): { readonly text: string; readonly from: number } | undefined {
  if (!isFormula(source)) return undefined;
  const position = Math.max(0, Math.min(caret, source.length));
  for (const token of tokeniseFormula(source)) {
    if (token.from >= position) break;
    if (token.to < position) continue;
    if (token.kind !== 'name') return undefined;
    const text = source.slice(token.from, position);
    return text === '' ? undefined : { text, from: token.from };
  }
  return undefined;
}

/**
 * The functions a prefix could still become, in catalogue order.
 *
 * `startsWith`, deliberately, where U07's combo box defaults to `contains`: a person typing `SU`
 * means *a function that starts with SU*, and a contains-filter would offer `TEXTJOIN` for `EX`.
 * U07's `filterOptions` is not reused here for exactly that reason — the mode is different, not the
 * mechanism — and the list it feeds **is** U07's `ListSurface`.
 */
export function completionsFor(
  prefix: string,
  catalogue: readonly FunctionSignature[] = formulaFunctions,
): readonly FunctionSignature[] {
  const wanted = prefix.trim().toUpperCase();
  if (wanted === '') return [];
  return catalogue.filter((candidate) => candidate.name.startsWith(wanted));
}

/** What a completion does to the text: the name, its `(`, and where the caret lands after. */
export function applyCompletion(
  source: string,
  prefix: { readonly text: string; readonly from: number },
  signature: FunctionSignature,
): { readonly text: string; readonly caret: number } {
  const before = source.slice(0, prefix.from);
  const after = source.slice(prefix.from + prefix.text.length);
  const inserted = `${signature.name}(`;
  return { text: `${before}${inserted}${after}`, caret: before.length + inserted.length };
}

// ── the four modes ─────────────────────────────────────────────────────────────────────────────

/**
 * Excel's four, and a person who has used Excel depends on the distinction: `Ready` and `Enter`
 * differ in whether the arrow keys move the selection or the caret, and getting that wrong loses
 * what somebody typed.
 */
export const formulaModeNames = ['ready', 'enter', 'edit', 'point'] as const;

/** One of the four. */
export type FormulaMode = (typeof formulaModeNames)[number];

/** What each mode is called and what it means. The label is the word Excel puts in its status bar. */
export interface FormulaModeSpec {
  readonly label: string;
  readonly description: string;
  /** What a screen reader is told on the way in. */
  readonly announcement: string;
}

/**
 * The four, with their announcements.
 *
 * ⚠ **A mode a screen-reader user cannot hear is a mode that does not exist for them**, and the
 * distinction is not decoration: in `Point` mode the arrow keys build a reference and in `Edit`
 * mode they move the caret, so a person who cannot tell which one they are in cannot predict what
 * the next key will do. The announcement is a sentence rather than a word for the same reason.
 */
export const formulaModes: Readonly<Record<FormulaMode, FormulaModeSpec>> = {
  ready: {
    label: 'Ready',
    description: 'Nothing is being edited. The arrow keys move the selection.',
    announcement: 'Ready. The arrow keys move the selection.',
  },
  enter: {
    label: 'Enter',
    description: 'Typing replaces what the cell held.',
    announcement: 'Enter mode. Typing replaces the cell’s contents.',
  },
  edit: {
    label: 'Edit',
    description: 'Editing what the cell already held. The arrow keys move the caret.',
    announcement: 'Edit mode. The arrow keys move the caret.',
  },
  point: {
    label: 'Point',
    description: 'Choosing a range. The arrow keys build a reference into the formula.',
    announcement: 'Point mode. The arrow keys choose a range.',
  },
};

/** What can move the bar between modes. */
export const formulaModeTriggerNames = [
  'beginEntry',
  'beginEdit',
  'commit',
  'cancel',
  'enterPoint',
  'leavePoint',
] as const;

/** One of the six. */
export type FormulaModeTrigger = (typeof formulaModeTriggerNames)[number];

/**
 * The mode, and the mode `Point` goes back to.
 *
 * ⚠ **`Point` has to remember where it came from**, and that is the whole reason this is a state
 * rather than a single value. A person who starts typing in an empty cell (`Enter`), points at a
 * range, and then types an operator must land back in `Enter` — not in `Edit`, which would tell
 * them the cell had contents it does not have.
 */
export interface FormulaModeState {
  readonly mode: FormulaMode;
  /** Which mode a `leavePoint` returns to. */
  readonly resume: Extract<FormulaMode, 'enter' | 'edit'>;
}

/** Where a bar starts. */
export const readyModeState: FormulaModeState = { mode: 'ready', resume: 'enter' };

/**
 * One trigger, applied.
 *
 * Total: every trigger has an answer in every mode, and a trigger that means nothing where it
 * arrived leaves the state alone rather than throwing. A `commit` in `Ready` is a person pressing
 * Enter on a cell they are not editing, which happens constantly.
 */
export function applyModeTrigger(state: FormulaModeState, trigger: FormulaModeTrigger): FormulaModeState {
  switch (trigger) {
    case 'beginEntry':
      return state.mode === 'ready' ? { mode: 'enter', resume: 'enter' } : state;
    case 'beginEdit':
      return state.mode === 'ready' ? { mode: 'edit', resume: 'edit' } : state;
    case 'commit':
    case 'cancel':
      return readyModeState;
    case 'enterPoint':
      if (state.mode !== 'enter' && state.mode !== 'edit') return state;
      return { mode: 'point', resume: state.mode };
    case 'leavePoint':
      return state.mode === 'point' ? { mode: state.resume, resume: state.resume } : state;
  }
}

/**
 * Whether the caret sits where an arrow key would start building a reference.
 *
 * Excel's rule, and it is entirely caret-relative: inside a formula, immediately after the `=`, an
 * operator, an opening parenthesis or a separator. Anywhere else — in the middle of a name, after a
 * closing parenthesis, inside a string — an arrow key moves the caret.
 */
export function pointReady(source: string, caret: number): boolean {
  if (!isFormula(source)) return false;
  const position = Math.max(0, Math.min(caret, source.length));
  if (position === 0) return false;
  let last: FormulaToken | undefined;
  for (const token of tokeniseFormula(source)) {
    if (token.from >= position) break;
    if (token.to > position && token.kind !== 'whitespace') return false;
    if (token.kind === 'whitespace') continue;
    last = token;
  }
  if (last === undefined) return false;
  return (
    last.kind === 'equals' ||
    last.kind === 'operator' ||
    last.kind === 'openParen' ||
    last.kind === 'separator'
  );
}

// ── multi-line expansion ───────────────────────────────────────────────────────────────────────

/**
 * How tall the editor may be, in rows.
 *
 * `SplitterBounds` from U11's foundation, reused as-is: the type is `{ min, max }` over a number
 * and carries no assumption that the number is a fraction. `clampToBounds` is reused with it.
 *
 * ⚠ **`keyFraction` is deliberately NOT reused**, and the reason is a real difference rather than
 * an oversight: it maps `ArrowLeft`/`ArrowRight` against a `left`/`right` side, because a window
 * splitter moves on the inline axis. This handle moves on the **block** axis, so its keys are
 * `ArrowUp`/`ArrowDown` and there is no side to mirror in Arabic. Reusing it would have meant
 * passing a fake side and then translating two keys, which is more code than the four lines below
 * and would have made `keyFraction` answer a question it was not asked.
 */
export const formulaRowBounds: SplitterBounds = { min: 1, max: 12 };

/** How tall the editor is when a person expands it without saying how far. */
export const expandedFormulaRows = 3;

/** A row count, clamped. U11's `clampToBounds`, with this child's bounds. */
export function clampFormulaRows(rows: number): number {
  return Math.round(clampToBounds(rows, formulaRowBounds, formulaRowBounds.min));
}

/**
 * What one key does to the editor's height, or `undefined` for a key that is not the handle's.
 *
 * ⚠ Returning `undefined` rather than the current value is load-bearing: a handle that claimed
 * every key would swallow `Tab`, and a `separator` a person cannot Tab out of is a trap. U11's
 * `keyFraction` states the same rule for the same reason.
 */
export function rowsAfterKey(key: string, current: number): number | undefined {
  switch (key) {
    case 'ArrowDown':
      return clampFormulaRows(current + 1);
    case 'ArrowUp':
      return clampFormulaRows(current - 1);
    case 'Home':
      return formulaRowBounds.min;
    case 'End':
      return formulaRowBounds.max;
    case 'Enter':
    case ' ':
      return current > formulaRowBounds.min ? formulaRowBounds.min : expandedFormulaRows;
    default:
      return undefined;
  }
}

/** How many rows a pointer drag of `delta` CSS pixels asks for, given a row's height. */
export function rowsAfterDrag(startRows: number, delta: number, rowHeight: number): number {
  if (rowHeight <= 0) return clampFormulaRows(startRows);
  return clampFormulaRows(startRows + Math.round(delta / rowHeight));
}

// ── the name box ───────────────────────────────────────────────────────────────────────────────

/** A name or table the name box lists, and what it points at. */
export interface DefinedNameEntry {
  /** What a person sees and types. */
  readonly name: string;
  /** Where it points, as text — `Summary!$B$1`, `SUM(Sheet1!A:A)`, anything. */
  readonly definition: string;
  /** `name` for a defined name, `table` for a table. Two icons, two groups in the list. */
  readonly kind: 'name' | 'table';
  /** The sheet it is scoped to, when it is not workbook-wide. */
  readonly scope?: string;
}

/** What typing into the name box asked for. */
export type NavigationRequest =
  | { readonly kind: 'address'; readonly text: string; readonly range: CellRange; readonly bounds: GridBounds; readonly sheet?: string }
  | { readonly kind: 'name'; readonly text: string; readonly entry: DefinedNameEntry }
  | { readonly kind: 'problem'; readonly text: string; readonly problem: AddressProblem };

/**
 * What a typed string in the name box means.
 *
 * ⚠ **A name is looked up before the address grammar is tried**, and the order matters: a workbook
 * may hold a defined name spelled like an address, and Excel's own rule is that the name wins —
 * so does this, because the alternative silently sends a person somewhere other than the place
 * their own workbook says that word means. Deferring to the user's document rather than to our
 * grammar is the standing rule; this is that rule in one `if`.
 */
export function navigationFor(
  typed: string,
  names: readonly DefinedNameEntry[],
): NavigationRequest {
  const trimmed = typed.trim();
  if (trimmed === '') return { kind: 'problem', text: trimmed, problem: 'empty' };

  const wanted = trimmed.toUpperCase();
  const entry = names.find((candidate) => candidate.name.toUpperCase() === wanted);
  if (entry !== undefined) return { kind: 'name', text: entry.name, entry };

  const normalised = normaliseTypedAddress(trimmed);
  const bang = normalised.lastIndexOf('!');
  const sheet = bang < 0 ? undefined : normalised.slice(0, bang).replaceAll("'", '');
  const body = bang < 0 ? normalised : normalised.slice(bang + 1);
  if (bang >= 0 && sheet === '') return { kind: 'problem', text: trimmed, problem: 'emptySheetName' };

  const range = parseCellRange(body);
  if (!range.ok) return { kind: 'problem', text: trimmed, problem: range.problem };
  return {
    kind: 'address',
    text: normalised,
    range: range.value,
    bounds: normalizedBounds(range.value),
    ...(sheet === undefined ? {} : { sheet }),
  };
}

/** How a selection is displayed when nothing is being typed: `B7`, or `A1:C9` with its size. */
export function selectionDisplay(range: CellRange): string {
  return rangeText(range);
}

/** What a multi-cell selection is announced as, since its size is not in its address. */
export function selectionAnnouncement(range: CellRange): string {
  const bounds = normalizedBounds(range);
  if (range.shape === 'cell') return rangeText(range);
  const rows = bounds.lastRow - bounds.firstRow + 1;
  const columns = bounds.lastColumn - bounds.firstColumn + 1;
  return `${rangeText(range)}, ${String(rows)} by ${String(columns)}`;
}

// ── the events both components speak ───────────────────────────────────────────────────────────

/**
 * Every event name this child emits, in one table.
 *
 * `references` is the one loop 2 binds to and it is deliberately not folded into a change event:
 * the highlights change when the *text* changes and also when nothing changes but the catalogue
 * does, and a grid that had to re-derive them from a change event would be re-tokenising.
 */
export const formulaEvents = {
  /** The mode changed. `detail: { mode, previous, announcement }`. */
  mode: 'mjx-formula-mode',
  /** The reference-colouring contract. `detail: { references, slots }`. */
  references: 'mjx-formula-references',
  /** The caret moved, or the text under it changed. `detail: { caret, argument, brackets }`. */
  caret: 'mjx-formula-caret',
  /** A formula was accepted. `detail: { text }`. */
  commit: 'mjx-formula-commit',
  /** An edit was abandoned. `detail: { text }` — the text that was thrown away. */
  cancel: 'mjx-formula-cancel',
  /** The `fx` affordance was pressed. `detail: {}`. */
  insertFunction: 'mjx-formula-insert-function',
  /** The editor's height changed. `detail: { rows }`. */
  rows: 'mjx-formula-rows',
  /** The name box was asked to go somewhere. `detail: NavigationRequest`. */
  navigate: 'mjx-name-navigate',
} as const;

/** The two tags this child registers. */
export const formulaTags = {
  bar: 'mjx-formula-bar',
  nameBox: 'mjx-name-box',
} as const;

/** The story titles, so a browser gate names a story once. */
export const formulaStoryTitles = {
  bar: 'Excel Chrome/Formula Bar',
  nameBox: 'Excel Chrome/Name Box',
} as const;
