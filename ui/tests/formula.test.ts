import { describe, expect, it } from 'vitest';

import { contrastRatio, bodyTextMinimum } from '../dev/contrast.ts';
import {
  activeArgument,
  applyCompletion,
  applyModeTrigger,
  addressProblemMessages,
  addressProblemNames,
  bracketAt,
  bracketReport,
  cellAddressText,
  cellCount,
  clampFormulaRows,
  columnCount,
  columnIndexFromLetters,
  columnLetters,
  completionPrefix,
  completionsFor,
  emphasisedParameter,
  expandedFormulaRows,
  formulaEvents,
  formulaFunctions,
  formulaModeNames,
  formulaModes,
  formulaRowBounds,
  isFormula,
  lastColumnIndex,
  lastRowIndex,
  naiveActiveArgument,
  naiveBracketReport,
  navigationFor,
  normalizedBounds,
  normaliseTypedAddress,
  parseCellRange,
  parseCellReference,
  pointReady,
  rangeText,
  readyModeState,
  referenceColourSlots,
  referenceHighlights,
  referenceIdentity,
  rowCount,
  rowsAfterDrag,
  rowsAfterKey,
  selectionAnnouncement,
  signatureFor,
  signatureText,
  tokeniseFormula,
  type DefinedNameEntry,
  type FormulaMode,
  type FormulaModeState,
} from '../src/formula/formula-model.ts';
import { generatedDefault } from '../src/tokens/resolver.ts';
import { tokens } from '../tokens/tokens.ts';

/**
 * MJXOFF-192's model, and **the caret-position table is the whole suite**.
 *
 * The ticket's trap, in its own words: *"every hard behaviour here is caret-relative, and a story
 * showing a static formula proves none of it … a naive comma-counting implementation passes the
 * simple case and fails the quoted one."*
 *
 * So the table below is driven through **both** implementations — the real one and the shipped
 * positive control, `naiveActiveArgument` — and the second half of the assertion is that they
 * **disagree**, by name, at the offsets a quoted comma moves. A table that only asserted the real
 * answer would be twelve rows that a correct implementation satisfies and so does any other; this
 * one cannot be satisfied by a comma count, and the repository contains the proof that it cannot.
 */

// ── the address grammar, against crates/mjx-sml/src/address.rs ─────────────────────────────────

