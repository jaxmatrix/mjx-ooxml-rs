import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MiniCommand } from '../../src/feedback/feedback-model.ts';
import type { VirtualItem } from '../../src/navigators/virtual-list.ts';
import { workbookTabs } from '../navigators/specimens.ts';
import { multiReferenceFormula, workbookNames } from '../formula/specimens.ts';
import { documentThemePalette, machineFonts, recentColors, standardColors } from '../pickers/specimens.ts';
import { largeGalleryItems } from '../gallery/specimens.ts';
import { wordPhoneCommands } from '../mobile/specimens.ts';
import {
  shellKeyboard,
  shellScreenReader,
  shellStatesMatrix,
  shellTokenDependencies,
} from './shell-model.ts';
import {
  canvasArea,
  canvasRow,
  contextRegionStyle,
  documentColumn,
  documentPlaceholder,
  field,
  openDeclaredSurface,
  openSheetOnCommand,
  paneHeading,
  paneStack,
  phoneBody,
  phoneRails,
  selectionRun,
  shellFrame,
  statusBar,
  surface,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonNarrowFieldStyle,
  workspaceStyle,
  zoom,
} from './shell-parts.ts';
import { excelContextualSets, excelTabs } from '../ribbons/excel.ts';

/**
 * **Excel, assembled** — the ribbon, the name box and formula bar, the grid, a task pane, the sheet
 * tab bar and the status bar, at desktop, tablet and phone.
 *
 * Excel is the shell with the most **horizontal bands**: five of them stacked above and below the
 * grid, each one a component with its own idea of how much block space it deserves. That is the
 * composition question here — whether the grid still reads as the thing the window is for.
 *
 * What to look at:
 *
 * 1. **The bands.** Ribbon, formula row, grid, sheet tabs, status bar. Their heights were each
 *    settled in a story of their own and never against one another.
 * 2. **The formula bar.** Type `=SU` into it: the completion list opens over the grid. Move the
 *    caret through `${'=SUM(A1:A9)+B2-Sheet2!C3*$A$1'}` and watch the references take their colours —
 *    which is the first time those four colours are seen against a ribbon rather than on a page of
 *    their own.
 * 3. **The sheet tab bar at tablet.** Ten sheets in 834 pixels: the strip overflows and its four
 *    scroll affordances appear, beside a status bar that is demoting readings at the same time.
 *
 * **Nothing here dispatches a command**, and no cell holds a value: the grid is an honestly labelled
 * placeholder, because the renderer is Rust and is not wired into Storybook.
 */

