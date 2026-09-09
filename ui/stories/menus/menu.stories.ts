import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  menuKeyboard,
  menuScreenReader,
  menuStatesFor,
  menuStatesMatrix,
  menuTokenDependencies,
  note,
} from './specimens.ts';
import type { MjxMenu } from '../../src/menus/menu.ts';

/**
 * `<mjx-menu>`, `<mjx-menu-item>`, `<mjx-menu-separator>` and `<mjx-menu-section>` — 649 of
 * Office's published controls, and the shape the largest surface in the product takes.
 *
 * **Open *Submenus And Diagonal Travel* first, and use a pointer.** Move from *Paste Options* down
 * and to the right, across *Paste Special*, into the submenu. The submenu stays open, because the
 * pointer is inside the triangle between where it left and the submenu's near edge. Then do it
 * again slowly, straight down: the submenu closes, because that time you meant the other item.
 * Neither of those is visible in a screenshot, which is why the gates for this component are
 * interaction tests rather than snapshots.
 */

const conventions = storyConventions({
  statesMatrix: menuStatesFor(),
  tokenDependencies: menuTokenDependencies(),
  keyboard: menuKeyboard,
  screenReader: menuScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal, and it must stay one — Storybook indexes CSF statically and refuses a
  // computed title. `tests/browser/menus.spec.ts` looks every story up by this string.
  title: 'Menus/Menu',
  parameters: {
    docs: {
      description: {
        component:
          'The ARIA menu pattern with the parts that are usually missing: hover intent with a ' +
          'safe triangle, edge flipping in all four directions and under RTL, a sheet on a phone, ' +
          'and focus that goes back to the invoker on every way of closing that should return it.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const stage = (height: string) =>
  `position:relative;block-size:${height};padding:calc(var(--mjx-density-gutter) * 2);` +
  'border:1px solid var(--theme-border-subtle);border-radius:var(--radius-card);' +
  'margin:calc(var(--mjx-density-gutter) * 2)';

/** Open a menu from the button that asked for it. The wiring a shell does, done once here. */
const openFromInvoker = (event: Event): void => {
  const invoker = event.currentTarget;
  if (!(invoker instanceof HTMLElement)) return;
  const menu = invoker.parentElement?.querySelector('mjx-menu');
  if (menu === null || menu === undefined) return;
  (menu as MjxMenu).openFrom(invoker);
};

/**
 * **The story every gate that measures paint opens.**
 *
 * Seven inline menus, one row each, one state each. Inline rather than floating because a floating
 * menu is `display: none` until it is opened, and a states matrix nobody can see is a states matrix
 * nobody audits.
 */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => html`
    ${note(
      'Seven states, and the two that a pointer produces are forced so a static page can show ' +
        'them. `focus` is never forced: press Tab and watch the ring arrive through the browser’s ' +
        'own judgement about how focus got there.',
    )}
    ${menuStatesMatrix()}
  `,
};

/** Everything a row can carry, in one menu. */
export const TheItemAnatomy: Story = {
  name: 'The Item Anatomy',
  render: () => html`
    ${note(
      'An icon, a label, a check or radio mark, a shortcut hint, a second line, a submenu arrow — ' +
        'and the two ways of dividing a menu. The mark gutter is reserved once for the whole menu, ' +
        'so the labels line up whether or not the row above is checkable.',
    )}
    <div style=${stage('auto')}>
      <mjx-menu label="Edit">
        <mjx-menu-item label="Cut" icon="delete" shortcut="Ctrl+X"></mjx-menu-item>
        <mjx-menu-item label="Copy" icon="save" shortcut="Ctrl+C"></mjx-menu-item>
        <mjx-menu-item
          label="Paste Special"
          icon="folder-open"
          unavailable
          explanation="The clipboard holds nothing that can be pasted specially."
        ></mjx-menu-item>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-section label="Paste Options">
          <mjx-menu-item
            kind="radio"
            label="Keep Source Formatting"
            value="source"
            checked
            description="Keeps the fonts and colours the text was copied with."
          ></mjx-menu-item>
          <mjx-menu-item
            kind="radio"
            label="Merge Formatting"
            value="merge"
            description="Takes the formatting of the paragraph it lands in."
          ></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Keep Text Only" value="text"></mjx-menu-item>
        </mjx-menu-section>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>
        <mjx-menu-item kind="checkbox" label="Gridlines"></mjx-menu-item>
        <mjx-menu-item label="More Options">
          <mjx-menu slot="submenu" label="More Options">
            <mjx-menu-item label="Navigation Pane"></mjx-menu-item>
            <mjx-menu-item label="Document Map"></mjx-menu-item>
          </mjx-menu>
        </mjx-menu-item>
      </mjx-menu>
    </div>
  `,
};

/**
 * **The story the diagonal-travel gate reads.**
 *
 * The second item opens a submenu. A pointer moving from it toward the submenu crosses the third
 * item on the way, and the third item is deliberately there.
 */
export const SubmenusAndDiagonalTravel: Story = {
  name: 'Submenus And Diagonal Travel',
  render: () => html`
    ${note(
      'Hover “Paste Options”, wait for its submenu, then move diagonally into the submenu across ' +
        '“Paste Special”. The submenu stays. Move straight down onto “Paste Special” instead and ' +
        'it closes. That difference is the whole feature, and it is invisible in a picture.',
    )}
    <div style=${stage('auto')}>
      <mjx-menu label="Paste" id="travel-menu">
        <mjx-menu-item label="Paste" icon="folder-open" shortcut="Ctrl+V"></mjx-menu-item>
        <mjx-menu-item label="Paste Options" icon="settings">
          <mjx-menu slot="submenu" label="Paste Options">
            <mjx-menu-item kind="radio" label="Keep Source Formatting" checked></mjx-menu-item>
            <mjx-menu-item kind="radio" label="Merge Formatting"></mjx-menu-item>
            <mjx-menu-item kind="radio" label="Keep Text Only"></mjx-menu-item>
            <mjx-menu-item label="Set Default Paste">
              <mjx-menu slot="submenu" label="Set Default Paste">
                <mjx-menu-item label="Within The Same Document"></mjx-menu-item>
                <mjx-menu-item label="Between Documents"></mjx-menu-item>
              </mjx-menu>
            </mjx-menu-item>
          </mjx-menu>
        </mjx-menu-item>
        <mjx-menu-item label="Paste Special" icon="add" shortcut="Ctrl+Alt+V"></mjx-menu-item>
        <mjx-menu-item label="Paste As Hyperlink"></mjx-menu-item>
      </mjx-menu>
    </div>
  `,
};

/**
 * **The story the placement gate reads.**
 *
 * One anchor the test moves to each corner of the frame, and one menu. What is being asserted is
 * not that the menu *moved* but that it went where `placeFloating()` says — flipping when the
 * preferred side has no room, shifting when the cross axis runs out, and never leaving the
 * rectangle that clips it.
 */
export const PositioningAndFlipping: Story = {
  name: 'Positioning And Flipping',
  render: () => html`
    ${note(
      'Press the button in each corner. The menu flips to the other side when there is no room, ' +
        'and shifts along the edge rather than hanging out of the frame. The frame is what clips ' +
        'it — not the window — because the harness frame is a container query container and ' +
        'therefore a containing block for anything fixed inside it.',
    )}
    <div style=${stage('26rem')} id="placement-stage">
      <div id="placement-anchor" style="position:absolute;inset-block-start:0;inset-inline-start:0">
        <button
          type="button"
          class="control mjx-type-control mjx-hit-target"
          id="placement-invoker"
          style="border:1px solid var(--theme-border);border-radius:var(--radius-control);background:var(--theme-surface);color:var(--theme-text-primary);padding-inline:var(--mjx-density-gutter);cursor:pointer"
          aria-haspopup="menu"
          @click=${openFromInvoker}
        >
          Open menu
        </button>
        <mjx-menu label="Placement" floating>
          <mjx-menu-item label="Bring To Front"></mjx-menu-item>
          <mjx-menu-item label="Send To Back"></mjx-menu-item>
          <mjx-menu-item label="Bring Forward"></mjx-menu-item>
          <mjx-menu-item label="Send Backward"></mjx-menu-item>
          <mjx-menu-item label="Align To Page"></mjx-menu-item>
        </mjx-menu>
      </div>
    </div>
  `,
};

/** The same placement, with the line running the other way. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () => html`
    ${note(
      'Exactly the same component with `dir="rtl"`. A submenu opens toward the *end* of the line, ' +
        'which is leftward here, and Arrow Left is the key that opens it. One function decides ' +
        'both — physicalSide() — rather than a second code path.',
    )}
    <div dir="rtl" style=${stage('22rem')} id="rtl-stage">
      <div id="rtl-anchor" style="position:absolute;inset-block-start:0;inset-inline-start:0">
        <button
          type="button"
          class="mjx-type-control mjx-hit-target"
          id="rtl-invoker"
          style="border:1px solid var(--theme-border);border-radius:var(--radius-control);background:var(--theme-surface);color:var(--theme-text-primary);padding-inline:var(--mjx-density-gutter);cursor:pointer"
          aria-haspopup="menu"
          @click=${openFromInvoker}
        >
          افتح القائمة
        </button>
        <mjx-menu label="محاذاة" floating>
          <mjx-menu-item label="إحضار إلى المقدمة"></mjx-menu-item>
          <mjx-menu-item label="إرسال إلى الخلف">
            <mjx-menu slot="submenu" label="إرسال إلى الخلف">
              <mjx-menu-item label="خطوة واحدة"></mjx-menu-item>
              <mjx-menu-item label="إلى النهاية"></mjx-menu-item>
            </mjx-menu>
          </mjx-menu-item>
          <mjx-menu-item label="محاذاة إلى الصفحة"></mjx-menu-item>
        </mjx-menu>
      </div>
    </div>
  `,
};

/** A menu inside a compact surface: the hit-target floor is where a dense chrome loses it. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => html`
    ${note(
      'The same menu at both densities. Compact takes the air out of the rows and stops at the ' +
        'floor: WCAG 2.2 Target Size (Minimum) is 24 CSS pixels and no density mode may go under it.',
    )}
    <div style="display:flex;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)">
      <mjx-menu label="Comfortable">
        <mjx-menu-item label="Cut" shortcut="Ctrl+X"></mjx-menu-item>
        <mjx-menu-item label="Copy" shortcut="Ctrl+C"></mjx-menu-item>
        <mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>
      </mjx-menu>
      <div data-density="compact">
        <mjx-menu label="Compact">
          <mjx-menu-item label="Cut" shortcut="Ctrl+X"></mjx-menu-item>
          <mjx-menu-item label="Copy" shortcut="Ctrl+C"></mjx-menu-item>
          <mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>
        </mjx-menu>
      </div>
    </div>
  `,
};

/** MJXOFF-182's split button, doing the thing its arrow exists to do. */
export const OpenedByASplitButton: Story = {
  name: 'Opened By A Split Button',
  render: () => html`
    ${note(
      'The arrow of a split button requests a menu and never activates the command — that is ' +
        'MJXOFF-182’s guarantee. This is the other half of it: a shell listens for the request and ' +
        'opens a menu anchored to the button. Press Arrow Down on either half.',
    )}
    <div
      style=${stage('18rem')}
      id="split-stage"
      @mjx-menu-request=${(event: Event): void => {
        const host = event.target;
        if (!(host instanceof HTMLElement)) return;
        const menu = host.parentElement?.querySelector('mjx-menu');
        if (menu === null || menu === undefined) return;
        host.setAttribute('expanded', '');
        (menu as MjxMenu).openFrom(host);
      }}
      @mjx-menu-close=${(event: Event): void => {
        const host = event.currentTarget;
        if (!(host instanceof HTMLElement)) return;
        host.querySelector('mjx-split-button')?.removeAttribute('expanded');
      }}
    >
      <mjx-split-button
        label="Undo"
        icon="arrow-undo"
        menu-label="Undo history"
      ></mjx-split-button>
      <mjx-menu label="Undo history" floating align="start">
        <mjx-menu-item label="Typing “fidelity”"></mjx-menu-item>
        <mjx-menu-item label="Paste"></mjx-menu-item>
        <mjx-menu-item label="Insert Table"></mjx-menu-item>
      </mjx-menu>
    </div>
  `,
};
