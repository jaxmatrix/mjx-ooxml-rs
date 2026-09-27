import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MjxTree } from '../../src/navigators/index.ts';
import { subtreeIds, type TreeNode } from '../../src/navigators/navigator-model.ts';
import {
  caption,
  largeNavigatorCount,
  everyBranch,
  largeOutline,
  navigatorTokenDependencies,
  note,
  paneStyle,
  smallOutline,
  smallOutlineExpanded,
  stage,
  treeKeyboard,
  treeScreenReader,
  treeStates,
} from './specimens.ts';

/**
 * `<mjx-tree>` — Word's navigation pane and outline view.
 *
 * **Open *A Heading Moves With Its Subtree* first, with a keyboard.** Put the cursor on *Method*,
 * press `Alt + Arrow Up`, and watch three rows move rather than one. Then collapse it and do it
 * again: the same three rows move, because the model operates on the tree and not on the rows that
 * happen to be visible.
 *
 * `GUESS:` **outdent does not promote the following siblings.** Word's `Shift + Tab` in the outline
 * view takes the headings *after* the promoted one and reparents them under it. This does not: it
 * moves the heading and its own children and leaves everything else where it was. Promoting text a
 * person had not selected is the kind of edit that is noticed three saves later, and the narrower
 * behaviour is the one that cannot surprise anybody.
 */

const conventions = storyConventions({
  statesMatrix: treeStates,
  tokenDependencies: navigatorTokenDependencies,
  keyboard: treeKeyboard,
  screenReader: treeScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Navigators/Tree',
  parameters: {
    docs: {
      description: {
        component:
          'An ARIA tree: expand and collapse, multi-level indentation, selection, type-ahead, and ' +
          'a reorder that carries a heading’s subtree — by keyboard as well as by drag.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Fill a tree once it has upgraded. */
function fill(id: string, nodes: readonly TreeNode[], expanded: readonly string[]): void {
  queueMicrotask(() => {
    const tree = document.getElementById(id);
    if (!(tree instanceof HTMLElement)) return;
    customElements.upgrade(tree);
    const typed = tree as MjxTree;
    typed.nodes = nodes;
    typed.expanded = expanded;
  });
}

/**
 * The behaviour story. Small on purpose: this one is about what the keys do, and the scale story
 * below is about the DOM.
 */
export const AHeadingMovesWithItsSubtree: Story = {
  name: 'A Heading Moves With Its Subtree',
  render: () => {
    fill('behaviour-tree', smallOutline, smallOutlineExpanded);
    const report = (): void => {
      const tree = document.getElementById('behaviour-tree');
      if (!(tree instanceof HTMLElement)) return;
      const typed = tree as MjxTree;
      const readout = document.getElementById('tree-readout');
      if (readout === null) return;
      readout.textContent =
        `${String(typed.rows.length)} rows visible of ${String(typed.nodeCount)} nodes · ` +
        `${String(typed.builtRowCount)} in the DOM · last announcement: ${typed.announcement}`;
    };
    return stage(
      note(
        'Tab into the tree. Arrow Right opens a branch, and pressing it again moves into it; Arrow ' +
          'Left closes one, and on a closed branch or a leaf it moves to the parent. Alt with the ' +
          'vertical arrows reorders; Alt with the horizontal ones changes depth. Every reorder is ' +
          'announced, including a refusal.',
      ),
      html`
        <mjx-tree
          id="behaviour-tree"
          label="Navigation"
          style=${paneStyle}
          @mjx-navigator-reorder=${report}
          @mjx-navigator-select=${report}
        ></mjx-tree>
        <output id="tree-readout" class="mjx-type-dense" style="color:var(--theme-text-secondary)"
          >move something</output
        >
      `,
      caption([
        `Method carries ${String(subtreeIds(smallOutline, 'method').length - 1)} rows beneath it, and they move with it.`,
        'A leaf carries no aria-expanded at all — announcing false over a heading with nothing under it would offer a branch that does not exist.',
        'A refused move says why. Silence is a key that appears not to work.',
      ]),
    );
  },
};

/** ⚠ Five thousand nodes, so the node-count assertion is about something. */
export const FiveThousandHeadings: Story = {
  name: 'Five Thousand Headings',
  render: () => {
    const outline = largeOutline();
    // ⚠ Every branch, not just the chapters: the wrapping labels are on the leaves, and a tree
    // that never shows one has rows that are all exactly a line tall.
    fill('big-tree', outline, everyBranch(outline));
    return stage(
      note(
        `This outline holds ${String(largeNavigatorCount)} headings, three levels deep, with every branch ` +
          'open. The DOM holds a screenful. Every seventh heading is long enough to wrap ' +
          'onto a second line, which is what makes the rows different heights — a virtualiser that ' +
          'divided a scroll offset by a row height would put every row below one of them at the ' +
          'wrong index, and the symptom is a tree that scrolls to nearly the right heading.',
      ),
      html`<mjx-tree
        id="big-tree"
        label="Navigation"
        style=${paneStyle}
      ></mjx-tree>`,
      caption([
        'Type a letter: the cursor goes to the next heading beginning with it, wrapping once.',
        'Press the asterisk: every sibling of the current row opens, and nothing deeper.',
      ]),
    );
  },
};

/**
 * The tree with a context menu, which every one of these four has.
 *
 * The menu is U05's, unchanged. What is worth watching is that opening it does not disturb the
 * tree's cursor or its selection.
 */
export const WithAContextMenu: Story = {
  name: 'With A Context Menu',
  render: () => {
    fill('menu-tree', smallOutline, smallOutlineExpanded);
    return stage(
      note('Right-click a heading, or press the context-menu key.'),
      html`
        <mjx-context-menu style="flex:1 1 auto;display:block">
          <mjx-tree
            id="menu-tree"
            label="Navigation"
            style=${paneStyle}
          ></mjx-tree>
          <mjx-menu slot="menu" label="Heading">
            <mjx-menu-item label="Promote" shortcut="Alt+Left"></mjx-menu-item>
            <mjx-menu-item label="Demote" shortcut="Alt+Right"></mjx-menu-item>
            <mjx-menu-separator></mjx-menu-separator>
            <mjx-menu-item label="Move Up" shortcut="Alt+Up"></mjx-menu-item>
            <mjx-menu-item label="Move Down" shortcut="Alt+Down"></mjx-menu-item>
            <mjx-menu-separator></mjx-menu-separator>
            <mjx-menu-item label="Select Heading and Content"></mjx-menu-item>
          </mjx-menu>
        </mjx-context-menu>
      `,
      caption([
        'The menu is MJXOFF-184’s, with nothing added. Every one of these four navigators has one.',
      ]),
    );
  },
};

/**
 * ⚠ **The hit-target floor holds in compact density**, twisty included — and a twisty is the
 * smallest pressable thing in this child.
 */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    fill('compact-tree', smallOutline, smallOutlineExpanded);
    return html`<div data-density="compact" style="display:flex;flex-direction:column;min-block-size:0">
      ${stage(
        note('Compact density. Every row still clears the accessible target minimum.'),
        html`<mjx-tree id="compact-tree" label="Navigation" style=${paneStyle}></mjx-tree>`,
      )}
    </div>`;
  },
};