describe('the address grammar mirrors mjx-sml', () => {
  it('carries the same two grid limits, restated as literals', () => {
    // ⚠ Literals on purpose. Reading them from the same constants they check would assert that a
    // number equals itself; these are transcribed from `crates/mjx-sml/src/address.rs`, and a
    // divergence from the crate is meant to be a failing line here.
    expect(columnCount).toBe(16_384);
    expect(rowCount).toBe(1_048_576);
    expect(lastColumnIndex).toBe(16_383);
    expect(lastRowIndex).toBe(1_048_575);
  });

  it('spells columns as the crate does, at both ends of the grid', () => {
    expect(columnLetters(0)).toBe('A');
    expect(columnLetters(25)).toBe('Z');
    expect(columnLetters(26)).toBe('AA');
    expect(columnLetters(lastColumnIndex)).toBe('XFD');
    expect(columnIndexFromLetters('XFD')).toEqual({ ok: true, value: lastColumnIndex });
  });

  it('refuses a column past XFD and a row outside the grid, never clamping either', () => {
    // The crate's reason, quoted: "A reference outside the grid is a defect in the file, and
    // answering XFD for it would silently move somebody's data one column."
    expect(columnIndexFromLetters('XFE')).toEqual({ ok: false, problem: 'columnOutOfGrid' });
    expect(parseCellReference('A0')).toEqual({ ok: false, problem: 'rowOutOfGrid' });
    expect(parseCellReference('A1048577')).toEqual({ ok: false, problem: 'rowOutOfGrid' });
  });

  it('refuses lowercase exactly as the crate does', () => {
    // `mjx_sml::column_index_from_letters` does not fold case, so neither does this.
    expect(parseCellReference('a1')).toEqual({ ok: false, problem: 'missingColumnLetters' });
  });

  it('accepts lowercase from a PERSON, because the normalisation is the name box’s', () => {
    // The one deliberate difference between the two grammars, and it lives outside the parser.
    expect(normaliseTypedAddress(' b7 ')).toBe('B7');
    expect(parseCellReference(normaliseTypedAddress('b7')).ok).toBe(true);
    // A sheet name keeps its case: it is a name, and Excel's are case-preserving.
    expect(normaliseTypedAddress("'My Sheet'!b7")).toBe("'My Sheet'!B7");
  });

  it('parses the crate’s four range shapes and no fifth', () => {
    expect(parseCellRange('A1')).toMatchObject({ ok: true, value: { shape: 'cell' } });
    expect(parseCellRange('A1:C3')).toMatchObject({ ok: true, value: { shape: 'cells' } });
    expect(parseCellRange('A:C')).toMatchObject({ ok: true, value: { shape: 'columns' } });
    expect(parseCellRange('1:3')).toMatchObject({ ok: true, value: { shape: 'rows' } });
    expect(parseCellRange('A1:B2:C3')).toEqual({ ok: false, problem: 'tooManyRangeEnds' });
    expect(parseCellRange('A:1')).toEqual({ ok: false, problem: 'mismatchedRangeEnds' });
    expect(parseCellRange('A1:B')).toEqual({ ok: false, problem: 'mismatchedRangeEnds' });
  });

  it('preserves the order the ends were written and orders only when asked', () => {
    // The crate's own doctest, transcribed: C3:A1 comes back as written.
    const parsed = parseCellRange('C3:A1');
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(rangeText(parsed.value)).toBe('C3:A1');
    expect(normalizedBounds(parsed.value).firstColumn).toBe(0);
    expect(normalizedBounds(parsed.value).lastColumn).toBe(2);
  });

  it('widens a whole-column and a whole-row form only in normalizedBounds', () => {
    const columns = parseCellRange('A:C');
    const rows = parseCellRange('1:3');
    expect(columns.ok && normalizedBounds(columns.value).lastRow).toBe(lastRowIndex);
    expect(rows.ok && normalizedBounds(rows.value).lastColumn).toBe(lastColumnIndex);
  });

  it('preserves anchoring through a round trip', () => {
    const parsed = parseCellReference('$B$7');
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(cellAddressText(parsed.value)).toBe('$B$7');
    expect(parsed.value.columnAnchoring).toBe('absolute');
  });

  it('names one problem per AddressError variant, and every one has a message', () => {
    expect(addressProblemNames.length).toBe(11);
    for (const problem of addressProblemNames) {
      expect(addressProblemMessages[problem].length).toBeGreaterThan(0);
    }
  });

  it('counts cells the way GridBounds does', () => {
    const parsed = parseCellRange('A1:C3');
    expect(parsed.ok && cellCount(normalizedBounds(parsed.value))).toBe(9);
  });
});

// ── the tokeniser ──────────────────────────────────────────────────────────────────────────────

