import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  demotionTable,
  ladderTable,
  note,
  ribbonKeyboard,
  ribbonScreenReader,
  ribbonStatesMatrix,
  ribbonTokenDependencies,
  wordTabHomeGroupsTemplate,
} from './specimens.ts';
import {
  wordCoreTabs,
  wordTabHomeControlCount,
  wordTabHomeGroups,
} from '../../dev/word-tab-home.ts';

/**
 * **The realistic worst case: Word's `TabHome`, at the size the committed census says it is.**
 *
 * 152 controls across thirteen groups, ten core tabs, degrading from a desktop to a phone. The
 * census is `docs/client-platform/data/command-surface.tsv` and `tests/ribbon.test.ts` reads it and
 * fails if `dev/word-tab-home.ts` has drifted from it — so the number in front of you is the one
 * the project measured rather than the one a ticket remembered. MJXOFF-183 says 165 across eight;
 * `dev/word-tab-home.ts` explains why the census wins.
 *
 * Seven of the thirteen groups hold one or two controls, which is exactly the shape that makes a
 * uniform collapse rule wrong: they are nobody's reason for opening Home, so they are `ancillary`
 * and give way while there is still room, and Font and Paragraph — 43 and 56 controls, and the
 * reason the tab exists — are `primary` and give way last.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal. See the note in ribbon.stories.ts.
  title: 'Ribbon/Word TabHome',
  parameters: {
    docs: {
      description: {
        component:
          'Word’s Home tab at 152 controls across thirteen groups, and the demotion rules that ' +
          'decide which of them survive into a phone toolbar.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const otherTabs = wordCoreTabs
  .filter((tab) => tab.id !== 'home')
  .map(
    (tab) => html`
      <mjx-ribbon-tab tab-id=${tab.id} label=${tab.label}>
        <mjx-ribbon-group label=${tab.label} priority="standard">
          <mjx-button label=${`${tab.label} command`} icon="document" size="large"></mjx-button>
        </mjx-ribbon-group>
      </mjx-ribbon-tab>
    `,
  );

/**
 * **The worst case.** Drive the container from `desktop` to `phone` and watch it degrade.
 *
 * What to look for, in this order: at 1440 the seven ancillary groups are already reduced while
 * Font and Paragraph are full; at 834 everything but the two primaries has collapsed to a button;
 * at 390 the tab strip itself is a picker and every group is a button with its essential commands
 * beside it. **No command is ever removed** — open any collapsed group and its whole contents are
 * there, because the popup is the same element and the same slot the strip was showing.
 */
export const TheWorstCase: Story = {
  name: 'The Worst Case',
  render: () => html`
    ${note(
      `Word’s TabHome: ${String(wordTabHomeControlCount)} controls across ` +
        `${String(wordTabHomeGroups.length)} groups, from the committed command surface. The ` +
        'named commands are Word’s; the numbered ones stand in for commands loop 2 names, ' +
        'because a made-up command name would be a worse lie than an obvious placeholder — and ' +
        'a placeholder occupies exactly as much of the layout as a command does.',
    )}
    <mjx-ribbon label="Word" selected="home">
      <mjx-ribbon-tab tab-id="home" label="Home">${wordTabHomeGroupsTemplate()}</mjx-ribbon-tab>
      ${otherTabs}
    </mjx-ribbon>
  `,
};

/** The same tab, pinned to a phone, so the degradation can be read rather than reproduced. */
export const TheWorstCaseOnAPhone: Story = {
  name: 'The Worst Case On A Phone',
  render: () => html`
    ${note(
      'The container is pinned to 390px. Thirteen buttons and seven essential commands, and every ' +
        'one of the 152 is still reachable. Open Font: Bold, Italic and Underline never left, and ' +
        'the other forty are behind the button.',
    )}
    <mjx-resizable-container width="390">
      <mjx-ribbon label="Word" selected="home">
        <mjx-ribbon-tab tab-id="home" label="Home">${wordTabHomeGroupsTemplate()}</mjx-ribbon-tab>
        ${otherTabs}
      </mjx-ribbon>
    </mjx-resizable-container>
  `,
};

/**
 * **The command demotion rules — the design decision this child owes the audit.**
 *
 * Both tables are drawn from the model rather than written here, so a rule that changes changes the
 * page. The ceiling of three per group is the one a gate can enforce, and it does.
 */
export const TheDemotionRules: Story = {
  name: 'The Demotion Rules',
  render: () => html`
    ${note(
      'A command survives a collapse only if all four hold. The one that matters most is the ' +
        'last: nothing is ever removed. A collapsed group holds every command it held when it ' +
        'was full, in the same DOM nodes, so reachability is structural rather than remembered.',
    )}
    ${demotionTable()} ${ladderTable()}
    ${note(
      'In this specimen: Font keeps Bold, Italic and Underline; Paragraph keeps the three ' +
        'alignments; Editing keeps Find. Clipboard keeps none, deliberately — its four verbs are ' +
        'on the keyboard anyway, which is also why it is declared secondary and gives way early.',
    )}
    <mjx-resizable-container width="390">
      <mjx-ribbon label="Word" selected="home">
        <mjx-ribbon-tab tab-id="home" label="Home">${wordTabHomeGroupsTemplate()}</mjx-ribbon-tab>
      </mjx-ribbon>
    </mjx-resizable-container>
  `,
};
