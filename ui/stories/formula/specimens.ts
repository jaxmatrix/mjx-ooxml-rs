/**
 * MJXOFF-192's specimens — and the **caret readouts**, which are the point.
 *
 * The ticket's trap, in its own words: *"every hard behaviour here is caret-relative, and a story
 * that shows a static formula proves none of it."* So no story here shows a formula sitting still.
 * Every one of them carries a live readout — the mode, the caret offset, the active argument, the
 * bracket pair and the reference-colour contract — written from the component's own events, so an
 * auditor moving the caret with the arrow keys can watch all five change, and so a story that had
 * stopped reporting would be visibly stuck rather than plausibly static.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  formulaEvents,
  formulaModes,
  type DefinedNameEntry,
  type ReferenceHighlight,
} from '../../src/formula/formula-model.ts';
import type { StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:72ch"
    >
      ${text}
    </p>
  `;
}

/** A caption under a specimen, in the measured-figure style the catalogue uses. */
export function caption(lines: readonly string[]): TemplateResult {
  return html`
    <ul
      class=${typeRoleClass('dense')}
      style="margin:0;padding:var(--mjx-density-gutter) calc(var(--mjx-density-gutter) * 3);
             color:var(--theme-text-secondary)"
    >
      ${lines.map((line) => html`<li>${line}</li>`)}
    </ul>
  `;
}

/** A stage with room under the bar for a popup and a readout. */
export function stage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="min-block-size:30rem;display:flex;flex-direction:column;
             gap:var(--mjx-density-gutter);padding:calc(var(--mjx-density-gutter) * 2);
             background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/** The live readout every formula story carries. */
export function readout(id: string): TemplateResult {
  return html`
    <output
      id=${id}
      class=${typeRoleClass('dense')}
      style="display:block;white-space:pre-wrap;font-family:var(--font-mono);
             color:var(--theme-text-primary);background:var(--theme-surface-raised);
             border:1px solid var(--theme-border);border-radius:var(--radius-chip);
             padding:var(--mjx-density-gutter)"
      >move the caret to see this change</output
    >
  `;
}

/** One line per reference, so the *contract* is visible rather than only the colouring. */
function describeReferences(references: readonly ReferenceHighlight[]): string {
  if (references.length === 0) return 'no references';
  return references
    .map(
      (reference) =>
        `slot ${String(reference.slot)}  ${reference.text.padEnd(14)} ` +
        `rows ${String(reference.bounds.firstRow + 1)}–${String(reference.bounds.lastRow + 1)} ` +
        `cols ${String(reference.bounds.firstColumn + 1)}–${String(reference.bounds.lastColumn + 1)}`,
    )
    .join('\n');
}

/**
 * Wire a bar's events into a readout.
 *
 * ⚠ Everything here is read from the **events**, never recomputed from the text. A story that
 * re-derived the answer would be a second implementation, and the one thing it could never show is
 * the component disagreeing with itself.
 */
export function watch(barId: string, readoutId: string): void {
  queueMicrotask(() => {
    const bar = document.getElementById(barId);
    const output = document.getElementById(readoutId);
    if (!(bar instanceof HTMLElement) || !(output instanceof HTMLElement)) return;
    customElements.upgrade(bar);
    const lines = new Map<string, string>([
      ['mode', `mode      ${formulaModes.ready.label}`],
      ['caret', 'caret     0'],
      ['argument', 'argument  —'],
      ['brackets', 'brackets  —'],
      ['references', 'no references'],
    ]);
    const paint = (): void => {
      output.textContent = [...lines.values()].join('\n');
    };
    bar.addEventListener(formulaEvents.mode, (event) => {
      const detail = (event as CustomEvent<{ mode: string; announcement: string }>).detail;
      lines.set('mode', `mode      ${detail.mode}  ·  “${detail.announcement}”`);
      paint();
    });
    bar.addEventListener(formulaEvents.caret, (event) => {
      const detail = (
        event as CustomEvent<{
          caret: number;
          argument?: { functionName: string; argumentIndex: number; depth: number };
          brackets?: { open: number; close: number };
        }>
      ).detail;
      lines.set('caret', `caret     ${String(detail.caret)}`);
      lines.set(
        'argument',
        detail.argument === undefined
          ? 'argument  —'
          : `argument  ${detail.argument.functionName} #${String(detail.argument.argumentIndex + 1)} at depth ${String(detail.argument.depth)}`,
      );
      lines.set(
        'brackets',
        detail.brackets === undefined
          ? 'brackets  —'
          : `brackets  ${String(detail.brackets.open)} ↔ ${String(detail.brackets.close)}`,
      );
      paint();
    });
    bar.addEventListener(formulaEvents.references, (event) => {
      const detail = (event as CustomEvent<{ references: readonly ReferenceHighlight[] }>).detail;
      lines.set('references', describeReferences(detail.references));
      paint();
    });
    paint();
  });
}

