import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  ladderGroups,
  ladderTable,
  note,
  ribbonKeyboard,
  ribbonScreenReader,
  ribbonStatesMatrix,
  ribbonTokenDependencies,
} from './specimens.ts';
import { contextualTabSets } from '../../dev/word-tab-home.ts';

/**
 * `<mjx-ribbon>`, `<mjx-ribbon-tab>`, `<mjx-ribbon-group>` and `<mjx-contextual-tab-set>` — the
 * structure the 10,518 controls of MJXOFF-182 live in.
 *
 * The story worth opening first is **The Priority Ladder**, and the way to read it is to drag the
 * container's handle or press its presets. Four identical groups differ only in the priority they
 * declare, and they give way one at a time. **Nothing here responds to the window**: the ribbon is
 * its own container query container, so a group inside a narrow task pane collapses in a wide
 * window, which is the whole point and is what `tests/browser/ribbon.spec.ts` asserts by driving
 * the container while the viewport stays exactly where it is.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal, and it must stay one — Storybook indexes CSF statically and refuses a
  // computed title. `tests/browser/ribbon.spec.ts` looks every story up by this string and fails
  // with "is missing from the catalogue" if the two diverge.
  title: 'Ribbon/Ribbon',
  parameters: {
    docs: {
      description: {
        component:
          'The ribbon’s containers: a group with three progressive presentations, a tab strip ' +
          'that becomes a picker, contextual tab sets, and the three ribbon states. Resize the ' +
          'container, do not resize the window — the mechanism is @container and a viewport ' +
          'media query would be wrong here in a way that still looked right.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The two ordinary tabs every story below shares, so the tab strip is a real strip. */
const coreTabs = (content: unknown) => html`
  <mjx-ribbon-tab tab-id="home" label="Home">${content}</mjx-ribbon-tab>
  <mjx-ribbon-tab tab-id="insert" label="Insert">
    <mjx-ribbon-group label="Tables" priority="standard">
      <mjx-button label="Table" icon="table" size="large"></mjx-button>
    </mjx-ribbon-group>
  </mjx-ribbon-tab>
  <mjx-ribbon-tab tab-id="review" label="Review">
    <mjx-ribbon-group label="Comments" priority="standard">
      <mjx-button label="New Comment" icon="comment" size="large"></mjx-button>
    </mjx-ribbon-group>
  </mjx-ribbon-tab>
`;

/**
 * **The story the presentation gate reads.**
 *
 * Four groups, identical but for their declared priority. Press the container's `tablet` preset and
 * watch three of them reduce while `ancillary` collapses; press `phone` and watch all four
 * collapse — and then open one, because *"a control that disappears at narrow width has not
 * degraded, it has been lost"* and the popup is where it did not.
 */
export const ThePriorityLadder: Story = {
  name: 'The Priority Ladder',
  render: () => html`
    ${note(
      'Four groups that differ only in priority. Drag the container narrower: ancillary gives way ' +
        'first, then secondary, then standard, and primary last. The window never moves.',
    )}
    ${ladderTable()}
    <mjx-ribbon label="Ladder" selected="home">${coreTabs(ladderGroups())}</mjx-ribbon>
  `,
};

/**
 * **Why the ladder is not a global breakpoint.**
 *
 * One container, one width, three different presentations on screen at once. A uniform rule cannot
 * produce this picture, which is what makes it the answer to *"groups must be able to declare their
 * own"*.
 */
export const ThreePresentationsAtOneWidth: Story = {
  name: 'Three Presentations At One Width',
  render: () => html`
    ${note(
      'The container is pinned to 1000px. At that width primary is full, standard and secondary ' +
        'are reduced, and ancillary is collapsed — three presentations, one width, because the ' +
        'priority is the group’s own.',
    )}
    <mjx-resizable-container width="1000">
      <mjx-ribbon label="Ladder" selected="home">${coreTabs(ladderGroups())}</mjx-ribbon>
    </mjx-resizable-container>
  `,
};

/** The narrow case, pinned: every group collapsed and the tab strip reduced to its picker. */
export const OnAPhone: Story = {
  name: 'On A Phone',
  render: () => html`
    ${note(
      'The container is pinned to 390px. Every group is one button; the commands marked essential ' +
        'are still beside it; the rest are one press away in the popup. The tab strip is a picker ' +
        'holding the whole tablist, because a strip that scrolled sideways would hide tabs with ' +
        'nothing on screen to say they exist.',
    )}
    <mjx-resizable-container width="390">
      <mjx-ribbon label="Ladder" selected="home">${coreTabs(ladderGroups())}</mjx-ribbon>
    </mjx-resizable-container>
  `,
};