describe('tokenising', () => {
  it('tiles the source exactly, leaving no character unaccounted for', () => {
    // The invariant every caret-relative answer below rests on: offsets are half-open and the
    // tokens abut. A gap would put the caret in no token at all.
    const sources = [
      '=SUM(A1:A9)',
      '=IF(COUNTIF(A1:A9,">5")>0,TEXT(B2,"#,##0"),"none, really")',
      "='My Sheet'!$A$1+[1]Book!B2",
      '=Table1[Amount]*1.5e3',
      '=#REF!+TRUE',
      'a plain label',
      '="unterminated',
    ];
    for (const source of sources) {
      const tokens_ = tokeniseFormula(source);
      let at = 0;
      for (const token of tokens_) {
        expect(token.from).toBe(at);
        expect(source.slice(token.from, token.to)).toBe(token.text);
        at = token.to;
      }
      expect(at).toBe(source.length);
    }
  });

  it('never treats a comma or a parenthesis inside a string as syntax', () => {
    const tokens_ = tokeniseFormula('=TEXT(B2,"#,##0 (x)")');
    const separators = tokens_.filter((token) => token.kind === 'separator');
    expect(separators.length).toBe(1);
    expect(tokens_.filter((token) => token.kind === 'openParen').length).toBe(1);
  });

  it('reads a half-typed string rather than throwing, and says it is unterminated', () => {
    const tokens_ = tokeniseFormula('=CONCAT("abc');
    const string = tokens_.find((token) => token.kind === 'string');
    expect(string?.unterminated).toBe(true);
  });

  it('tells a function name from a defined name by the parenthesis after it', () => {
    expect(tokeniseFormula('=SUM(1)')[1]?.kind).toBe('functionName');
    expect(tokeniseFormula('=SUM+1')[1]?.kind).toBe('name');
  });

  it('tells a reference from a name that merely looks like one', () => {
    expect(tokeniseFormula('=A1')[1]?.kind).toBe('reference');
    expect(tokeniseFormula('=A1B2')[1]?.kind).toBe('name');
    expect(tokeniseFormula('=XFE1')[1]?.kind).toBe('name');
  });

  it('reads a sheet-qualified and an external reference as ONE token', () => {
    const qualified = tokeniseFormula("='My Sheet'!$A$1")[1];
    expect(qualified?.kind).toBe('reference');
    expect(qualified?.reference?.sheet).toBe('My Sheet');
    const external = tokeniseFormula('=[1]Book!B2:C3')[1];
    expect(external?.reference?.book).toBe('1');
    expect(external?.reference?.shape).toBe('cells');
  });

  it('reads an error value as a literal, and a structured reference whole', () => {
    expect(tokeniseFormula('=#REF!')[1]?.kind).toBe('errorValue');
    const structured = tokeniseFormula('=Table1[[#Data],[Amount]]')[1];
    expect(structured?.kind).toBe('structured');
    expect(structured?.text).toBe('Table1[[#Data],[Amount]]');
  });

  it('knows what is a formula and what is a label', () => {
    expect(isFormula('=1')).toBe(true);
    expect(isFormula('1 = 1')).toBe(false);
  });
});

// ── the reference-colouring contract ───────────────────────────────────────────────────────────

describe('the reference-colouring contract', () => {
  const multiple = '=SUM(A1:A9)+B2-Sheet2!C3*$A$1/Sheet2!C3';

  it('reports every reference, in source order, with where it sits and what it covers', () => {
    const highlights = referenceHighlights(multiple);
    expect(highlights.map((highlight) => highlight.text)).toEqual([
      'A1:A9',
      'B2',
      'Sheet2!C3',
      '$A$1',
      'Sheet2!C3',
    ]);
    for (const highlight of highlights) {
      expect(multiple.slice(highlight.from, highlight.to)).toBe(highlight.text);
    }
    expect(highlights[0]?.bounds).toEqual({ firstColumn: 0, lastColumn: 0, firstRow: 0, lastRow: 8 });
    expect(highlights[2]?.sheet).toBe('Sheet2');
  });

  it('gives the same range one colour however it was spelled, and a repeat the SAME slot', () => {
    const highlights = referenceHighlights(multiple);
    // A1:A9, B2, Sheet2!C3, $A$1 — four distinct ranges, four slots. `$A$1` is `A1` with anchors,
    // which is the same cell, so it takes A1:A9's… no: A1:A9 is nine cells, $A$1 is one. Four.
    expect(highlights.map((highlight) => highlight.slot)).toEqual([0, 1, 2, 3, 2]);
  });

  it('wraps to the first slot on the fifth distinct range, and says so rather than hiding it', () => {
    const many = '=A1+B1+C1+D1+E1+F1';
    expect(referenceHighlights(many).map((highlight) => highlight.slot)).toEqual([0, 1, 2, 3, 0, 1]);
  });

  it('ignores anchoring when deciding sameness, and does NOT ignore the sheet', () => {
    const anchored = referenceHighlights('=A1+$A$1');
    expect(anchored[0]?.slot).toBe(anchored[1]?.slot);
    const sheets = referenceHighlights('=Sheet1!A1+Sheet2!A1');
    expect(sheets[0]?.slot).not.toBe(sheets[1]?.slot);
  });

  it('keys identity on the ordered rectangle, so C3:A1 and A1:C3 are one range', () => {
    const first = tokeniseFormula('=C3:A1')[1]?.reference;
    const second = tokeniseFormula('=A1:C3')[1]?.reference;
    expect(first).toBeDefined();
    expect(second).toBeDefined();
    if (first === undefined || second === undefined) return;
    expect(referenceIdentity(first)).toBe(referenceIdentity(second));
  });

  it('finds no reference inside a string, which is the contract’s own version of the trap', () => {
    expect(referenceHighlights('=CONCAT("A1:A9")')).toEqual([]);
  });

  it('draws every slot in a colour that clears 4.5 : 1 in the scheme it is used in', () => {
    // ⚠ This is why a slot carries TWO tokens. A single one would be illegible in whichever scheme
    // it was not chosen for, silently — the a11y sweep runs in the light scheme only.
    expect(referenceColourSlots.length).toBe(4);
    for (const slot of referenceColourSlots) {
      const light = contrastRatio(generatedDefault(slot.light), tokens.theme.light.surface) ?? 0;
      const dark = contrastRatio(generatedDefault(slot.dark), tokens.theme.dark.surface) ?? 0;
      expect(light, `${slot.name} on the light surface`).toBeGreaterThanOrEqual(bodyTextMinimum);
      expect(dark, `${slot.name} on the dark surface`).toBeGreaterThanOrEqual(bodyTextMinimum);
    }
  });

  it('draws four DIFFERENT colours in each scheme, so the ring distinguishes anything', () => {
    // A ring whose four slots resolved to one colour would pass the contrast sweep perfectly and
    // colour nothing — the same shape of vacuity as a cache that evicts everything.
    const light = new Set(referenceColourSlots.map((slot) => generatedDefault(slot.light)));
    const dark = new Set(referenceColourSlots.map((slot) => generatedDefault(slot.dark)));
    expect(light.size).toBe(referenceColourSlots.length);
    expect(dark.size).toBe(referenceColourSlots.length);
  });
});

