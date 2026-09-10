import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { formulaRowBounds, referenceColourSlots } from '../../src/formula/formula-model.ts';
import type { MjxFormulaBar } from '../../src/formula/index.ts';
import {
  caption,
  fillNames,
  formulaKeyboard,
  formulaScreenReader,
  formulaStates,
  formulaTokenDependencies,
  gateFormula,
  longFormula,
  multiReferenceFormula,
  note,
  readout,
  stage,
  watch,
  workbookNames,
} from './specimens.ts';

/**
 * `<mjx-formula-bar>` — Excel's formula bar.
 *
 * **Open *The Argument Tooltip Tracks The Caret* first, and use the arrow keys.** Put the caret
 * inside `TEXT(B2,"#,##0")` and walk it left through the format string: the tooltip keeps saying
 * *argument 2*, because the comma in `"#,##0"` is inside a string and is not a separator. That one
 * behaviour is the whole child, and the caret readout under every story is there so it can be
 * watched rather than assumed. `tests/formula.test.ts` asserts it at fourteen offsets and asserts
 * that a comma count gets two of them wrong.
 *
 * `GUESS:` **Point mode is shown whenever the caret sits where an arrow key WOULD name a range.**
 * Excel enters Point when an arrow key or a click actually names one. This bar has no grid to point
 * at, so it reports the thing a person needs to know — what the next arrow key will do — which is
 * exactly the condition Excel uses to decide.
 *
 * `GUESS:` **the reference ring is four colours and not Excel's seven.** Four is what this palette
 * can spell at 4.5 : 1 *in both schemes*, measured in `tests/formula.test.ts`; a fifth would have
 * been a colour nobody had checked. A fifth distinct range reuses the first slot, and the contract
 * says so rather than hiding it.
 *
 * **No formula is ever evaluated.** There is no calculation engine in this loop, and nothing in
 * this component has anywhere to put a result.
 */