/** Expanded, collapsed to its tabs, and hidden — the three states, side by side. */
export const RibbonStates: Story = {
  name: 'Ribbon States',
  render: () => html`
    ${note(
      'Expanded is the ordinary state. Collapsed-to-tabs gives the document its room back and ' +
        'keeps the commands one press away. Hidden draws nothing but the button that brings it ' +
        'back — never display:none on the whole element, because a ribbon a keyboard cannot ' +
        'restore is a ribbon a keyboard has lost.',
    )}
    <div style="display:grid;gap:calc(var(--mjx-density-gutter) * 2)">
      <mjx-ribbon label="Expanded" selected="home" state="expanded">
        ${coreTabs(ladderGroups())}
      </mjx-ribbon>
      <mjx-ribbon label="Collapsed" selected="home" state="tabs">
        ${coreTabs(ladderGroups())}
      </mjx-ribbon>
      <mjx-ribbon label="Hidden" selected="home" state="hidden">
        ${coreTabs(ladderGroups())}
      </mjx-ribbon>
    </div>
  `,
};

/** Office's single-row form: every group at most reduced, at every width. */
export const TheSimplifiedForm: Story = {
  name: 'The Simplified Form',
  render: () => html`
    ${note(
      'Simplified refuses the full presentation and never prevents a collapse, which is why it ' +
        'sits between the two container conditions in the cascade rather than after both. The ' +
        'same ribbon at the same width as The Priority Ladder.',
    )}
    <mjx-ribbon label="Simplified" selected="home" simplified>
      ${coreTabs(ladderGroups())}
    </mjx-ribbon>
  `,
};

/** A small group, imperatively, so the contextual story can put one behind a new tab. */
function demoGroup(label: string, command: string, icon: string): HTMLElement {
  const group = document.createElement('mjx-ribbon-group');
  group.setAttribute('label', label);
  group.setAttribute('priority', 'standard');
  const button = document.createElement('mjx-button');
  button.setAttribute('label', command);
  button.setAttribute('icon', icon);
  button.setAttribute('size', 'large');
  group.append(button);
  return group;
}

const contextualRibbonId = 'contextual-demo';

function tableToolsSet(): HTMLElement {
  const set = document.createElement('mjx-contextual-tab-set');
  set.setAttribute('label', 'Table Tools');
  for (const tab of [
    { id: 'table-design', label: 'Design', command: 'Table Styles', icon: 'table' },
    { id: 'table-layout', label: 'Layout', command: 'Merge Cells', icon: 'add' },
  ]) {
    const panel = document.createElement('mjx-ribbon-tab');
    panel.setAttribute('tab-id', tab.id);
    panel.setAttribute('label', tab.label);
    panel.append(demoGroup(tab.command, tab.command, tab.icon));
    set.append(panel);
  }
  return set;
}

/**
 * **The general mechanism, with three of the sets the census records.**
 *
 * Picture Tools and Chart Tools are here from the start; Table Tools appears and disappears on the
 * buttons above, which is what a selection actually does. Two things are being watched here and
 * neither is visible in a screenshot: **an appearance must not steal focus** — put the keyboard on
 * a tab first, then press *Select a table* — and **a disappearance while its tab is selected must
 * move selection somewhere sane**, which is back to the last core tab that was selected.
 */
export const ContextualTabSets: Story = {
  name: 'Contextual Tab Sets',
  render: () => html`
    ${note(
      'A contextual set is a titled band over its own tabs. The band is a picture, so each tab ' +
        'carries the set’s name in its accessible name too — “Design, Table Tools” — and the ' +
        'appearance is announced politely. Three sets, one mechanism.',
    )}
    <div style="display:flex;gap:var(--mjx-density-gutter);margin-block-end:var(--mjx-density-gutter)">
      <button
        type="button"
        data-action="select-table"
        @click=${() => {
          const ribbon = document.getElementById(contextualRibbonId);
          if (ribbon === null || ribbon.querySelector('[tab-id="table-design"]') !== null) return;
          ribbon.append(tableToolsSet());
        }}
      >
        Select a table
      </button>
      <button
        type="button"
        data-action="deselect-table"
        @click=${() => {
          const ribbon = document.getElementById(contextualRibbonId);
          ribbon?.querySelector('mjx-contextual-tab-set[label="Table Tools"]')?.remove();
        }}
      >
        Deselect it
      </button>
    </div>
    <mjx-ribbon id=${contextualRibbonId} label="Word" selected="home">
      ${coreTabs(ladderGroups())}
      ${contextualTabSets
        .filter((set) => set.label !== 'Table Tools')
        .map(
          (set) => html`
            <mjx-contextual-tab-set label=${set.label}>
              ${set.tabs.map(
                (tab) => html`
                  <mjx-ribbon-tab tab-id=${tab.id} label=${tab.label}>
                    <mjx-ribbon-group label=${set.label} priority="standard">
                      <mjx-button
                        label=${`${set.label} ${tab.label}`}
                        icon="slide-layout"
                        size="large"
                      ></mjx-button>
                    </mjx-ribbon-group>
                  </mjx-ribbon-tab>
                `,
              )}
            </mjx-contextual-tab-set>
          `,
        )}
    </mjx-ribbon>
  `,
};