// ── bracket matching ───────────────────────────────────────────────────────────────────────────

describe('bracket matching', () => {
  it('pairs nested parentheses and reports the ones with no partner', () => {
    const report = bracketReport('=IF(SUM(A1:A9)>0,1,2');
    expect(report.pairs.length).toBe(1);
    expect(report.unmatched).toEqual([3]);
    expect(bracketReport('=1)').unmatched).toEqual([2]);
  });

  it('does not see a parenthesis inside a string — and the naive scan does', () => {
    // `=CONCAT("(")`: the real `(` is at 7 and its `)` at 11; the one at 9 is inside the string.
    const source = '=CONCAT("(")';
    expect(bracketReport(source).pairs).toEqual([{ open: 7, close: 11 }]);
    expect(bracketReport(source).unmatched).toEqual([]);
    // The positive control, failing where it is supposed to: it pairs the STRING's parenthesis
    // with the call's, and then reports the call's own `(` as unclosed.
    expect(naiveBracketReport(source).pairs).toEqual([{ open: 9, close: 11 }]);
    expect(naiveBracketReport(source).unmatched).toEqual([7]);
  });

  it('lights a pair only when the caret is ON it, never for the whole span', () => {
    const source = '=SUM(A1)';
    expect(bracketAt(source, 5)?.open).toBe(4);
    expect(bracketAt(source, 7)?.close).toBe(7);
    expect(bracketAt(source, 8)?.close).toBe(7);
    expect(bracketAt(source, 6)).toBeUndefined();
  });
});

// ── THE GATE: the caret-position table ─────────────────────────────────────────────────────────

/**
 * One formula, with nested calls **and** two quoted strings, one of which contains a comma.
 *
 * `IF` takes three arguments; `COUNTIF` and `TEXT` are nested inside two of them; `"#,##0"` puts a
 * comma inside `TEXT`'s second argument, and `"none, really"` puts one inside `IF`'s third. Those
 * two commas are the whole test: a comma count sees five arguments where there are three.
 */
const gateFormula = '=IF(COUNTIF(A1:A9,">5")>0,TEXT(B2,"#,##0"),"none, really")';

