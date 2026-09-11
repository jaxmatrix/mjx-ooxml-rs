import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MiniCommand } from '../../src/feedback/feedback-model.ts';
import type { RailSlide } from '../../src/navigators/navigator-model.ts';
import type { VirtualItem } from '../../src/navigators/virtual-list.ts';
import { everyPlate, largeDeck } from '../navigators/specimens.ts';
import { alignmentSegments, lineSpacingOptions } from '../inputs/specimens.ts';
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
  navigatorPane,
  openDeclaredSurface,
  openSheetOnCommand,
  paneHeading,
  paneSplitter,
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
import { powerpointContextualSets, powerpointTabs } from '../ribbons/powerpoint.ts';

/**
 * **PowerPoint, assembled** — the ribbon, the thumbnail rail, the slide surface, a task pane and
 * the status bar, at desktop, tablet and phone.
 *
 * Fifteen children audited these components one at a time, and every one of them can be right while
 * the composition is wrong. What to look at, in this order:
 *
 * 1. **The desktop at rest.** Spacing that reads generous around one button, seen across five
 *    ribbon groups and a status bar. Whether the rail, the slide and the pane divide the width in
 *    proportions that read as deliberate.
 * 2. **The tablet.** The ribbon is neither full nor collapsed and the task pane is still docked —
 *    which is the decision this assembly makes, and the one MJXOFF-195 is most likely to overturn.
 * 3. **The phone.** No ribbon at all: the command bar *is* the ribbon, and the contextual action
 *    bar replaces it the moment something is selected.
 *
 * **Nothing here dispatches a command.** Menus open, galleries preview, the pane resizes, the
 * dialog appears — and no document changes, because command dispatch, document binding, the
 * `ShellBridge` and the real Rust canvas are all loop 2. The placeholder in the middle of the
 * slide says so, on its face, for exactly that reason.
 */

