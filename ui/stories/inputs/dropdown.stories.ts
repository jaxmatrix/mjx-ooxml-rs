import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MjxDropdown } from '../../src/inputs/index.ts';
import {
  dropdownKeyboard,
  dropdownScreenReader,
  fontOptions,
  lineSpacingOptions,
  listStatesFor,
  listTokenDependencies,
  note,
  stack,
  stage,
} from './specimens.ts';

/**
 * `<mjx-dropdown>` — a **listbox**, not a menu, and the keyboard contract that follows.
 *
 * **Open *The Option States* and use the arrow keys.** The row the keyboard is on carries a ring
 * that a hovered row also carries, and the chosen row carries the accent tint. Both at once is a
 * fifth state, and it is the one where the weaker ring would have been missed — which is why the
 * cursor's ring is `accentPressed` rather than the control table's `accent`: 4.68 : 1 on an accent
 * fill rather than 2.98 : 1.
 *
 * Then press `Home`. It goes to the first option, because this field is not a text box. The combo
 * box next door inverts exactly that row, and one flag in the model decides both.
 */

const conventions = storyConventions({
  statesMatrix: listStatesFor(),
  tokenDependencies: listTokenDependencies(),
  keyboard: dropdownKeyboard,
  screenReader: dropdownScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Inputs/Dropdown',
  parameters: {
    docs: {
      description: {
        component:
          'Choose one of a fixed set. One tab stop, focus never leaving the field, and ' +
          'aria-activedescendant pointing at the option the keyboard is on — the ARIA combobox ' +
          'pattern, which is a different pattern from a menu and gives different keys.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Open the list as soon as the story has rendered, so an auditor sees the rows without a click. */
function openOnLoad(select: string, options: { value?: string; active?: number } = {}): void {
  requestAnimationFrame(() => {
    const found = document.querySelector(select);
    if (found === null) return;
    const dropdown = found as MjxDropdown;
    if (options.value !== undefined) dropdown.value = options.value;
    dropdown.openList();
    // Through the surface, which is what a key press moves — not by writing an attribute, which
    // would be the story staging a picture of a state rather than producing one.
    if (options.active !== undefined) dropdown.surface?.setActive(options.active);
  });
}

/** **The states matrix, on live rows.** */
export const TheOptionStates: Story = {
  name: 'The Option States',
  render: () => {
    openOnLoad('#option-states', { value: '1.15', active: 3 });
    return html`
      ${note(
        'All five option states at once, and none of them forced: 1.15 is chosen, the keyboard is ' +
          'on Double, “Exactly…” is unavailable and the rest are resting. Arrow up twice and the ' +
          'chosen row and the cursor coincide, which is the fifth. The gate reads each row’s own ' +
          'aria-selected, data-active and aria-disabled and compares the paint against the model.',
      )}
      <div style=${stage('26rem')}>
        <mjx-dropdown id="option-states" label="Line spacing" style="inline-size:16rem">
          ${lineSpacingOptions.map(
            (option) => html`
              <mjx-option
                value=${option.value}
                label=${option.label}
                description=${option.description ?? ''}
                ?unavailable=${option.unavailable === true}
                explanation=${option.explanation ?? ''}
              ></mjx-option>
            `,
          )}
        </mjx-dropdown>
      </div>
    `;
  },
};

/** The closed field, in its own states. */
export const TheClosedField: Story = {
  name: 'The Closed Field',
  render: () => html`
    ${note(
      'A dropdown holding a value, a dropdown holding none, and one that cannot be used. The ' +
        'placeholder is secondary text on the field’s own fill — which never changes, in any ' +
        'state, precisely so that this line stays at 5.23 : 1 while the pointer is over it.',
    )}
    ${stack(
      html`<mjx-dropdown label="Line spacing" value="1.5">
        ${lineSpacingOptions.map(
          (option) => html`<mjx-option value=${option.value} label=${option.label}></mjx-option>`,
        )}
      </mjx-dropdown>`,
      html`<mjx-dropdown label="Line spacing" placeholder="Choose…">
        ${lineSpacingOptions.map(
          (option) => html`<mjx-option value=${option.value} label=${option.label}></mjx-option>`,
        )}
      </mjx-dropdown>`,
      html`<mjx-dropdown
        label="Line spacing"
        value="1.0"
        unavailable
        explanation="The paragraph inherits its spacing from its style."
      >
        ${lineSpacingOptions.map(
          (option) => html`<mjx-option value=${option.value} label=${option.label}></mjx-option>`,
        )}
      </mjx-dropdown>`,
    )}
  `,
};

/**
 * **The story the virtualisation gate reads.**
 *
 * Thirty-seven fonts in two sections, of which eight are ever visible.
 */
export const ALongList: Story = {
  name: 'A Long List',
  render: () => {
    openOnLoad('#long-list', { value: 'cambria' });
    return html`
      ${note(
        'Thirty-seven options in two sections. Scroll the list: the rows outside the window are ' +
          'heights rather than elements, and the plan comes from the gallery’s own row planner at ' +
          'one column rather than from a second windowing function written for lists.',
      )}
      <div style=${stage('30rem')}>
        <mjx-dropdown id="long-list" label="Font" style="inline-size:18rem">
          ${fontOptions.map(
            (option) => html`
              <mjx-option
                value=${option.value}
                label=${option.label}
                category=${option.category ?? ''}
                ?unavailable=${option.unavailable === true}
                explanation=${option.explanation ?? ''}
              ></mjx-option>
            `,
          )}
        </mjx-dropdown>
      </div>
    `;
  },
};

/** Near the bottom of a short container, so the list has to flip. */
export const AtTheBottomOfItsRoom: Story = {
  name: 'At The Bottom Of Its Room',
  render: () => {
    openOnLoad('#flipping', { value: '1.0' });
    return html`
      ${note(
        'The field is at the foot of what clips it, so the list opens upward. The placement is ' +
          '`placeFloating` from the overlay module — the same arithmetic a menu and a gallery use ' +
          '— and the gate re-runs it in Node over the component’s own recorded inputs and requires ' +
          'the same answer.',
      )}
      <div
        style=${`${stage('18rem')};display:flex;align-items:flex-end`}
      >
        <mjx-dropdown id="flipping" label="Line spacing" style="inline-size:16rem">
          ${lineSpacingOptions.map(
            (option) => html`<mjx-option value=${option.value} label=${option.label}></mjx-option>`,
          )}
        </mjx-dropdown>
      </div>
    `;
  },
};

/** Under right-to-left, where the placement mirrors and the coordinates do not. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () => {
    openOnLoad('#rtl-dropdown', { value: '1.15' });
    return html`
      ${note(
        'The list lines up with the field’s end rather than its start, because `start` is a ' +
          'logical alignment. The coordinates it is placed at are physical, which is the trap the ' +
          'menu model records and this inherits by using the same placement.',
      )}
      <div dir="rtl" style=${stage('24rem')}>
        <mjx-dropdown id="rtl-dropdown" label="تباعد الأسطر" style="inline-size:16rem">
          ${lineSpacingOptions.map(
            (option) => html`<mjx-option value=${option.value} label=${option.label}></mjx-option>`,
          )}
        </mjx-dropdown>
      </div>
    `;
  },
};