const conventions = storyConventions({
  statesMatrix: formulaStates,
  tokenDependencies: formulaTokenDependencies,
  keyboard: formulaKeyboard,
  screenReader: formulaScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Excel Chrome/Formula Bar',
  parameters: {
    docs: {
      description: {
        component:
          'The formula bar: a single-line editor that expands to multi-line, coloured range ' +
          'references emitted as a contract, bracket matching, function autocomplete, the ' +
          'argument tooltip, and Excel’s four modes — ready, enter, edit and point.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Put a formula into a bar once it has upgraded, and start watching it. */
function fill(id: string, value: string, readoutId: string, rows = formulaRowBounds.min): void {
  queueMicrotask(() => {
    const bar = document.getElementById(id);
    if (!(bar instanceof HTMLElement)) return;
    customElements.upgrade(bar);
    const typed = bar as MjxFormulaBar;
    typed.value = value;
    typed.rows = rows;
  });
  watch(id, readoutId);
}

/** A bar with its name box in the slot it was designed for. */
function bar(id: string) {
  return html`
    <mjx-formula-bar id=${id} label="Formula">
      <mjx-name-box slot="name-box" label="Name box" address="B7" id=${`${id}-name`}></mjx-name-box>
    </mjx-formula-bar>
  `;
}

export const TheArgumentTooltipTracksTheCaret: Story = {
  name: 'The Argument Tooltip Tracks The Caret',
  render: () => {
    fill('tooltip-bar', gateFormula, 'tooltip-readout');
    fillNames('tooltip-bar-name', workbookNames);
    return stage(
      note(
        'Click into the formula and walk the caret with ← and →. The tooltip names the innermost ' +
          'call and emphasises the argument the caret is in. Two commas in this formula are inside ' +
          'quoted strings — one in "#,##0" and one in "none, really" — and neither of them ' +
          'separates an argument. A comma count says otherwise, and the readout below is how you ' +
          'can tell which answer you are looking at.',
      ),
      html`${bar('tooltip-bar')}`,
      readout('tooltip-readout'),
      caption([
        'The tooltip is the editor’s aria-describedby, so a screen reader reads it as the field’s description.',
        'Nesting is a stack: inside COUNTIF the depth is 1, back in IF it is 0.',
        'A bare grouping parenthesis is not a call — =SUM((A1,B1)) is still SUM argument 1.',
      ]),
    );
  },
};

export const ColouredReferencesAndTheContract: Story = {
  name: 'Coloured References, And The Contract Under Them',
  render: () => {
    fill('colour-bar', multiReferenceFormula, 'colour-readout');
    fillNames('colour-bar-name', workbookNames);
    return stage(
      note(
        'Four distinct ranges and one repeat. The colour is drawn in the bar and the same ' +
          'assignment is emitted as an event, with each reference’s offsets and its ordered ' +
          'bounds — which is what lets the in-canvas grid draw a matching box in loop 2 without ' +
          'tokenising the formula a second time. Sheet2!C3 appears twice and takes the same slot ' +
          'both times; A1 and $A$1 would too, because anchoring changes what a copy does and not ' +
          'which cells are named.',
      ),
      html`${bar('colour-bar')}`,
      readout('colour-readout'),
      caption([
        `The ring is ${String(referenceColourSlots.length)} slots: ${referenceColourSlots
          .map((slot) => slot.name)
          .join(', ')}. A fifth distinct range wraps to the first.`,
        'Each slot is two tokens — one per scheme — because a colour legible on the light surface is usually not on the dark one.',
        'Switch the theme in the toolbar: the same slot, a different token, still 4.5 : 1.',
      ]),
    );
  },
};

export const TheFunctionAutocomplete: Story = {
  name: 'The Function Autocomplete',
  render: () => {
    fill('complete-bar', '=SU', 'complete-readout');
    fillNames('complete-bar-name', workbookNames);
    return stage(
      note(
        'Click at the end of the formula and keep typing a function name. The list filters by ' +
          'prefix — a person typing SU means a function that starts with SU — and each row carries ' +
          'the signature’s parameter names and what the function is for. ↓ and ↑ move, Tab or ' +
          'Enter completes to the name and its opening parenthesis, and Escape dismisses the list ' +
          'without cancelling the edit. Press Escape a second time to cancel.',
      ),
      html`${bar('complete-bar')}`,
      readout('complete-readout'),
      caption([
        'The list is U07’s ListSurface, unchanged — so it is one listbox implementation, one aria-activedescendant contract and one virtualiser.',
        'aria-expanded is NOT set: it is illegal on a textarea, and axe says so. The live region announces the count instead.',
        'The fx button opens the same list with the whole catalogue in it.',
      ]),
    );
  },
};

export const BracketMatchingAsTheCaretMoves: Story = {
  name: 'Bracket Matching As The Caret Moves',
  render: () => {
    fill('bracket-bar', '=IF(SUM(A1:A9)>ROUND(AVERAGE(B1:B9),2),"over","under")', 'bracket-readout');
    fillNames('bracket-bar-name', workbookNames);
    return stage(
      note(
        'Put the caret immediately after any parenthesis. Its partner is lit, and only while the ' +
          'caret is on one of the two — a bar that emboldened a pair for as long as the caret was ' +
          'anywhere inside it would have a permanently emboldened outer pair. Delete a closing ' +
          'parenthesis: the one left over is underlined instead.',
      ),
      html`${bar('bracket-bar')}`,
      readout('bracket-readout'),
      caption([
        'A matched pair is drawn as a fill and an unmatched one as a wavy underline, because every text colour here has to clear 4.5 : 1 and the four that do are already spent on the reference ring.',
        'Parentheses inside a string are not brackets at all: try =CONCAT("(").',
      ]),
    );
  },
};

export const TheFourModes: Story = {
  name: 'The Four Modes, Announced',
  render: () => {
    fill('mode-bar', '', 'mode-readout');
    fillNames('mode-bar-name', workbookNames);
    /*
     * ⚠ **Enter mode has no in-component trigger, and this button is the honest way to reach it.**
     * Excel enters it when a person starts typing *into a cell*, which is a grid this loop does not
     * have — so the bar exposes `beginEntry()` and a shell calls it. Without the button, Enter is
     * the one mode of the four an auditor could not reach by hand, and *"all four are
     * distinguishable"* would have been a claim about a state nobody could see.
     */
    const begin = (kind: 'entry' | 'edit') => (): void => {
      const barElement = document.getElementById('mode-bar');
      if (!(barElement instanceof HTMLElement)) return;
      const typed = barElement as MjxFormulaBar;
      if (kind === 'entry') typed.beginEntry();
      else typed.beginEdit();
      typed.focus();
    };
    return stage(
      note(
        'Ready until something starts an edit. The two buttons below are what a grid would do: ' +
          'typing into an empty cell is Enter mode, and F2 or a double-click on a cell with ' +
          'contents is Edit. Then type = and the mode becomes Point — the caret is where an arrow ' +
          'key would name a range. Type A1 and it is Edit or Enter again, whichever it came from. ' +
          'Enter commits and Escape cancels, and both return to Ready. Every one of those ' +
          'transitions is announced as a sentence, which the readout quotes.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-step)">
          <button id="mode-begin-entry" type="button" class="mjx-hit-target" @click=${begin('entry')}>
            Type into an empty cell (Enter mode)
          </button>
          <button id="mode-begin-edit" type="button" class="mjx-hit-target" @click=${begin('edit')}>
            F2 on a cell with contents (Edit mode)
          </button>
        </div>
      `,
      html`${bar('mode-bar')}`,
      readout('mode-readout'),
      caption([
        'Four fills and four shapes: a ring, a dot, a bar and a diamond. A state told only in colour is a state some readers cannot read.',
        'Point remembers what it came from, so leaving it returns to Enter or to Edit and never guesses.',
        'Focusing the editor on its own is Edit: the bar cannot know a cell was empty, and guessing Enter would tell a person their content had been replaced.',
      ]),
    );
  },
};

export const ExpandingToMultipleLines: Story = {
  name: 'Expanding To Multiple Lines',
  render: () => {
    fill('rows-bar', longFormula, 'rows-readout');
    fillNames('rows-bar-name', workbookNames);
    return stage(
      note(
        'Drag the grip at the end of the bar, or Tab to it and use ↓ and ↑. Home is one row, End ' +
          `is ${String(formulaRowBounds.max)}, and Space or Enter toggles between one and three. ` +
          'It is a separator with a value in rows, so a screen reader reads the height as a number ' +
          'rather than as a control with no state.',
      ),
      html`${bar('rows-bar')}`,
      readout('rows-readout'),
      caption([
        'The coloured layer scrolls with the editor, so a formula longer than the box stays aligned with its own colouring.',
        'aria-multiline is set once the editor is taller than one row, and not before.',
      ]),
    );
  },
};

export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    fill('dense-bar', multiReferenceFormula, 'dense-readout');
    fillNames('dense-bar-name', workbookNames);
    return html`
      <div data-density="compact">
        ${stage(
          note(
            'The density mode a formula bar is actually used in — `densityModes.compact` names it ' +
              'in as many words. Every affordance still clears the 24 CSS-pixel floor, which ' +
              '`tests/browser/formula.spec.ts` measures rather than assumes.',
          ),
          html`${bar('dense-bar')}`,
          readout('dense-readout'),
        )}
      </div>
    `;
  },
};