/** An offset named by what it is after, so a row cannot silently point at the wrong character. */
function after(needle: string, extra = 0): number {
  const at = gateFormula.indexOf(needle);
  expect(at, `'${needle}' is not in the gate formula`).toBeGreaterThanOrEqual(0);
  return at + needle.length + extra;
}

interface CaretRow {
  readonly where: string;
  readonly caret: number;
  readonly fn: string | undefined;
  readonly argument: number | undefined;
  readonly depth?: number;
}

const caretTable: readonly CaretRow[] = [
  { where: 'before the =', caret: 0, fn: undefined, argument: undefined },
  { where: 'just after IF(', caret: after('IF('), fn: 'IF', argument: 0, depth: 0 },
  { where: 'inside COUNTIF’s first argument', caret: after('COUNTIF(A1'), fn: 'COUNTIF', argument: 0, depth: 1 },
  { where: 'after COUNTIF’s separator', caret: after('A1:A9,'), fn: 'COUNTIF', argument: 1, depth: 1 },
  { where: 'inside COUNTIF’s quoted criteria', caret: after('">'), fn: 'COUNTIF', argument: 1, depth: 1 },
  { where: 'after COUNTIF closed, back in IF’s first argument', caret: after('">5")'), fn: 'IF', argument: 0, depth: 0 },
  { where: 'after IF’s first separator', caret: after('>0,'), fn: 'IF', argument: 1, depth: 0 },
  { where: 'inside TEXT’s first argument', caret: after('TEXT(B2'), fn: 'TEXT', argument: 0, depth: 1 },
  { where: 'after TEXT’s separator', caret: after('TEXT(B2,'), fn: 'TEXT', argument: 1, depth: 1 },
  // ⚠ THE ROW THE NAIVE IMPLEMENTATION FAILS. The comma is inside "#,##0".
  { where: 'after the comma INSIDE TEXT’s format string', caret: after('"#,'), fn: 'TEXT', argument: 1, depth: 1 },
  { where: 'after TEXT closed, still in IF’s second argument', caret: after('"#,##0")'), fn: 'IF', argument: 1, depth: 0 },
  { where: 'after IF’s second separator', caret: after('"#,##0"),'), fn: 'IF', argument: 2, depth: 0 },
  // ⚠ AND THE SECOND ONE. The comma is inside "none, really".
  { where: 'after the comma INSIDE IF’s third argument’s string', caret: after('"none,'), fn: 'IF', argument: 2, depth: 0 },
  { where: 'after the final )', caret: gateFormula.length, fn: undefined, argument: undefined },
];

describe('the caret-position table', () => {
  it('has more than a dozen rows over a formula with nested calls and a quoted comma', () => {
    expect(caretTable.length).toBeGreaterThanOrEqual(12);
    expect(gateFormula).toContain('COUNTIF(');
    expect(gateFormula).toContain('"#,##0"');
    // A table over a formula with no quoted comma would be a table any implementation passes.
    expect(gateFormula.match(/"[^"]*,[^"]*"/g)?.length).toBe(2);
  });

  for (const row of caretTable) {
    it(`says ${row.fn ?? 'nothing'} argument ${String(row.argument ?? -1)} ${row.where}`, () => {
      const answer = activeArgument(gateFormula, row.caret);
      expect(answer?.functionName).toBe(row.fn);
      expect(answer?.argumentIndex).toBe(row.argument);
      if (row.depth !== undefined) expect(answer?.depth).toBe(row.depth);
      if (answer !== undefined) {
        // The argument's own extent, so a tooltip could underline it.
        expect(answer.from).toBeLessThanOrEqual(row.caret);
        expect(answer.to).toBeGreaterThanOrEqual(row.caret);
      }
    });
  }

  it('DISAGREES with a naive comma count, at the two offsets a quoted comma moves', () => {
    // ⚠ The half of the gate that cannot be satisfied by a correct-looking implementation. If this
    // ever passes, the naive control has been fixed and the table has stopped proving anything.
    const quoted = caretTable.filter((row) => row.where.includes('INSIDE'));
    expect(quoted.length).toBe(2);
    for (const row of quoted) {
      const naive = naiveActiveArgument(gateFormula, row.caret);
      expect(naive?.argumentIndex, `naive at ${row.where}`).not.toBe(row.argument);
    }
  });

  it('agrees with the naive count everywhere no string is in the way', () => {
    // The other half: the two implementations differ ONLY because of the quoting, so the
    // disagreement above is attributable rather than incidental.
    for (const row of caretTable.filter((candidate) => !candidate.where.includes('INSIDE'))) {
      const naive = naiveActiveArgument(gateFormula, row.caret);
      if (row.fn === undefined) continue;
      expect(naive?.functionName, row.where).toBe(row.fn);
      expect(naive?.argumentIndex, row.where).toBe(row.argument);
    }
  });

  it('treats a bare grouping parenthesis as part of the argument, not as a call', () => {
    // `=SUM((A1,B1))` — the inner comma is the union operator. Excel still shows SUM, argument one.
    const source = '=SUM((A1,B1))';
    const answer = activeArgument(source, source.indexOf('B1'));
    expect(answer?.functionName).toBe('SUM');
    expect(answer?.argumentIndex).toBe(0);
  });

  it('never throws on a half-typed formula, at any caret at all', () => {
    const source = '=IF(COUNTIF(A1:A9,">5';
    for (let caret = 0; caret <= source.length; caret += 1) {
      expect(() => activeArgument(source, caret)).not.toThrow();
    }
    expect(activeArgument(source, source.length)?.functionName).toBe('COUNTIF');
  });
});

