import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { controlStateSpecs } from '../../src/controls/control-states.ts';
import {
  alignmentSegments,
  note,
  segmentedKeyboard,
  segmentedScreenReader,
  stack,
} from './specimens.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/**
 * `<mjx-segmented-control>` — one choice out of a small set, all of it visible.
 *
 * **Tab into *The Alignment Set* and press the arrow keys.** They move *and choose*, which is what
 * a radio group does and is deliberately unlike the two list controls: there is no popup to commit
 * from, so a selection a person has arrowed onto is a selection they have made. Tab again: one
 * press leaves the whole group, because exactly one segment carries the tab stop.
 *
 * Every paint here is the shared control table's, through the same `data-pressed` attribute
 * `<mjx-toggle-button>` writes. A segmented control with a green of its own would be a design
 * system with two greens, and `tests/inputs.test.ts` asserts the fingerprints are *identical*
 * rather than similar.
 */

const paints: readonly TokenPath[] = [
  'theme.light.accentSurface',
  'theme.light.accentBorder',
  'theme.light.accent',
  'theme.light.background',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'duration.transition',
  'ease.outSoft',
  'fontWeight.bold',
  'fontWeight.medium',
  'radius.chip',
  'radius.control',
  'spacing',
  'text.sm',
];

const conventions = storyConventions({
  statesMatrix: [
    'rest',
    'hover',
    'active',
    'focus',
    'on',
    'onHover',
    'unavailable',
    'disabled',
  ].map((state) => ({
    name: state,
    description: controlStateSpecs[state as keyof typeof controlStateSpecs].description,
  })),
  tokenDependencies: paints,
  keyboard: segmentedKeyboard,
  screenReader: segmentedScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Inputs/Segmented Control',
  parameters: {
    docs: {
      description: {
        component:
          'A radio group with a roving tab stop — the one control in this child that does not use ' +
          'aria-activedescendant, because a radio group’s members are real focusable controls and ' +
          'ARIA’s pattern moves focus between them. It still holds exactly one tab stop.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const segments = (chosen: string) => html`
  ${alignmentSegments.map(
    (segment) => html`
      <mjx-segment value=${segment.value} label=${segment.label}></mjx-segment>
    `,
  )}
  ${chosen === '' ? html`` : html``}
`;

/** **The story the paint and the tab-stop gates read.** */
export const TheAlignmentSet: Story = {
  name: 'The Alignment Set',
  render: () => html`
    ${note(
      'Four segments, one chosen. The chosen one is the shared table’s `on` — an accent tint and a ' +
        'bold label — because a segment is a toggle button and this catalogue has one answer to ' +
        'what pressed looks like.',
    )}
    ${stack(
      html`<mjx-segmented-control id="alignment" label="Alignment" value="left">
        ${segments('left')}
      </mjx-segmented-control>`,
    )}
  `,
};

/** Two of them beside each other, which is what a paragraph inspector looks like. */
export const InAPropertiesRow: Story = {
  name: 'In A Properties Row',
  render: () => html`
    ${note(
      'A second, smaller set beside the first. Tab through the pair: two stops for two groups, ' +
        'and the arrows never carry the keyboard out of the group it is in.',
    )}
    ${stack(
      html`<div>
        <mjx-label for="row-alignment">Alignment</mjx-label>
        <mjx-segmented-control id="row-alignment" value="center">
          ${segments('center')}
        </mjx-segmented-control>
      </div>`,
      html`<div>
        <mjx-label for="row-direction">Text direction</mjx-label>
        <mjx-segmented-control id="row-direction" value="ltr">
          <mjx-segment value="ltr" label="Left to right"></mjx-segment>
          <mjx-segment value="rtl" label="Right to left"></mjx-segment>
        </mjx-segmented-control>
      </div>`,
    )}
  `,
};

/** One member that exists and cannot be chosen. */
export const AMemberThatCannotBeChosen: Story = {
  name: 'A Member That Cannot Be Chosen',
  render: () => html`
    ${note(
      'Justify is unavailable. The arrows still reach it — reachable and refused, exactly as an ' +
        'unavailable menu row is — and pressing it changes nothing. A member the arrows skipped ' +
        'would be a member nobody can find out exists.',
    )}
    ${stack(
      html`<mjx-segmented-control id="with-unavailable" label="Alignment" value="left">
        <mjx-segment value="left" label="Left"></mjx-segment>
        <mjx-segment value="center" label="Centre"></mjx-segment>
        <mjx-segment value="right" label="Right"></mjx-segment>
        <mjx-segment
          value="justify"
          label="Justify"
          unavailable
          explanation="A single-line paragraph cannot be justified."
        ></mjx-segment>
      </mjx-segmented-control>`,
    )}
  `,
};

/** Right to left, where the arrows mirror. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () => html`
    ${note(
      'Arrow Right moves toward the *previous* segment, because the group runs the other way. ' +
        'Arrow Down does not mirror, for the same reason the slider’s does not.',
    )}
    <div dir="rtl">
      ${stack(
        html`<mjx-segmented-control id="rtl-segments" label="محاذاة" value="left">
          ${segments('left')}
        </mjx-segmented-control>`,
      )}
    </div>
  `,
};