const conventions = storyConventions({
  statesMatrix: shellStatesMatrix,
  tokenDependencies: shellTokenDependencies,
  keyboard: shellKeyboard,
  screenReader: shellScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  // `stories/shell/shell-model.ts` derives the same string and the browser gate looks every story
  // up by the id it produces, so a divergence fails with a name rather than skipping silently.
  title: 'Shell/PowerPoint',
  parameters: {
    docs: {
      description: {
        component:
          'The whole application assembled from the catalogue’s own components: ribbon, thumbnail ' +
          'rail, slide surface, task pane and status bar on a desktop; a command rail and a sheet ' +
          'on a phone. Cosmetic only — every command is inert.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

// ── the fixtures ─────────────────────────────────────────────────────────────

/**
 * A forty-slide deck, four in five of which have a plate.
 *
 * The gaps are deliberate: R10's plate generator is asynchronous, so *waiting for a picture* is the
 * state a rail is in most of the time it is being built, and an assembly whose rail was uniformly
 * complete would never show a reviewer what that looks like beside real chrome.
 */
function deck(): RailSlide[] {
  const plates = everyPlate(8);
  return largeDeck(40).map((slide, index) =>
    index % 5 === 3 ? slide : { ...slide, thumbnail: plates[index % 8] ?? '' },
  );
}

/** The comments on this deck, as the pane's list holds them. */
const deckComments: readonly VirtualItem[] = [
  { id: 'c1', label: 'Can we lead with the revenue chart instead?', detail: 'Ada Lovelace' },
  { id: 'c2', label: 'Section 3 runs long — cut two slides.', detail: 'Grace Hopper' },
  { id: 'c3', label: 'Check the footer date on every slide.', detail: 'Ada Lovelace' },
  { id: 'c4', label: 'This transition is doing too much.', detail: 'Charles Babbage' },
  {
    id: 'c5',
    label: 'Reworded in the deck template.',
    detail: 'Grace Hopper',
    unavailable: 'This comment was resolved and cannot be replied to.',
  },
  { id: 'c6', label: 'Add the source under the table.', detail: 'Ada Lovelace' },
];

/** What the mini toolbar offers over a selected shape. */
const shapeCommands: readonly MiniCommand[] = [
  { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle', pressed: true },
  { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
  { command: 'align-left', label: 'Align left', icon: 'text-align-left', separatorBefore: true },
  { command: 'align-center', label: 'Centre', icon: 'text-align-center' },
  { command: 'comment', label: 'New comment', icon: 'comment', separatorBefore: true },
  {
    command: 'delete',
    label: 'Delete',
    icon: 'delete',
    unavailable: true,
    explanation: 'This shape is on the slide layout and cannot be deleted from the slide.',
  },
];

// ── the ribbon ──────────────────────────────────────────────────────────

/**
 * **PowerPoint's ribbon, from `stories/ribbons/powerpoint.ts`** — the same functions
 * `Ribbons/PowerPoint` audits.
 *
 * The tabs used to be written here; moving them out is the ribbon programme's unit 0. What stays is
 * what belongs to an *application* rather than to a ribbon: this machine's font list, this deck's
 * palette, the id of the menu the paste button opens, the shape-style gallery's contents and the
 * screentip that explains Arrange. They are bound by the stable command ids
 * `dev/ribbons/census.ts` declares.
 *
 * `powerpointTabs()` leaves out the eight `appearance: 'view'` tabs — the two colour modes, the
 * four masters, Print Preview and Background Removal — because Office shows none of them in the
 * ordinary strip. The contextual set stays here as a call rather than as markup, for the reason it
 * always had: a coloured band naming a set of tabs is the most obviously *compositional* thing in
 * a ribbon, and whether it belongs to this chrome is not a question a component's own story can put.
 */
function ribbon(): TemplateResult {
  return surface(
    'ribbon',
    'flex:0 0 auto;min-inline-size:0',
    html`
      <mjx-ribbon label="PowerPoint" selected="home" @mjx-activate=${openDeclaredSurface}>
        ${powerpointTabs({
          controls: {
            'powerpoint.home.clipboard.paste': html`<mjx-split-button
              slot="essential"
              label="Paste"
              icon="clipboard-paste"
              size="large"
              menu-label="Paste options"
              data-opens="ppt-paste-menu"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'powerpoint.home.font.name': html`<mjx-font-picker
              id="ppt-font"
              style=${ribbonFieldStyle}
              label="Font"
              value="Aptos"
              .fonts=${machineFonts}
            ></mjx-font-picker>`,
            'powerpoint.home.font.size': html`<mjx-dropdown
              id="ppt-size"
              label="Font size"
              value="18"
              style=${ribbonNarrowFieldStyle}
            >
              ${['12', '14', '18', '24', '32', '44'].map(
                (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'powerpoint.home.font.colour': html`<mjx-color-picker
              id="ppt-colour"
              style=${ribbonColourFieldStyle}
              label="Font colour"
              show-automatic
              .themePalette=${documentThemePalette}
              .standardColors=${standardColors}
              .recentColors=${recentColors}
            ></mjx-color-picker>`,
            'powerpoint.home.drawing.styles': html`<mjx-gallery
              id="ppt-shape-styles"
              label="Shape styles"
              value="office-1"
              style=${ribbonGalleryStyle}
            >
              ${largeGalleryItems().slice(0, 24)}
            </mjx-gallery>`,
            'powerpoint.home.drawing.arrange': html`<mjx-screentip
              heading="Arrange"
              description="Change how the selected shapes overlap one another, and how they line up."
              shortcut="Alt + J D A"
            >
              <mjx-button label="Arrange" icon="slide-layout"></mjx-button>
            </mjx-screentip>`,
          },
        })}
        ${powerpointContextualSets()}
      </mjx-ribbon>
    `,
  );
}

// ── the workspace ────────────────────────────────────────────────────────────

const railStyle = 'flex:1 1 auto;min-block-size:0;inline-size:100%;border:1px solid var(--theme-border)';

/** The slide, its context menu, the selection a mini toolbar hangs off, and the scrollbar. */
function slideArea(): TemplateResult {
  return canvasRow(
    canvasArea(
      'ppt-canvas',
      html`
      <mjx-context-menu id="ppt-context" style=${contextRegionStyle}>
        ${documentPlaceholder(
          'Slide 4 — “Where the time went”',
          html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
            ${selectionRun('ppt-selection', 'A selected run of title text')} has a mini toolbar
            above it.
          </p>`,
        )}
        <mjx-menu slot="menu" label="Slide" floating>
          <mjx-menu-item label="Cut" icon="cut" shortcut="Ctrl+X"></mjx-menu-item>
          <mjx-menu-item label="Copy" icon="copy" shortcut="Ctrl+C"></mjx-menu-item>
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
            <mjx-menu-item kind="checkbox" label="Guides"></mjx-menu-item>
          </mjx-menu-section>
        </mjx-menu>
      </mjx-context-menu>
    `,
      html`<mjx-mini-toolbar
        id="ppt-mini"
        label="Formatting"
        for="ppt-selection"
        open
        .commands=${shapeCommands}
      ></mjx-mini-toolbar>`,
    ),
    html`<mjx-scrollbar
      id="ppt-scroll"
      label="Deck"
      controls="ppt-canvas"
      pages="40"
      page-height="620"
      viewport="700"
    ></mjx-scrollbar>`,
  );
}

/** The Format Shape pane: the fields Office actually puts there, plus what is still rendering. */
function taskPane(fraction: string): TemplateResult {
  return html`
    <mjx-task-pane
      id="ppt-pane"
      label="Format Shape"
      open
      dock="inlineEnd"
      fraction=${fraction}
    >
      ${paneStack(
        // ⚠ 360 and 202 are **points**, which is what `<mjx-measure-input>`'s `value` always is —
        // `src/inputs/measure.ts` makes the point canonical and the unit a display choice. Written
        // as 12.7 and 7.14 they render as 0.45 cm and 0.25 cm, which is what this shell showed
        // until it was looked at: a number that is wrong and perfectly plausible.
        field(
          'ppt-width',
          'Width',
          html`<mjx-measure-input
            id="ppt-width"
            value="360"
            unit="cm"
            step="0.1"
            min="0"
          ></mjx-measure-input>`,
        ),
        field(
          'ppt-height',
          'Height',
          html`<mjx-measure-input
            id="ppt-height"
            value="202"
            unit="cm"
            step="0.1"
            min="0"
          ></mjx-measure-input>`,
        ),
        html`<mjx-checkbox id="ppt-lock" label="Lock aspect ratio" checked="true"></mjx-checkbox>`,
        field(
          'ppt-align',
          'Text alignment',
          html`<mjx-segmented-control id="ppt-align" label="Text alignment" value="center">
            ${alignmentSegments.map(
              (segment) =>
                html`<mjx-segment value=${segment.value} label=${segment.label}></mjx-segment>`,
            )}
          </mjx-segmented-control>`,
        ),
        field(
          'ppt-transparency',
          'Transparency',
          html`<mjx-slider
            id="ppt-transparency"
            label="Transparency"
            min="0"
            max="100"
            step="1"
            value="20"
            suffix="%"
          ></mjx-slider>`,
        ),
        field(
          'ppt-spacing',
          'Line spacing',
          html`<mjx-combo-box id="ppt-spacing" label="Line spacing" value="1.15">
            ${lineSpacingOptions.map(
              (option) => html`<mjx-option
                value=${option.value}
                label=${option.label}
                description=${option.description ?? ''}
                ?unavailable=${option.unavailable === true}
                explanation=${option.explanation ?? ''}
              ></mjx-option>`,
            )}
          </mjx-combo-box>`,
        ),
        html`<mjx-popover id="ppt-fill" label="Fill" kind="flyout">
          <mjx-button slot="anchor" label="Fill…" icon="settings"></mjx-button>
          <mjx-checkbox id="ppt-gradient" label="Gradient fill"></mjx-checkbox>
          <mjx-checkbox id="ppt-shadow" label="Shadow"></mjx-checkbox>
        </mjx-popover>`,
        paneHeading('Comments'),
        html`<mjx-virtual-list
          id="ppt-comments"
          label="Comments on this deck"
          style="block-size:9rem;border:1px solid var(--theme-border-subtle);
                 border-radius:var(--radius-control)"
          .items=${deckComments}
        ></mjx-virtual-list>`,
        html`<mjx-progress
          id="ppt-render"
          label="Rendering slide thumbnails"
          value="0.62"
          readout
        ></mjx-progress>`,
      )}
    </mjx-task-pane>
  `;
}

/** The bar across the foot, and the readings a deck actually carries. */
function foot(): TemplateResult {
  return statusBar(
    'ppt-status',
    'Presentation status',
    [
      { id: 'ppt-slide', label: 'Slide', value: '4 of 40', priority: 'essential' },
      { id: 'ppt-notes', label: 'Notes', value: 'Hidden', priority: 'supplementary' },
      { id: 'ppt-a11y', label: 'Accessibility', value: '3 to review', priority: 'standard' },
      {
        id: 'ppt-theme',
        label: 'Theme',
        value: 'Office',
        priority: 'ancillary',
        region: 'centre',
      },
    ],
    zoom('ppt-zoom', { width: 1280, height: 720 }),
  );
}

// ── the desktop and tablet shells ────────────────────────────────────────────

/**
 * Everything above the phone.
 *
 * Desktop and tablet are the **same assembly**, deliberately — the ribbon, the rail, the pane and
 * the status bar each decide their own presentation from the width they are given, and a shell that
 * swapped in different components at 834 would be hiding the very question the middle size exists to
 * ask.
 *
 * ⚠ **What does change between the two is the split, and it had to.** `<mjx-task-pane>` will not lay
 * out below about 336 px — its own fields have a floor — so a fraction that gives a docked pane a
 * quarter of a desktop gives it 200 px of an 834 px tablet and the workspace overflows by 136. The
 * assembly's answer is to give the pane a *larger share* of the smaller screen and take it out of
 * the thumbnail rail, which is a decision rather than an arithmetic accident: at tablet the pane is
 * the thing a person opened, and the rail is the thing they can scroll past. The alternative —
 * making the pane an overlay at tablet — is a presentation `<mjx-task-pane>` does not have, and it
 * is one of the questions in `ui/README.md` for MJXOFF-195.
 */
function wideShell(size: 'desktop' | 'tablet'): TemplateResult {
  // ⚠ 0.19 at tablet and not less. A thumbnail rail under about 150 px wraps a slide title to one
  // character per line — which is what 0.14 produced, and which reads as a broken component rather
  // than as a pane somebody made too narrow.
  const fraction = size === 'desktop' ? '0.16' : '0.19';
  const paneFraction = size === 'desktop' ? '0.24' : '0.41';
  return shellFrame(
    'powerpoint',
    size,
    ribbon(),
    surface(
      'workspace',
      workspaceStyle,
      html`
        ${navigatorPane(
          'ppt-rail-pane',
          fraction,
          html`<mjx-thumbnail-rail
            id="ppt-rail"
            label="Slides"
            style=${railStyle}
            .slides=${deck()}
          ></mjx-thumbnail-rail>`,
        )}
        ${paneSplitter('ppt-split', 'ppt-rail-pane', fraction)}
        ${documentColumn(slideArea())} ${taskPane(paneFraction)}
      `,
    ),
    foot(),
    html`
      <mjx-menu id="ppt-paste-menu" label="Paste options" floating>
        <mjx-menu-section label="Paste">
          <mjx-menu-item kind="radio" label="Use Destination Theme" checked></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Keep Source Formatting"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Picture"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Keep Text Only"></mjx-menu-item>
        </mjx-menu-section>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
      </mjx-menu>
      <mjx-dialog id="ppt-paragraph" label="Paragraph" modal>
        ${paneStack(
          field(
            'ppt-indent',
            'Indentation before text',
            html`<mjx-measure-input id="ppt-indent" value="0" unit="cm" step="0.25"></mjx-measure-input>`,
          ),
          field(
            'ppt-before',
            'Space before',
            html`<mjx-measure-input id="ppt-before" value="10" unit="pt" step="1"></mjx-measure-input>`,
          ),
          html`<mjx-checkbox id="ppt-widow" label="Widow/orphan control" checked="true"></mjx-checkbox>`,
        )}
      </mjx-dialog>
    `,
    /*
     * ⚠ **Desktop only, and that is a decision.** `<mjx-toast-region>` pins itself to the viewport's
     * corner, so a paused stack sits over whatever is in that corner — at tablet it covered the task
     * pane's Height field, and at phone it would cover the command rail, which is the one thing on
     * that screen a thumb must be able to reach. A reviewer needs to see a toast beside real chrome
     * once; they do not need it in front of eight other shells.
     */
    size === 'desktop'
      ? html`
          <mjx-toast-region id="ppt-toasts" label="Notifications" paused>
            <mjx-toast tone="success" message="Saved to OneDrive."></mjx-toast>
            <mjx-toast
              tone="warning"
              message="Two fonts in this deck are not installed. They were substituted."
            ></mjx-toast>
          </mjx-toast-region>
        `
      : html``,
  );
}

// ── the stories ──────────────────────────────────────────────────────────────

/** The whole application at 1440. */
export const Desktop: Story = {
  globals: { containerPreset: 'desktop' },
  render: () => wideShell('desktop'),
};

/**
 * **The middle size, which is the one that breaks.**
 *
 * At 834 the ribbon is neither full nor collapsed, and the task pane and the thumbnail rail are both
 * still docked — so the slide gets what is left of the width, which is the proportion to judge.
 */
export const Tablet: Story = {
  globals: { containerPreset: 'tablet' },
  render: () => wideShell('tablet'),
};

/**
 * **The phone, where the ribbon is a rail.**
 *
 * There is no `<mjx-ribbon>` here at all: MJXOFF-194's command bar *is* the phone's ribbon, ordered
 * by the same demotion ladder, and the contextual action bar replaces it the moment something is
 * selected. Press the overflow control on either: every command is still in the same rail.
 *
 * Press **Styles** on the command rail and a sheet arrives at its half detent; drag its handle and
 * it snaps between the three.
 */
export const Mobile: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    shellFrame(
      'powerpoint',
      'mobile',
      phoneBody(
        canvasArea('ppt-phone-canvas', documentPlaceholder('Slide 4 — “Where the time went”')),
      ),
      phoneRails(
        html`<mjx-contextual-action-bar
          id="ppt-phone-selection"
          selection="object"
          selection-label="a picture"
        ></mjx-contextual-action-bar>`,
        html`<mjx-command-bar
          id="ppt-phone-commands"
          label="Home"
          .commands=${wordPhoneCommands}
          @mjx-mobile-command=${openSheetOnCommand('styles', 'ppt-phone-sheet')}
        ></mjx-command-bar>`,
      ),
      html`<mjx-dialog id="ppt-phone-sheet" label="Slide layout" modal detent="half">
        <mjx-gallery id="ppt-phone-layouts" label="Slide layouts" value="office-1">
          ${largeGalleryItems().slice(0, 12)}
        </mjx-gallery>
      </mjx-dialog>`,
    ),
};