// ── the function catalogue and the tooltip’s emphasis ──────────────────────────────────────────

describe('the function catalogue', () => {
  it('writes a signature the way Excel does', () => {
    const sum = signatureFor('sum');
    expect(sum).toBeDefined();
    if (sum === undefined) return;
    expect(signatureText(sum)).toBe('SUM(number1, [number2], …)');
  });

  it('emphasises the repeating parameter for every argument past the list’s end', () => {
    // The defect this exists to prevent: a tooltip that indexed straight into the parameter list
    // would emphasise NOTHING from the third argument onward, which is when it is most needed.
    const sum = signatureFor('SUM');
    expect(sum).toBeDefined();
    if (sum === undefined) return;
    expect(emphasisedParameter(sum, 0)).toBe(0);
    expect(emphasisedParameter(sum, 1)).toBe(1);
    expect(emphasisedParameter(sum, 7)).toBe(1);
  });

  it('says -1 rather than lying, when a call has more arguments than the signature allows', () => {
    const round = signatureFor('ROUND');
    expect(round).toBeDefined();
    if (round === undefined) return;
    expect(emphasisedParameter(round, 2)).toBe(-1);
  });

  it('filters by prefix, not by containment', () => {
    expect(completionsFor('SU').map((entry) => entry.name)).toEqual(['SUM', 'SUMIF', 'SUMIFS']);
    expect(completionsFor('su').map((entry) => entry.name)).toEqual(['SUM', 'SUMIF', 'SUMIFS']);
    expect(completionsFor('EX')).toEqual([]);
    expect(completionsFor('')).toEqual([]);
  });

  it('offers a completion only where one would not be an interruption', () => {
    expect(completionPrefix('=SU', 3)?.text).toBe('SU');
    expect(completionPrefix('=SUM(', 5)).toBeUndefined();
    expect(completionPrefix('=CONCAT("SU', 11)).toBeUndefined();
    expect(completionPrefix('=A1', 3)).toBeUndefined();
    expect(completionPrefix('plain text', 5)).toBeUndefined();
  });

  it('completes to the name AND its opening parenthesis, with the caret inside', () => {
    const prefix = completionPrefix('=SU+1', 3);
    expect(prefix).toBeDefined();
    const sum = signatureFor('SUM');
    if (prefix === undefined || sum === undefined) return;
    const applied = applyCompletion('=SU+1', prefix, sum);
    expect(applied.text).toBe('=SUM(+1');
    expect(applied.caret).toBe(5);
  });

  it('names every function once and sorts nothing by accident', () => {
    const names = formulaFunctions.map((entry) => entry.name);
    expect(new Set(names).size).toBe(names.length);
    expect([...names].sort((first, second) => first.localeCompare(second))).toEqual(names);
  });

  it('has no way to evaluate anything, which is the scope this child was given', () => {
    // Stated as an assertion rather than as a sentence: a signature is a LABEL. If a `compute` or
    // an `evaluate` ever appears on one, the component has begun implying a calculation engine.
    for (const entry of formulaFunctions) {
      expect(Object.keys(entry).sort()).toEqual(['category', 'name', 'parameters', 'summary']);
    }
  });
});

