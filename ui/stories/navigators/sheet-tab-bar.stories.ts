import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MjxSheetTabBar } from '../../src/navigators/index.ts';
import {
  sheetNameForbidden,
  sheetNameMaximum,
  type SheetTab,
} from '../../src/navigators/navigator-model.ts';
import {
  caption,
  navigatorTokenDependencies,
  note,
  stage,
  tabBarKeyboard,
  tabBarScreenReader,
  tabBarStates,
  workbookTabs,
} from './specimens.ts';

/**
 * `<mjx-sheet-tab-bar>` — Excel's sheet tabs.
 *
 * **Open *Rename In Place, And Escape Cancels* first, with a keyboard.** Press `F2`, type
 * something, and press `Escape`. The old name is still there. Then do it again and press `Enter`.
 * Those two are the whole component: a rename field that commits on Escape has destroyed a name with
 * the key people press to mean *stop*, and afterwards the two look identical.
 *
 * `GUESS:` **a sheet's colour is drawn as an edge and never as a fill.** Excel paints the whole
 * inactive tab. This catalogue cannot, because the colour belongs to the user's workbook and may be
 * anything at all — a label drawn on it would have a contrast nobody has checked. As a bar along the
 * tab's own edge the user's choice stays visible and the label stays on a surface whose contrast is
 * the palette's and is measured.
 *
 * `GUESS:` **hidden sheets are shown, marked and named.** Excel omits them from the strip entirely
 * and hides them behind a dialog. Showing them is the choice that cannot lose somebody's work, and
 * the state is carried in the accessible name as well as in the drawing.
 */

const conventions = storyConventions({
  statesMatrix: tabBarStates,
  tokenDependencies: navigatorTokenDependencies,
  keyboard: tabBarKeyboard,
  screenReader: tabBarScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Navigators/Sheet Tab Bar',
  parameters: {
    docs: {
      description: {
        component:
          'Excel’s sheet tabs: scroll affordances when they overflow, a per-sheet colour, rename ' +
          'in place with Escape cancelling, drag and keyboard reorder, hidden-sheet indication and ' +
          'the new-sheet control.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Fill a bar once it has upgraded. */
function fill(id: string, tabs: readonly SheetTab[], value: string): void {
  queueMicrotask(() => {
    const bar = document.getElementById(id);
    if (!(bar instanceof HTMLElement)) return;
    customElements.upgrade(bar);
    const typed = bar as MjxSheetTabBar;
    typed.tabs = tabs;
    typed.value = value;
  });
}

/** A canvas above the bar, so it sits where a workbook's tabs sit. */
function workspace(bar: unknown) {
  return html`
    <div
      style="flex:1 1 auto;min-block-size:0;background:var(--theme-surface);
             border:1px solid var(--theme-border);border-end-start-radius:0;border-end-end-radius:0;
             border-start-start-radius:var(--radius-card);border-start-end-radius:var(--radius-card)"
    ></div>
    ${bar}
  `;
}

export const RenameInPlaceAndEscapeCancels: Story = {
  name: 'Rename In Place, And Escape Cancels',
  render: () => {
    fill('rename-bar', workbookTabs(), 'summary');
    const report = (event: Event): void => {
      const readout = document.getElementById('rename-readout');
      if (readout === null) return;
      const detail = (event as CustomEvent<{ label?: string }>).detail;
      readout.textContent = `renamed to ${detail.label ?? ''}`;
    };
    return stage(
      note(
        'Double-click a tab, or focus one and press F2. Enter commits; Escape cancels and the old ' +
          `name is kept. A name Excel would refuse — empty, over ${String(sheetNameMaximum)} ` +
          `characters, containing one of ${sheetNameForbidden.join(' ')}, or one another sheet ` +
          'already has — keeps the text you typed, says what is wrong, and commits nothing.',
      ),
      workspace(html`
        <mjx-sheet-tab-bar
          id="rename-bar"
          label="Sheets"
          @mjx-navigator-rename=${report}
        ></mjx-sheet-tab-bar>
      `),
      html`<output id="rename-readout" class="mjx-type-dense" style="color:var(--theme-text-secondary)"
        >nothing renamed yet</output
      >`,
      caption([
        'Alt with the horizontal arrows moves a sheet one place, and dragging a tab does the same thing through the same function.',
        'The four scroll affordances are buttons and not tabs — a tab list may only own tabs, and pressing one changes nothing about which sheet is showing.',
      ]),
    );
  },
};

/**
 * Narrow the container until the strip overflows.
 *
 * The four affordances are always present rather than appearing at a breakpoint: a control that
 * comes and goes as a window is resized is a control people cannot learn the position of, and this
 * strip's own scroll position is not something CSS can report to JavaScript without measuring.
 */
export const WhenTheTabsOverflow: Story = {
  name: 'When The Tabs Overflow',
  render: () => {
    const many: SheetTab[] = [
      ...workbookTabs(),
      ...Array.from({ length: 22 }, (_unused, index) => ({
        id: `extra-${String(index)}`,
        label: `Region ${String(index + 1)} Detail`,
      })),
    ];
    fill('overflow-bar', many, 'summary');
    return stage(
      note(
        'Thirty-two sheets in a strip that cannot show them. Use the container control in the ' +
          'toolbar to narrow the frame — never the browser window — and press the affordances or ' +
          'arrow through the tabs. Focusing a tab at the far end brings it into view without ' +
          'scrolling the page around it.',
      ),
      workspace(html`<mjx-sheet-tab-bar id="overflow-bar" label="Sheets"></mjx-sheet-tab-bar>`),
      caption([
        'Two sheets carry a colour, drawn as a bar along the tab’s edge.',
        'Workings is hidden: a dashed edge, a glyph, and “hidden” in its accessible name — three cues, all at full contrast.',
      ]),
    );
  },
};

/** The bar with a context menu, which every one of these four has. */
export const WithAContextMenu: Story = {
  name: 'With A Context Menu',
  render: () => {
    fill('menu-bar', workbookTabs(), 'q1');
    return stage(
      note('Right-click a tab.'),
      workspace(html`
        <mjx-context-menu style="display:block">
          <mjx-sheet-tab-bar id="menu-bar" label="Sheets"></mjx-sheet-tab-bar>
          <mjx-menu slot="menu" label="Sheet">
            <mjx-menu-item label="Insert…"></mjx-menu-item>
            <mjx-menu-item label="Delete"></mjx-menu-item>
            <mjx-menu-item label="Rename" shortcut="F2"></mjx-menu-item>
            <mjx-menu-separator></mjx-menu-separator>
            <mjx-menu-item label="Move or Copy…"></mjx-menu-item>
            <mjx-menu-item label="Hide"></mjx-menu-item>
          </mjx-menu>
        </mjx-context-menu>
      `),
      caption(['The menu is MJXOFF-184’s, with nothing added.']),
    );
  },
};

/**
 * ⚠ **The hit-target floor holds in compact density** — and here it is the four affordance buttons
 * and the new-sheet control that are at risk, because they are square and have no label to widen
 * them.
 */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    fill('compact-bar', workbookTabs(), 'summary');
    return html`<div data-density="compact">
      ${stage(
        note('Compact density. The tabs and all five buttons still clear the accessible minimum.'),
        workspace(html`<mjx-sheet-tab-bar id="compact-bar" label="Sheets"></mjx-sheet-tab-bar>`),
      )}
    </div>`;
  },
};