const conventions = storyConventions({
  statesMatrix: shellStatesMatrix,
  tokenDependencies: shellTokenDependencies,
  keyboard: shellKeyboard,
  screenReader: shellScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Shell/Excel',
  parameters: {
    docs: {
      description: {
        component:
          'The whole application assembled from the catalogue’s own components: ribbon, name box ' +
          'and formula bar, grid, task pane, sheet tab bar and status bar on a desktop; a formula ' +
          'bar and a command rail on a phone. Cosmetic only — every command is inert.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

// ── the fixtures ─────────────────────────────────────────────────────────────

/** The workbook's defined names, as the Name Manager list holds them. */
const definedNameRows: readonly VirtualItem[] = workbookNames.map((entry) => ({
  id: entry.name,
  label: entry.name,
  detail: entry.definition,
}));

/** What the mini toolbar offers over a selected range. */
const cellCommands: readonly MiniCommand[] = [
  { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle' },
  { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
  { command: 'align-center', label: 'Centre', icon: 'text-align-center', separatorBefore: true },
  { command: 'comment', label: 'New note', icon: 'comment', separatorBefore: true },
  {
    command: 'delete',
    label: 'Delete',
    icon: 'delete',
    unavailable: true,
    explanation: 'The selection includes a cell in a protected range.',
  },
];

// ── the ribbon ──────────────────────────────────────────────────────────

/**
 * **Excel's ribbon, from `stories/ribbons/excel.ts`** — the same functions `Ribbons/Excel` audits.
 *
 * The tabs used to be written here; moving them out is the ribbon programme's unit 0. What stays is
 * what belongs to an *application*: this machine's font list, this workbook's palette, the number
 * formats this locale offers, the id of the menu the paste button opens, the cell-style gallery's
 * contents. They are bound by the stable command ids `dev/ribbons/census.ts` declares.
 *
 * `excelTabs()` leaves out Print Preview and Background Removal, which Office shows only inside the
 * view they name.
 */
function ribbon(): TemplateResult {
  return surface(
    'ribbon',
    'flex:0 0 auto;min-inline-size:0',
    html`
      <mjx-ribbon label="Excel" selected="home" @mjx-activate=${openDeclaredSurface}>
        ${excelTabs({
          controls: {
            'excel.home.clipboard.paste': html`<mjx-split-button
              slot="essential"
              label="Paste"
              icon="clipboard-paste"
              size="large"
              menu-label="Paste options"
              data-opens="xl-paste-menu"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.home.font.name': html`<mjx-font-picker
              id="xl-font"
              style=${ribbonFieldStyle}
              label="Font"
              value="Aptos"
              .fonts=${machineFonts}
            ></mjx-font-picker>`,
            'excel.home.font.size': html`<mjx-dropdown
              id="xl-size"
              label="Font size"
              value="11"
              style=${ribbonNarrowFieldStyle}
            >
              ${['9', '10', '11', '12', '14', '18'].map(
                (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'excel.home.font.fill': html`<mjx-color-picker
              id="xl-fill"
              style=${ribbonColourFieldStyle}
              label="Fill colour"
              show-no-fill
              .themePalette=${documentThemePalette}
              .standardColors=${standardColors}
              .recentColors=${recentColors}
            ></mjx-color-picker>`,
            'excel.home.number.format': html`<mjx-dropdown
              id="xl-number"
              label="Number format"
              value="general"
              style=${ribbonColourFieldStyle}
            >
              ${[
                { value: 'general', label: 'General' },
                { value: 'number', label: 'Number' },
                { value: 'currency', label: 'Currency' },
                { value: 'accounting', label: 'Accounting' },
                { value: 'percentage', label: 'Percentage' },
                { value: 'date', label: 'Short Date' },
              ].map(
                (format) =>
                  html`<mjx-option value=${format.value} label=${format.label}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'excel.home.styles.gallery': html`<mjx-gallery
              id="xl-cell-styles"
              label="Cell styles"
              value="office-2"
              style=${ribbonGalleryStyle}
            >
              ${largeGalleryItems().slice(0, 18)}
            </mjx-gallery>`,
          },
        })}
        ${excelContextualSets()}
      </mjx-ribbon>
    `,
  );
}

// ── the bands ────────────────────────────────────────────────────────────────

/**
 * The name box and the formula bar — **one band, and the name box goes inside the bar.**
 *
 * ⚠ This is a *check before you consume* finding, and it was got wrong first. `<mjx-formula-bar>`
 * builds a `name-box` **slot** of its own, before its divider and its three affordances, because
 * Excel's name box has always been part of the same band. An assembly that set the two side by side
 * — which is what this shell did until it was measured — produced a bar with an empty 96 px slot in
 * it and a name box beside it, at 101 px tall instead of 58. Nothing failed; it just looked wrong in
 * a way nobody would have attributed to the markup.
 */
function formulaRow(id: string, box: string, value: string): TemplateResult {
  return surface(
    'formula-row',
    'flex:0 0 auto;min-inline-size:0;padding:var(--mjx-density-step) var(--mjx-density-gutter)',
    html`
      <mjx-formula-bar id=${id} label="Formula bar" value=${value} style="min-inline-size:0">
        <mjx-name-box
          id=${box}
          slot="name-box"
          label="Name box"
          address="B2"
          .names=${workbookNames}
        ></mjx-name-box>
      </mjx-formula-bar>
    `,
  );
}

/** The grid, its context menu, the selection a mini toolbar hangs off, and the scrollbar. */
function gridArea(): TemplateResult {
  return canvasRow(
    canvasArea(
      'xl-grid',
      html`
      <mjx-context-menu id="xl-context" style=${contextRegionStyle}>
        ${documentPlaceholder(
          'Summary — B2:D9 selected',
          html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
            ${selectionRun('xl-selection', 'B2:D9')} is what the mini toolbar above is about.
          </p>`,
        )}
        <mjx-menu slot="menu" label="Cells" floating>
          <mjx-menu-item label="Cut" icon="cut" shortcut="Ctrl+X"></mjx-menu-item>
          <mjx-menu-item label="Copy" icon="copy" shortcut="Ctrl+C"></mjx-menu-item>
          <mjx-menu-item label="Paste Options">
            <mjx-menu slot="submenu" label="Paste Options">
              <mjx-menu-item kind="radio" label="Values" checked></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Formulas"></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Formatting"></mjx-menu-item>
            </mjx-menu>
          </mjx-menu-item>
          <mjx-menu-separator></mjx-menu-separator>
          <mjx-menu-section label="Cells">
            <mjx-menu-item label="Insert…" icon="add"></mjx-menu-item>
            <mjx-menu-item label="Delete…" icon="delete"></mjx-menu-item>
            <mjx-menu-item
              label="Clear Contents"
              unavailable
              explanation="The selection includes a cell in a protected range."
            ></mjx-menu-item>
          </mjx-menu-section>
        </mjx-menu>
      </mjx-context-menu>
    `,
      html`<mjx-mini-toolbar
        id="xl-mini"
        label="Formatting"
        for="xl-selection"
        open
        .commands=${cellCommands}
      ></mjx-mini-toolbar>`,
    ),
    html`<mjx-scrollbar
      id="xl-scroll"
      label="Worksheet"
      controls="xl-grid"
      pages="30"
      page-height="700"
      viewport="620"
    ></mjx-scrollbar>`,
  );
}

/** The Queries pane: a real empty state, which is what a new workbook actually shows. */
function taskPane(fraction: string): TemplateResult {
  return html`
    <mjx-task-pane
      id="xl-pane"
      label="Queries &amp; Connections"
      open
      dock="inlineEnd"
      fraction=${fraction}
    >
      ${paneStack(
        html`<mjx-empty-state
          id="xl-queries-empty"
          heading="No queries yet"
          description="Queries you add from the Data tab appear here, with when each one last refreshed."
          icon="table"
          action-label="Get data"
          action-command="data.getData"
        ></mjx-empty-state>`,
        paneHeading('Defined names'),
        html`<mjx-virtual-list
          id="xl-names"
          label="Defined names in this workbook"
          value="Revenue"
          style="block-size:11rem;border:1px solid var(--theme-border-subtle);
                 border-radius:var(--radius-control)"
          .items=${definedNameRows}
        ></mjx-virtual-list>`,
        field(
          'xl-width',
          'Row height',
          html`<mjx-measure-input
            id="xl-width"
            value="15"
            unit="pt"
            step="0.75"
            min="0"
          ></mjx-measure-input>`,
        ),
        html`<mjx-checkbox id="xl-gridlines" label="Show gridlines" checked="true"></mjx-checkbox>`,
      )}
    </mjx-task-pane>
  `;
}

/** The sheet tab bar, with ten sheets, two colours and one hidden. */
function sheetTabs(): TemplateResult {
  return surface(
    'sheet-tabs',
    'flex:0 0 auto;min-inline-size:0;padding:0 var(--mjx-density-gutter)',
    html`<mjx-sheet-tab-bar
      id="xl-sheets"
      label="Sheets"
      value="summary"
      .tabs=${workbookTabs()}
    ></mjx-sheet-tab-bar>`,
  );
}

/** The bar across the foot, and the readings a workbook actually carries. */
function foot(): TemplateResult {
  return statusBar(
    'xl-status',
    'Workbook status',
    [
      { id: 'xl-mode', label: 'Mode', value: 'Ready', priority: 'essential' },
      { id: 'xl-average', label: 'Average', value: '18,204', priority: 'standard' },
      { id: 'xl-count', label: 'Count', value: '24', priority: 'standard' },
      { id: 'xl-sum', label: 'Sum', value: '436,896', priority: 'supplementary' },
      {
        id: 'xl-calc',
        label: 'Calculation',
        value: 'Automatic',
        priority: 'ancillary',
        region: 'centre',
      },
    ],
    zoom('xl-zoom', { width: 1100, height: 850 }),
  );
}

// ── the desktop and tablet shells ────────────────────────────────────────────

/**
 * Everything above the phone: five stacked bands and a pane.
 *
 * The pane's share grows at tablet for the reason PowerPoint's does — a docked `<mjx-task-pane>`
 * will not lay out below about 280 px, so a fraction sized for a desktop overflows an 834 px
 * workspace by 96. See `powerpoint.stories.ts` for the decision and `ui/README.md` for the question
 * it leaves open.
 */
function wideShell(size: 'desktop' | 'tablet'): TemplateResult {
  const paneFraction = size === 'desktop' ? '0.22' : '0.4';
  return shellFrame(
    'excel',
    size,
    ribbon(),
    formulaRow('xl-formula', 'xl-name-box', multiReferenceFormula),
    surface(
      'workspace',
      workspaceStyle,
      html`${documentColumn(gridArea())} ${taskPane(paneFraction)}`,
    ),
    sheetTabs(),
    foot(),
    html`
      <mjx-menu id="xl-paste-menu" label="Paste options" floating>
        <mjx-menu-section label="Paste">
          <mjx-menu-item kind="radio" label="All" checked></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Values"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Formats"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Transpose"></mjx-menu-item>
        </mjx-menu-section>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
      </mjx-menu>
      <mjx-dialog id="xl-format-cells" label="Format Cells" modal>
        ${paneStack(
          // A measure input is for a *measure*, so the field here is an indent rather than a count
          // of decimal places: `value` is in points and a spin box over 0…30 places would be a
          // different component. Naming that is cheaper than misusing this one.
          field(
            'xl-indent',
            'Indent',
            html`<mjx-measure-input
              id="xl-indent"
              value="0"
              unit="pt"
              step="3"
              min="0"
              max="216"
            ></mjx-measure-input>`,
          ),
          html`<mjx-checkbox
            id="xl-thousands"
            label="Use 1000 separator"
            checked="true"
          ></mjx-checkbox>`,
          html`<mjx-popover id="xl-negatives" label="Negative numbers" kind="flyout">
            <mjx-button slot="anchor" label="Negative numbers…" icon="settings"></mjx-button>
            <mjx-checkbox id="xl-negative-red" label="Show in red"></mjx-checkbox>
            <mjx-checkbox id="xl-negative-brackets" label="Show in brackets"></mjx-checkbox>
          </mjx-popover>`,
        )}
      </mjx-dialog>
    `,
  );
}

// ── the stories ──────────────────────────────────────────────────────────────

/** The whole application at 1440: five bands and a pane around one grid. */
export const Desktop: Story = {
  globals: { containerPreset: 'desktop' },
  render: () => wideShell('desktop'),
};

/**
 * **The middle size.** The sheet tab strip overflows at 834 and grows its four scroll affordances
 * while the status bar is demoting readings — two components deciding to change shape at once, which
 * is a thing only an assembly can show.
 */
export const Tablet: Story = {
  globals: { containerPreset: 'tablet' },
  render: () => wideShell('tablet'),
};

/**
 * **The phone**, and Excel is the one application whose phone shell keeps a desktop band: the
 * formula bar. A spreadsheet without one is not a spreadsheet, so the rail below and the bar above
 * have to share a screen 390 pixels wide.
 */
export const Mobile: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    shellFrame(
      'excel',
      'mobile',
      formulaRow('xl-phone-formula', 'xl-phone-name-box', '=SUM(B2:B9)'),
      phoneBody(canvasArea('xl-phone-grid', documentPlaceholder('Summary — B2:D9 selected'))),
      surface(
        'sheet-tabs',
        'flex:0 0 auto;min-inline-size:0;padding:0 var(--mjx-density-step)',
        html`<mjx-sheet-tab-bar
          id="xl-phone-sheets"
          label="Sheets"
          value="summary"
          .tabs=${workbookTabs()}
        ></mjx-sheet-tab-bar>`,
      ),
      phoneRails(
        html`<mjx-contextual-action-bar
          id="xl-phone-selection"
          selection="cells"
          selection-label="B2:D9"
        ></mjx-contextual-action-bar>`,
        html`<mjx-command-bar
          id="xl-phone-commands"
          label="Home"
          .commands=${wordPhoneCommands}
          @mjx-mobile-command=${openSheetOnCommand('styles', 'xl-phone-sheet')}
        ></mjx-command-bar>`,
      ),
      html`<mjx-dialog id="xl-phone-sheet" label="Cell styles" modal detent="half">
        <mjx-gallery id="xl-phone-styles" label="Cell styles" value="office-2">
          ${largeGalleryItems().slice(0, 12)}
        </mjx-gallery>
      </mjx-dialog>`,
    ),
};