// ── the four modes ─────────────────────────────────────────────────────────────────────────────

describe('the four modes', () => {
  it('has exactly four, each with a label and an announcement of its own', () => {
    expect(formulaModeNames).toEqual(['ready', 'enter', 'edit', 'point']);
    const announcements = new Set(formulaModeNames.map((mode) => formulaModes[mode].announcement));
    expect(announcements.size).toBe(4);
    for (const mode of formulaModeNames) {
      expect(formulaModes[mode].label.length).toBeGreaterThan(0);
      expect(formulaModes[mode].announcement.length).toBeGreaterThan(0);
    }
  });

  it('remembers what Point came from, which is the whole reason it is a state', () => {
    const typing = applyModeTrigger(readyModeState, 'beginEntry');
    expect(typing.mode).toBe('enter');
    const pointingFromEntry = applyModeTrigger(typing, 'enterPoint');
    expect(pointingFromEntry.mode).toBe('point');
    expect(applyModeTrigger(pointingFromEntry, 'leavePoint').mode).toBe('enter');

    const editing = applyModeTrigger(readyModeState, 'beginEdit');
    const pointingFromEdit = applyModeTrigger(editing, 'enterPoint');
    expect(applyModeTrigger(pointingFromEdit, 'leavePoint').mode).toBe('edit');
  });

  it('is total: every trigger has an answer in every mode', () => {
    const triggers = ['beginEntry', 'beginEdit', 'commit', 'cancel', 'enterPoint', 'leavePoint'] as const;
    const states: FormulaModeState[] = [
      readyModeState,
      { mode: 'enter', resume: 'enter' },
      { mode: 'edit', resume: 'edit' },
      { mode: 'point', resume: 'edit' },
    ];
    for (const state of states) {
      for (const trigger of triggers) {
        const next = applyModeTrigger(state, trigger);
        expect(formulaModeNames as readonly FormulaMode[]).toContain(next.mode);
      }
    }
  });

  it('returns to Ready from anywhere on a commit or a cancel', () => {
    for (const mode of formulaModeNames) {
      const state: FormulaModeState = { mode, resume: 'edit' };
      expect(applyModeTrigger(state, 'commit')).toEqual(readyModeState);
      expect(applyModeTrigger(state, 'cancel')).toEqual(readyModeState);
    }
  });

  it('knows where an arrow key would start pointing, and where it would move the caret', () => {
    expect(pointReady('=', 1)).toBe(true);
    expect(pointReady('=SUM(', 5)).toBe(true);
    expect(pointReady('=SUM(A1,', 8)).toBe(true);
    expect(pointReady('=A1+', 4)).toBe(true);
    expect(pointReady('=A1', 3)).toBe(false);
    expect(pointReady('=SUM(A1)', 8)).toBe(false);
    expect(pointReady('=CONCAT("a', 10)).toBe(false);
    expect(pointReady('plain', 5)).toBe(false);
  });
});

// ── multi-line expansion ───────────────────────────────────────────────────────────────────────

