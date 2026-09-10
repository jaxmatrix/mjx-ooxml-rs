import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  comboBoxKeyboard,
  comboBoxScreenReader,
  fieldStatesMatrix,
  fontOptions,
  listStatesFor,
  listTokenDependencies,
  note,
  stack,
  stage,
} from './specimens.ts';

/**
 * `<mjx-combo-box>` — a text field **and** a listbox, which is the control this child exists for.
 *
 * **Open *Typing Filters* and type `co`.** The list narrows to the fonts containing those letters,
 * the cursor lands on the first, the arrows move through the *filtered* set, `Enter` commits and
 * `Escape` puts back the text as it was when the list opened — not the value, the *text*, which is
 * the difference a person notices and a screenshot cannot show.
 *
 * Then type something that is not a font and press `Tab`. Without `allow-custom` the field goes
 * back to what it reports and fires `mjx-input-invalid` with what it refused; with it, the string
 * becomes the value. Both are stated, both are asserted, and neither is silent.
 */

const conventions = storyConventions({
  statesMatrix: listStatesFor(),
  tokenDependencies: listTokenDependencies(),
  keyboard: comboBoxKeyboard,
  screenReader: comboBoxScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Inputs/Combo Box',
  parameters: {
    docs: {
      description: {
        component:
          'An editable combo box with list autocomplete. The invariant it is built around: the ' +
          'value the field shows is the value the control reports, after every path — and the ' +
          'only moment the two may differ is while a person is actively typing.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const fontOptionMarkup = () =>
  fontOptions.map(
    (option) => html`
      <mjx-option
        value=${option.value}
        label=${option.label}
        category=${option.category ?? ''}
        ?unavailable=${option.unavailable === true}
        explanation=${option.explanation ?? ''}
      ></mjx-option>
    `,
  );

/** **The field states matrix** — the same seven a measure input shows, on the same table. */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => html`
    ${note(
      'The seven states of a field. The fill is the same in every one of them, which is the ' +
        'decision the whole table is built on: a placeholder on --theme-border-subtle would be ' +
        '4.32 : 1, so the hover state moves the edge instead. The specimen is a measure input ' +
        'because a field is a field; a combo box wears exactly these rules.',
    )}
    ${fieldStatesMatrix()}
  `,
};

/** **The story the filtering and commit gates read.** */
export const TypingFilters: Story = {
  name: 'Typing Filters',
  render: () => html`
    ${note(
      'Type `co`. Nine fonts remain, the cursor is on the first, and the arrows move through those ' +
        'nine rather than through all thirty-seven. Enter commits; Escape restores the text as it ' +
        'was when the list opened.',
    )}
    <div style=${stage('28rem')}>
      <mjx-combo-box
        id="filtering"
        label="Font"
        value="cambria"
        filter="contains"
        style="inline-size:18rem"
      >
        ${fontOptionMarkup()}
      </mjx-combo-box>
    </div>
  `,
};

/** The two answers to a string the list does not carry. */
export const AStringTheListDoesNotCarry: Story = {
  name: 'A String The List Does Not Carry',
  render: () => html`
    ${note(
      'Type “Helvetica” into each and press Tab. The first refuses: the field goes back to what it ' +
        'reports and fires mjx-input-invalid carrying what was refused, so a host can say why. The ' +
        'second accepts it, because `allow-custom` says a value outside the list is a value. Both ' +
        'end with the field showing exactly what the control reports.',
    )}
    ${stack(
      html`<mjx-combo-box id="strict" label="Font (from the list only)" value="cambria">
        ${fontOptionMarkup()}
      </mjx-combo-box>`,
      html`<mjx-combo-box id="permissive" label="Font (anything)" value="cambria" allow-custom>
        ${fontOptionMarkup()}
      </mjx-combo-box>`,
    )}
  `,
};

/** `startsWith` beside `contains`, because the choice changes what a person finds. */
export const TheTwoFilters: Story = {
  name: 'The Two Filters',
  render: () => html`
    ${note(
      'Type `co` into both. The first keeps Consolas, Constantia, Corbel and Courier New; the ' +
        'second also keeps Bookman Old Style, Comic Sans MS, Lucida Console and the rest that ' +
        'merely contain the letters. Neither is right in general, which is why it is an attribute.',
    )}
    ${stack(
      html`<mjx-combo-box label="Font (starts with)" filter="startsWith" placeholder="Type to filter…">
        ${fontOptionMarkup()}
      </mjx-combo-box>`,
      html`<mjx-combo-box label="Font (contains)" filter="contains" placeholder="Type to filter…">
        ${fontOptionMarkup()}
      </mjx-combo-box>`,
    )}
  `,
};

/** A task pane's worth of fields, which is the shape this control is actually used in. */
export const InAPropertiesPane: Story = {
  name: 'In A Properties Pane',
  render: () => html`
    ${note(
      'A label, a combo box, a dropdown and a measure input in a column, at compact density. Tab ' +
        'through it: four stops for four controls, and every open list closes when it is left.',
    )}
    <div data-density="compact" style=${stage('24rem')}>
      ${stack(
        html`<div>
          <mjx-label for="pane-font" required hint="The face the run is drawn with.">Font</mjx-label>
          <mjx-combo-box id="pane-font" value="cambria" allow-custom>
            ${fontOptionMarkup()}
          </mjx-combo-box>
        </div>`,
        html`<div>
          <mjx-label for="pane-size">Size</mjx-label>
          <mjx-measure-input id="pane-size" value="12" unit="pt" min="1" max="1638">
          </mjx-measure-input>
        </div>`,
      )}
    </div>
  `,
};