/** Fill a name box once it has upgraded. */
export function fillNames(id: string, names: readonly DefinedNameEntry[]): void {
  queueMicrotask(() => {
    const box = document.getElementById(id);
    if (!(box instanceof HTMLElement)) return;
    customElements.upgrade(box);
    (box as HTMLElement & { names: readonly DefinedNameEntry[] }).names = names;
  });
}

/** A workbook's defined names and tables, of both kinds and both scopes. */
export const workbookNames: readonly DefinedNameEntry[] = [
  { name: 'Revenue', definition: 'Summary!$B$1', kind: 'name' },
  { name: 'CostOfSales', definition: 'Summary!$B$2', kind: 'name' },
  { name: 'Headcount', definition: "'HR Data'!$C$1:$C$240", kind: 'name', scope: 'HR Data' },
  { name: 'Q1', definition: "'Q1 Data'!$A$1:$D$40", kind: 'name' },
  { name: 'PrintArea', definition: 'Summary!$A$1:$H$60', kind: 'name', scope: 'Summary' },
  { name: 'SalesTable', definition: 'Sheet1!$A$1:$F$120', kind: 'table' },
  { name: 'BudgetTable', definition: "'Budget 2026'!$A$1:$K$400", kind: 'table' },
];

/**
 * **The formula the caret table is written over**, so the story and the gate look at one thing.
 *
 * Nested calls, a quoted comparison, a format string with a comma in it and a message with another.
 */
export const gateFormula = '=IF(COUNTIF(A1:A9,">5")>0,TEXT(B2,"#,##0"),"none, really")';

/** A formula with four distinct ranges and one repeat, which is the contract's own specimen. */
export const multiReferenceFormula = '=SUM(A1:A9)+B2-Sheet2!C3*$A$1/Sheet2!C3';

/** A long formula, for the multi-line story. */
export const longFormula =
  '=IFERROR(VLOOKUP($A2,SalesTable,MATCH("Amount",Sheet1!$A$1:$F$1,0),FALSE),' +
  'TEXTJOIN(", ",TRUE,"not found",TEXT(TODAY,"yyyy-mm-dd")))';

/** The states matrix both components declare. */
export const formulaStates: readonly StoryState[] = [
  { name: 'ready', description: 'Nothing is being edited. Confirm and cancel are unavailable.' },
  { name: 'enter', description: 'Typing replaces the cell’s contents.' },
  { name: 'edit', description: 'Editing what the cell held. The arrow keys move the caret.' },
  { name: 'point', description: 'The caret is where an arrow key would name a range.' },
  { name: 'autocomplete', description: 'A function name is being typed and the list is open.' },
  { name: 'argument tooltip', description: 'The caret is inside a call; its argument is emphasised.' },
  { name: 'expanded', description: 'The editor has been dragged or keyed taller than one row.' },
  { name: 'invalid', description: 'The name box refused what was typed and said why.' },
];

/** The keyboard both components declare. */
export const formulaKeyboard = [
  { keys: 'Tab', does: 'Moves to the next control. One tab stop for the editor, one for the handle.' },
  { keys: 'Enter', does: 'Commits the formula — or accepts the completion, when the list is open.' },
  { keys: 'Escape', does: 'Dismisses the completion list; a second press cancels the edit.' },
  { keys: '↓ / ↑', does: 'Moves through the completion list. On the handle, changes the editor’s height.' },
  { keys: 'Home / End', does: 'On the handle: one row, or the tallest the bar allows.' },
  { keys: 'Space / Enter', does: 'On the handle: toggles between one row and three.' },
];

/** The screen-reader note both components declare. */
export const formulaScreenReader =
  'Every mode change is announced as a sentence saying what the arrow keys will do — a mode told ' +
  'only in colour is a mode a screen-reader user cannot read. The completion list announces how ' +
  'many functions matched and names the first, because aria-expanded is not legal on a textarea; ' +
  'the active completion is carried on aria-activedescendant, which is. The argument tooltip is ' +
  'the editor’s aria-describedby. The height handle is a separator with a value in rows.';

/** The token dependencies every formula story declares. */
export const formulaTokenDependencies: readonly TokenPath[] = [
  'theme.light.background',
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.light.accentSurface',
  'theme.light.accentBorder',
  'theme.light.accentPressed',
  'theme.light.secondaryAccent',
  'theme.dark.surface',
  'theme.dark.surfaceRaised',
  'theme.dark.textPrimary',
  'theme.dark.textSecondary',
  // The reference ring, both schemes. These are the four colours the contract hands to loop 2.
  'color.greenDeep',
  'color.greenLifted',
  'color.clay',
  'color.honey',
  'color.ink',
  'radius.chip',
  'radius.control',
  'shadow.lift',
  'spacing',
  'duration.transition',
  'ease.outSoft',
  'text.sm',
  'leading.snug',
  'font.mono',
  'font.sans',
  'font.serif',
  'fontWeight.semibold',
  'fontWeight.bold',
];