describe('the expansion handle', () => {
  it('clamps to the declared bounds', () => {
    expect(clampFormulaRows(0)).toBe(formulaRowBounds.min);
    expect(clampFormulaRows(99)).toBe(formulaRowBounds.max);
    expect(clampFormulaRows(Number.NaN)).toBe(formulaRowBounds.min);
  });

  it('moves one row per arrow, both ends per Home and End, and toggles on Enter', () => {
    expect(rowsAfterKey('ArrowDown', 1)).toBe(2);
    expect(rowsAfterKey('ArrowUp', 2)).toBe(1);
    expect(rowsAfterKey('ArrowUp', 1)).toBe(formulaRowBounds.min);
    expect(rowsAfterKey('Home', 5)).toBe(formulaRowBounds.min);
    expect(rowsAfterKey('End', 1)).toBe(formulaRowBounds.max);
    expect(rowsAfterKey('Enter', 1)).toBe(expandedFormulaRows);
    expect(rowsAfterKey('Enter', expandedFormulaRows)).toBe(formulaRowBounds.min);
  });

  it('claims no key that is not its own, so Tab still leaves', () => {
    expect(rowsAfterKey('Tab', 1)).toBeUndefined();
    expect(rowsAfterKey('a', 1)).toBeUndefined();
  });

  it('turns a drag into whole rows, and refuses to divide by a height of nothing', () => {
    expect(rowsAfterDrag(1, 40, 20)).toBe(3);
    expect(rowsAfterDrag(3, -40, 20)).toBe(1);
    expect(rowsAfterDrag(2, 100, 0)).toBe(2);
  });
});

// ── the name box ───────────────────────────────────────────────────────────────────────────────

describe('the name box’s navigation', () => {
  const names: readonly DefinedNameEntry[] = [
    { name: 'Revenue', definition: 'Summary!$B$1', kind: 'name' },
    { name: 'Q1', definition: "'Q1 Data'!$A$1:$D$40", kind: 'name' },
    { name: 'SalesTable', definition: 'Sheet1!$A$1:$F$120', kind: 'table' },
  ];

  it('reads an address as an address', () => {
    const request = navigationFor('b7', names);
    expect(request.kind).toBe('address');
    if (request.kind !== 'address') return;
    expect(request.text).toBe('B7');
    expect(request.bounds).toEqual({ firstColumn: 1, lastColumn: 1, firstRow: 6, lastRow: 6 });
  });

  it('reads a range, a whole column and a sheet-qualified address', () => {
    expect(navigationFor('a1:c9', names).kind).toBe('address');
    expect(navigationFor('A:C', names).kind).toBe('address');
    const qualified = navigationFor("'My Sheet'!A1", names);
    expect(qualified.kind).toBe('address');
    if (qualified.kind !== 'address') return;
    expect(qualified.sheet).toBe('My Sheet');
  });

  it('reads a defined name and a table as a name', () => {
    const request = navigationFor('revenue', names);
    expect(request.kind).toBe('name');
    if (request.kind !== 'name') return;
    expect(request.entry.definition).toBe('Summary!$B$1');
    expect(navigationFor('SalesTable', names).kind).toBe('name');
  });

  it('lets a defined name WIN over an address it happens to be spelled like', () => {
    // ⚠ The standing rule: the user's document beats our grammar. A workbook whose author named a
    // range `Q1` means that range, and a name box that sent them to cell Q1 instead would be this
    // library imposing its own reading of somebody else's file.
    const request = navigationFor('Q1', names);
    expect(request.kind).toBe('name');
  });

  it('reports a problem by name rather than navigating somewhere arbitrary', () => {
    expect(navigationFor('XFE1', names)).toMatchObject({ kind: 'problem', problem: 'columnOutOfGrid' });
    expect(navigationFor('A0', names)).toMatchObject({ kind: 'problem', problem: 'rowOutOfGrid' });
    expect(navigationFor('  ', names)).toMatchObject({ kind: 'problem', problem: 'empty' });
    expect(navigationFor('!A1', names)).toMatchObject({ kind: 'problem', problem: 'emptySheetName' });
  });

  it('announces a multi-cell selection with its size, which its address does not carry', () => {
    const range = parseCellRange('A1:C9');
    expect(range.ok).toBe(true);
    if (!range.ok) return;
    expect(selectionAnnouncement(range.value)).toBe('A1:C9, 9 by 3');
    const single = parseCellRange('B7');
    expect(single.ok && selectionAnnouncement(single.value)).toBe('B7');
  });
});

describe('the event vocabulary', () => {
  it('names every event once, under one prefix', () => {
    const names = Object.values(formulaEvents);
    expect(new Set(names).size).toBe(names.length);
    for (const name of names) expect(name.startsWith('mjx-')).toBe(true);
  });
});
