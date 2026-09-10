import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  commandBarOrder,
  commandBarPartition,
  mobileFormFactors,
  type MobileCommand,
} from '../../src/mobile/mobile-model.ts';
import { demotionRules, essentialCommandLimit } from '../../src/ribbon/ribbon-model.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  barStates,
  caption,
  demotionCaption,
  documentArea,
  mobileTokenDependencies,
  note,
  phoneStage,
  reachCaption,
  wordPhoneCommands,
} from './specimens.ts';

/**
 * `<mjx-command-bar>` — the phone's ribbon, and the place U04's demotion rules become visible.
 *
 * **Open *The Demotion Ladder, Made Visible* first**, at the phone preset. The bar is Word's Home
 * tab declared in the order Office lays it out — Clipboard first — and what you see is Bold,
 * Italic and Underline, because Clipboard is `secondary` and Font is `primary`. Then press the
 * overflow control and watch what happens: the rail does not gain a menu, it *becomes* the grid.
 * The same DOM nodes, laid out differently. That is why nothing here can be lost.
 *
 * **Then set the container to tablet.** Eight slots, and the bar stops spanning and floats clear of
 * the edges — an 834 px full-bleed bar is a long way for one thumb.
 *
 * **Then narrow the browser window until it is short** and go back to tablet width. That is the
 * landscape phone, and it is the one presentation a container query alone cannot find.
 */

const conventions = storyConventions({
  statesMatrix: barStates(),
  tokenDependencies: mobileTokenDependencies,
  keyboard: [
    { keys: 'Tab', does: 'Enters the bar at its one tab stop — a toolbar holds a roving stop.' },
    { keys: 'ArrowRight / ArrowLeft', does: 'Moves the stop along the rail, wrapping at each end.' },
    { keys: 'Home / End', does: 'Jumps to the first or last reachable control.' },
    { keys: 'Enter / Space', does: 'Activates the focused command.' },
    { keys: 'Escape', does: 'Closes the overflow grid and returns focus to the overflow control.' },
  ],
  screenReader:
    'Announces "Commands, toolbar", then the focused command\'s name. The overflow control ' +
    'announces "More commands, collapsed" and its expanded state changes as it opens; opening it ' +
    'also announces "More commands shown" politely.',
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Mobile/Command Bar',
  parameters: {
    docs: {
      description: {
        component:
          'One scrollable row of the highest-priority commands with an overflow control, pinned ' +
          'to the block end of a phone. Its order is U04’s command-demotion ladder and not a ' +
          'second priority scheme, and every demoted command stays in the same rail.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

function bar(commands: readonly MobileCommand[], attributes: { open?: boolean } = {}) {
  return html`
    <mjx-command-bar
      id="bar"
      label="Home"
      ?open-overflow=${attributes.open ?? false}
      .commands=${commands}
    ></mjx-command-bar>
  `;
}

/** The whole point of the component, at the width it exists for. */
export const TheDemotionLadderMadeVisible: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'Word’s Home tab, declared in ribbon order — Clipboard first. What survives into the ' +
          'visible run is Font’s, because Font is the group the tab exists for. Scroll the rail ' +
          'sideways, or open the overflow: every command declared is still there.',
      )}
      ${phoneStage(
        documentArea('The document. The bar sits under it, where a thumb is.'),
        bar(wordPhoneCommands),
      )}
      ${caption(demotionCaption('phonePortrait'))}
    `,
};

/** The same bar with the rail presented as a grid. */
export const TheOverflowIsTheSameRail: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'The overflow is open. Count the commands: the grid holds every one of them, including ' +
          'the three that were visible a moment ago — because it is the same element, not a ' +
          'second copy rendered into a menu. The rail’s touch-action changes with it, from ' +
          'pan-x to pan-y, which is what makes a horizontal scroller become a vertical one ' +
          'without either of them fighting the document.',
      )}
      ${phoneStage(documentArea('The document.'), bar(wordPhoneCommands, { open: true }))}
      ${caption([
        `Every command is in the rail at both presentations: ${String(wordPhoneCommands.length)}.`,
        'A command with a popup is never in the visible run — demotion rule 1 — but it is always ' +
          'in the rail.',
      ])}
    `,
};

/** Three form factors, three answers, one component. */
export const ThreeFormFactors: Story = {
  render: () =>
    html`
      ${note(
        'Change the container preset in the toolbar and watch the visible run change with it. ' +
          'Then make the browser window short — under 500 px — and set the container to tablet: ' +
          'that is a landscape phone, and the bar loses its block padding rather than its ' +
          'commands.',
      )}
      ${phoneStage(documentArea('The document.'), bar(wordPhoneCommands))}
      ${caption([
        ...(['phonePortrait', 'phoneLandscape', 'smallTablet'] as const).map(
          (factor) =>
            `${factor}: ${String(mobileFormFactors[factor].visibleSlots)} slots, ` +
            `${mobileFormFactors[factor].spans ? 'spans the container' : 'floats clear of it'}.`,
        ),
      ])}
    `,
};

/** The rules themselves, drawn from the model so the page cannot disagree with it. */
export const TheRulesItFollows: Story = {
  render: () =>
    html`
      ${note(
        'These are MJXOFF-183’s rules, not a second set. The table below is generated from ' +
          'demotionRules, so this page cannot say something the ribbon does not.',
      )}
      <table
        class=${typeRoleClass('body')}
        style="border-collapse:collapse;margin:calc(var(--mjx-density-gutter) * 2);
               color:var(--theme-text-primary);max-inline-size:80ch"
      >
        <thead>
          <tr>
            ${['Rule', 'Because', 'Checked by'].map(
              (heading) =>
                html`<th
                  style="text-align:start;padding:var(--mjx-density-step);
                         border-block-end:1px solid var(--theme-border)"
                >
                  ${heading}
                </th>`,
            )}
          </tr>
        </thead>
        <tbody>
          ${demotionRules.map(
            (rule) => html`
              <tr>
                ${[rule.rule, rule.because, rule.checkedBy].map(
                  (cell) =>
                    html`<td
                      style="vertical-align:top;padding:var(--mjx-density-step);
                             border-block-end:1px solid var(--theme-border-subtle)"
                    >
                      ${cell}
                    </td>`,
                )}
              </tr>
            `,
          )}
        </tbody>
      </table>
      ${caption([
        `The per-group ceiling is ${String(essentialCommandLimit)}, and it is the ribbon's own ` +
          'constant.',
        `On this fixture Font declares four essential commands, so the ceiling bites: ` +
          `${commandBarPartition(wordPhoneCommands, 4)
            .visible.map((command) => command.label)
            .join(', ')} is what four slots produce.`,
        `The rail's full order is ${commandBarOrder(wordPhoneCommands)
          .map((command) => command.label)
          .join(' → ')}.`,
      ])}
    `,
};

/** Reachability, which is a layout constraint rather than a preference. */
export const WithinThumbReach: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'The band below the line is what one thumb reaches on a large phone without a grip ' +
          'shift. The bar is inside it because it is pinned to the block end, which is the whole ' +
          'reason a phone’s commands live at the bottom and a desktop’s live at the top.',
      )}
      <div style="position:relative">
        ${phoneStage(documentArea('The document.'), bar(wordPhoneCommands))}
        <div
          aria-hidden="true"
          style="position:absolute;inset-inline:0;inset-block-start:45%;
                 border-block-start:1px dashed var(--theme-accent-border)"
        ></div>
      </div>
      ${caption(reachCaption(30 * 16))}
    `,
};
