import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  menuKeyboard,
  menuScreenReader,
  menuStatesFor,
  menuTokenDependencies,
  note,
} from './specimens.ts';

/**
 * `<mjx-context-menu>` — 302 published controls of its own, and the doorway to the **largest**
 * bucket in the whole inventory: *"None (Context Menu)"* is 1,155 controls in Word, 1,104 in Excel
 * and 1,660 in PowerPoint. More of Office is reached by right-clicking than through any ribbon tab.
 *
 * Right-click the canvas. Then focus it with Tab and press the `Context Menu` key, or `Shift + F10`
 * — a keyboard user's context is their focus, so it opens there rather than wherever the pointer
 * was left. Then narrow the container to the phone preset and do it again: below the width at which
 * the ribbon's tab strip becomes a picker, the menu is a **sheet**, which is a behavioural
 * difference rather than a skin.
 */

const conventions = storyConventions({
  statesMatrix: menuStatesFor(),
  tokenDependencies: menuTokenDependencies(),
  keyboard: menuKeyboard,
  screenReader: menuScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Menus/Context Menu',
  parameters: {
    docs: {
      description: {
        component:
          'A region that owns a menu, opened by right-click, by the Context Menu key or ' +
          'Shift + F10, and by a long press on touch. On a narrow container it is a sheet.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const canvasStyle =
  'display:grid;place-items:center;block-size:18rem;' +
  'background:var(--theme-surface);border:1px solid var(--theme-border-subtle);' +
  'border-radius:var(--radius-card);color:var(--theme-text-primary)';

const slideMenu = () => html`
  <mjx-menu slot="menu" label="Slide">
    <mjx-menu-item label="Cut" icon="delete" shortcut="Ctrl+X"></mjx-menu-item>
    <mjx-menu-item label="Copy" icon="save" shortcut="Ctrl+C"></mjx-menu-item>
    <mjx-menu-item
      label="Paste Special"
      unavailable
      explanation="The clipboard holds nothing that can be pasted specially."
    ></mjx-menu-item>
    <mjx-menu-separator></mjx-menu-separator>
    <mjx-menu-item label="Arrange">
      <mjx-menu slot="submenu" label="Arrange">
        <mjx-menu-item label="Bring To Front"></mjx-menu-item>
        <mjx-menu-item label="Send To Back"></mjx-menu-item>
      </mjx-menu>
    </mjx-menu-item>
    <mjx-menu-separator></mjx-menu-separator>
    <mjx-menu-section label="View">
      <mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>
      <mjx-menu-item kind="checkbox" label="Gridlines"></mjx-menu-item>
      <mjx-menu-item kind="checkbox" label="Guides"></mjx-menu-item>
    </mjx-menu-section>
  </mjx-menu>
`;

/** **The story the context-menu gates read.** */
export const OnACanvas: Story = {
  name: 'On A Canvas',
  render: () => html`
    ${note(
      'Right-click the canvas — the menu opens at the pointer. Tab to it and press the Context ' +
        'Menu key or Shift + F10 — it opens at the canvas instead, because that is what the ' +
        'keyboard was pointing at. Escape closes one level and gives focus back.',
    )}
    <div style="padding:calc(var(--mjx-density-gutter) * 2)">
      <mjx-context-menu id="canvas-context">
        <div id="context-canvas" tabindex="0" style=${canvasStyle}>
          Right-click here, or focus and press the Context Menu key.
        </div>
        ${slideMenu()}
      </mjx-context-menu>
    </div>
  `,
};

/**
 * The same component in a phone-width container.
 *
 * Nothing about the markup changes. The container query does, and the menu becomes a sheet pinned
 * to the bottom of the frame at full width — because a floating list aimed at with a thumb is a
 * list nobody hits.
 */
export const OnAPhone: Story = {
  name: 'On A Phone',
  render: () => html`
    ${note(
      'The container is pinned to 390px. Right-click, or long-press with a touch pointer, and the ' +
        'menu arrives as a sheet at the bottom of the frame rather than as a list beside the ' +
        'pointer. The same element, the same attributes, a different behaviour.',
    )}
    <mjx-resizable-container width="390">
      <div style="padding:calc(var(--mjx-density-gutter) * 2)">
        <mjx-context-menu id="phone-context">
          <div id="phone-canvas" tabindex="0" style=${canvasStyle}>
            Long-press, or right-click.
          </div>
          ${slideMenu()}
        </mjx-context-menu>
      </div>
    </mjx-resizable-container>
  `,
};
