import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { formulaEvents, type NavigationRequest } from '../../src/formula/formula-model.ts';
import { inputEvents } from '../../src/inputs/input-model.ts';
import {
  caption,
  fillNames,
  formulaKeyboard,
  formulaScreenReader,
  formulaStates,
  formulaTokenDependencies,
  note,
  readout,
  stage,
  workbookNames,
} from './specimens.ts';

/**
 * `<mjx-name-box>` — the address display that is also a combo box and also a navigation input.
 *
 * **Open *Typing An Address Navigates* and type `b7`, then `Q1`, then `XFE1`.** Three different
 * answers, and the middle one is the interesting one: this workbook has a defined name spelled
 * `Q1`, and a name box that sent a person to cell Q1 instead would be this library overriding what
 * their own file says that word means. The name wins, and the readout says which kind of request
 * was emitted.
 *
 * `GUESS:` **lowercase is accepted here and refused by the grammar.** `crates/mjx-sml/src/address.rs`
 * does not fold case, deliberately — a *file reader* that accepted `a1` would accept bytes Excel
 * would not have written. A *person* typing into a name box does expect `a1` to work, so the
 * up-casing happens in this component, before the grammar sees the text, and the grammar stays as
 * strict as the Rust one.
 *
 * It does not move a selection: there is no grid in this loop. It **requests** one, with the parsed
 * range and its ordered bounds attached, so whatever owns the grid has nothing left to parse.
 */

const conventions = storyConventions({
  statesMatrix: formulaStates,
  tokenDependencies: formulaTokenDependencies,
  keyboard: formulaKeyboard,
  screenReader: formulaScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Excel Chrome/Name Box',
  parameters: {
    docs: {
      description: {
        component:
          'The name box: the current address, a combo box over the workbook’s defined names and ' +
          'tables, and a navigation input. It is U07’s combo box with one method replaced.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Report what a name box asked for, and what it refused. */
function report(boxId: string, readoutId: string): void {
  queueMicrotask(() => {
    const box = document.getElementById(boxId);
    const output = document.getElementById(readoutId);
    if (!(box instanceof HTMLElement) || !(output instanceof HTMLElement)) return;
    box.addEventListener(formulaEvents.navigate, (event) => {
      const request = (event as CustomEvent<NavigationRequest>).detail;
      if (request.kind === 'address') {
        output.textContent =
          `navigate  address  ${request.text}\n` +
          `bounds    rows ${String(request.bounds.firstRow + 1)}–${String(request.bounds.lastRow + 1)}` +
          `  cols ${String(request.bounds.firstColumn + 1)}–${String(request.bounds.lastColumn + 1)}` +
          (request.sheet === undefined ? '' : `\nsheet     ${request.sheet}`);
        return;
      }
      if (request.kind === 'name') {
        output.textContent =
          `navigate  ${request.entry.kind}  ${request.entry.name}\n` +
          `points at ${request.entry.definition}`;
        return;
      }
      output.textContent = `refused   ${request.problem}`;
    });
    box.addEventListener(inputEvents.invalid, (event) => {
      const detail = (event as CustomEvent<{ text: string }>).detail;
      output.textContent = `refused   “${detail.text}” — the box reverted to what it reports`;
    });
  });
}

export const TypingAnAddressNavigates: Story = {
  name: 'Typing An Address Navigates',
  render: () => {
    fillNames('nav-box', workbookNames);
    report('nav-box', 'nav-readout');
    return stage(
      note(
        'Type an address and press Enter: b7, a1:c9, A:C, or ‘Q1 Data’!A1. Then type a ' +
          'defined name — Revenue, SalesTable — or open the list and choose one. Both emit a ' +
          'navigation request; the readout says which kind and what it carried. Type XFE1 or A0 ' +
          'and the box reverts and says which rule refused it, by name.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);align-items:center">
          <mjx-name-box id="nav-box" label="Name box" address="B7" filter="startsWith"></mjx-name-box>
        </div>
      `,
      readout('nav-readout'),
      caption([
        'A name wins over an address spelled like one: this workbook defines Q1, so typing Q1 goes to the name.',
        'The grammar mirrors crates/mjx-sml/src/address.rs — the same grid limits, the same four range shapes, one problem per AddressError variant.',
        'A multi-cell selection is announced with its size, because its address does not carry one.',
      ]),
    );
  },
};

export const TheListOfNamesAndTables: Story = {
  name: 'The List Of Names And Tables',
  render: () => {
    fillNames('list-box', workbookNames);
    report('list-box', 'list-readout');
    return stage(
      note(
        'The list is grouped: defined names first, then tables. Each row carries what the name ' +
          'points at, because two names called Revenue and CostOfSales tell a person nothing about ' +
          'where they lead. Everything about the list — the filtering, the arrow keys, ' +
          'aria-activedescendant, Escape restoring the text as it was when the list opened — is ' +
          'U07’s combo box, inherited rather than re-implemented.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);align-items:center">
          <mjx-name-box id="list-box" label="Name box" address="B7"></mjx-name-box>
        </div>
      `,
      readout('list-readout'),
      caption([
        'One tab stop, so Tab means leave and an open list is a disclosure rather than a trap — U05’s rule, inherited.',
        'The only method this component replaces is resolveTyped, whose own documentation names this case in advance.',
      ]),
    );
  },
};

export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    fillNames('dense-box', workbookNames);
    report('dense-box', 'dense-readout');
    return html`
      <div data-density="compact">
        ${stage(
          note('The density a formula bar is used in. The field still clears the 24-pixel floor.'),
          html`<mjx-name-box id="dense-box" label="Name box" address="A1:C9"></mjx-name-box>`,
          readout('dense-readout'),
        )}
      </div>
    `;
  },
};
