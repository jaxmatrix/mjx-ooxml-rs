import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { labelKeyboard, labelScreenReader, labelTokenDependencies, note, stack } from './specimens.ts';

/**
 * `<mjx-label>` — a field's name, and the one honest way to attach it across a shadow boundary.
 *
 * **Open *What It Actually Does* and inspect the field beside each label.** The label's text has
 * become the control's `label` attribute, and from there its accessible name. `for` and
 * `aria-labelledby` are IDREF attributes and an IDREF resolves inside one tree, so either of them
 * pointing into a component's shadow root would produce a label that looks attached in the markup
 * and announces nothing — which is the worst available outcome, because it passes review.
 *
 * Then look at the third pair, where the control already has a `label` of its own. The caption does
 * **not** overwrite it. Author a default only where nothing exists.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'rest', description: 'A caption above a field, at the label type role.' },
    { name: 'required', description: 'An asterisk in the honey accent, and the word “required” announced beside it.' },
    { name: 'with a hint', description: 'A second line of secondary text under the caption — on the surface, never inside a field.' },
    { name: 'bound', description: 'Carries `for`, so a press focuses the control and the text becomes its name.' },
    { name: 'unbound', description: 'A caption belonging to a row of controls rather than to one — a properties inspector’s common case.' },
  ],
  tokenDependencies: labelTokenDependencies(),
  keyboard: labelKeyboard,
  screenReader: labelScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Inputs/Label',
  parameters: {
    docs: {
      description: {
        component:
          'Pushes its text into the control’s `label` attribute, which every input in this child ' +
          'puts on its inner focusable as the accessible name. One string, in the place the ' +
          'platform already computes names from — and never an override of a name the control ' +
          'already declares.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** **The story the naming gate reads.** */
export const WhatItActuallyDoes: Story = {
  name: 'What It Actually Does',
  render: () => html`
    ${note(
      'Three pairs. The first two take their names from their labels; the third keeps the name it ' +
        'already declared, and the caption is decoration beside it. Press each caption — the ' +
        'control it names takes focus, which is the behaviour a native <label> would have given ' +
        'and which is implemented here rather than inherited.',
    )}
    ${stack(
      html`<div>
        <mjx-label for="named-size">Font size</mjx-label>
        <mjx-measure-input id="named-size" value="12" unit="pt"></mjx-measure-input>
      </div>`,
      html`<div>
        <mjx-label for="named-spacing">Line spacing</mjx-label>
        <mjx-slider id="named-spacing" min="1" max="3" step="0.05" value="1.15" suffix="×">
        </mjx-slider>
      </div>`,
      html`<div>
        <mjx-label for="already-named">Caption that is only a caption</mjx-label>
        <mjx-checkbox id="already-named" label="Keep with next"></mjx-checkbox>
      </div>`,
    )}
  `,
};

/** Required, and the word that goes with the mark. */
export const RequiredAndHinted: Story = {
  name: 'Required And Hinted',
  render: () => html`
    ${note(
      'The asterisk is drawn and the word “required” is announced beside it, off-screen. An ' +
        'asterisk on its own is a convention a screen reader reads as “star” and a person who has ' +
        'not met the convention reads as nothing. The hint is secondary text on the surface — ' +
        'never inside a field, where a hover fill would put it at 4.32 : 1.',
    )}
    ${stack(
      html`<div>
        <mjx-label for="required-name" required hint="Shown in the document’s properties.">
          Title
        </mjx-label>
        <mjx-combo-box id="required-name" allow-custom placeholder="Untitled">
          <mjx-option value="report" label="Quarterly report"></mjx-option>
          <mjx-option value="memo" label="Memo"></mjx-option>
        </mjx-combo-box>
      </div>`,
      html`<div>
        <mjx-label for="optional-name" hint="Left blank for most documents.">Subject</mjx-label>
        <mjx-combo-box id="optional-name" allow-custom placeholder="None">
          <mjx-option value="finance" label="Finance"></mjx-option>
        </mjx-combo-box>
      </div>`,
    )}
  `,
};

/** A caption with no `for`, which is the commonest kind. */
export const ACaptionForAGroup: Story = {
  name: 'A Caption For A Group',
  render: () => html`
    ${note(
      'A label with no `for` names nothing and focuses nothing. That is not an oversight: a group ' +
        'heading in a properties inspector belongs to a row of controls rather than to one of ' +
        'them, and giving it a spurious `for` would attach a name to whichever control happened ' +
        'to be first.',
    )}
    ${stack(
      html`<div>
        <mjx-label>Paragraph spacing</mjx-label>
        <div style="display:flex;gap:var(--mjx-density-gutter)">
          <mjx-measure-input label="Before" value="0" unit="pt"></mjx-measure-input>
          <mjx-measure-input label="After" value="8" unit="pt"></mjx-measure-input>
        </div>
      </div>`,
    )}
  `,
};
