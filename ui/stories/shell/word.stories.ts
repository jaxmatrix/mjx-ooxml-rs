import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MiniCommand } from '../../src/feedback/feedback-model.ts';
import type { ReviewAnnotation } from '../../src/annotation/review-pane.ts';
import type { TreeNode } from '../../src/navigators/navigator-model.ts';
import { smallOutline, smallOutlineExpanded } from '../navigators/specimens.ts';
import { lineSpacingOptions } from '../inputs/specimens.ts';
import { documentThemePalette, machineFonts, recentColors, standardColors } from '../pickers/specimens.ts';
import { styleGalleryItems } from '../gallery/specimens.ts';
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
  group,
  navigatorPane,
  openDeclaredSurface,
  openSheetOnCommand,
  paneSplitter,
  paneStack,
  phoneBody,
  phoneRails,
  selectionRun,
  shellFrame,
  statusBar,
  stubTab,
  surface,
  toggle,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonNarrowFieldStyle,
  workspaceStyle,
  zoom,
} from './shell-parts.ts';

/**
 * **Word, assembled** — the ribbon, the navigation pane, the page, the review margin and the status
 * bar, at desktop, tablet and phone.
 *
 * Word is the shell with **two** panes flanking the document, and that is the composition question
 * this story exists to put: a navigation pane on one side and a review margin on the other leave the
 * page whatever is between them, and at tablet width there may not be enough of it. What to look at:
 *
 * 1. **The desktop.** Whether the page reads as a page rather than as a column between two panels.
 * 2. **The margin.** Cards packed to their anchors beside a document, in both schemes — the author
 *    colours were searched for against the palette, and this is the first time they are seen next to
 *    a ribbon and a status bar rather than on their own.
 * 3. **The tablet.** Both panes are still docked. Decide whether that is right.
 * 4. **The scroll marks.** Search hits, comments and tracked changes along the scrollbar, beside a
 *    review margin that says the same thing a different way.
 *
 * **Nothing here dispatches a command.** Press the Paragraph group's dialog launcher and a modal
 * arrives over the assembled shell; nothing in the document moves, because command dispatch and
 * document binding are loop 2.
 */

const conventions = storyConventions({
  statesMatrix: shellStatesMatrix,
  tokenDependencies: shellTokenDependencies,
  keyboard: shellKeyboard,
  screenReader: shellScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Shell/Word',
  parameters: {
    docs: {
      description: {
        component:
          'The whole application assembled from the catalogue’s own components: ribbon, navigation ' +
          'pane, page, review margin and status bar on a desktop; a command rail and a sheet on a ' +
          'phone. Cosmetic only — every command is inert.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

// ── the fixtures ─────────────────────────────────────────────────────────────

/** The document's outline, with a few more headings than the navigators' small specimen. */
const outline: readonly TreeNode[] = [
  ...smallOutline,
  {
    id: 'findings',
    label: 'Findings',
    children: [
      { id: 'findings-fidelity', label: 'Fidelity' },
      { id: 'findings-performance', label: 'Performance' },
      { id: 'findings-open', label: 'Open questions' },
    ],
  },
];

/**
 * The review margin's contents: **all three card kinds at once**, which is the point of putting them
 * in a shell.
 *
 * A legacy comment, a threaded conversation, a resolved one and three tracked changes — so the
 * author colours have four authors to tell apart and the packer has anchors close enough together to
 * have to move something.
 */
const annotations: readonly ReviewAnnotation[] = [
  {
    id: 'a1',
    kind: 'comment',
    model: 'threaded',
    author: 'Ada Lovelace',
    time: '09:12',
    text: 'Define “fidelity” here.',
    anchorTop: 8,
    replies: [{ id: 'a1r1', author: 'Grace Hopper', time: '09:40', text: 'Byte identity.' }],
  },
  {
    id: 'a2',
    kind: 'comment',
    model: 'legacy',
    author: 'Charles Babbage',
    time: '10:02',
    text: 'Predates the corpus.',
    anchorTop: 70,
  },
  {
    id: 'a3',
    kind: 'trackedChange',
    changeKind: 'insertion',
    author: 'Grace Hopper',
    time: '09:41',
    text: 'Inserted a definition.',
    excerpt: 'byte identity',
    anchorTop: 132,
  },
  {
    id: 'a4',
    kind: 'trackedChange',
    changeKind: 'deletion',
    author: 'Ada Lovelace',
    time: '10:15',
    text: 'Removed a clause.',
    excerpt: 'and structural identity',
    anchorTop: 152,
  },
  {
    id: 'a5',
    kind: 'comment',
    model: 'threaded',
    author: 'Katherine Johnson',
    time: '11:30',
    text: 'Agreed — closing.',
    anchorTop: 240,
    resolved: true,
    replies: [],
  },
  {
    id: 'a6',
    kind: 'trackedChange',
    changeKind: 'formatting',
    author: 'Katherine Johnson',
    time: '11:44',
    text: 'Set to Heading 2.',
    excerpt: 'The corpus',
    anchorTop: 320,
  },
];

/** What the mini toolbar offers over selected text. */
const textCommands: readonly MiniCommand[] = [
  { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle', pressed: true },
  { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
  { command: 'underline', label: 'Underline', icon: 'text-underline', kind: 'toggle' },
  { command: 'align-left', label: 'Align left', icon: 'text-align-left', separatorBefore: true },
  { command: 'comment', label: 'New comment', icon: 'comment', separatorBefore: true },
];

// ── the ribbon ───────────────────────────────────────────────────────────────

function ribbon(): TemplateResult {
  return surface(
    'ribbon',
    'flex:0 0 auto;min-inline-size:0',
    html`
      <mjx-ribbon label="Word" selected="home" @mjx-activate=${openDeclaredSurface}>
        <mjx-ribbon-tab tab-id="home" label="Home">
          ${group(
            'Clipboard',
            'secondary',
            { launcher: 'Clipboard settings' },
            html`<mjx-split-button
              slot="essential"
              label="Paste"
              icon="clipboard-paste"
              size="large"
              menu-label="Paste options"
              data-opens="word-paste-menu"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            html`<mjx-button label="Cut" icon="cut"></mjx-button>`,
            html`<mjx-button label="Copy" icon="copy"></mjx-button>`,
            html`<mjx-button label="Format Painter" icon="settings"></mjx-button>`,
          )}
          ${group(
            'Font',
            'primary',
            { launcher: 'Font settings' },
            html`<mjx-font-picker
              id="word-font"
              style=${ribbonFieldStyle}
              label="Font"
              value="Cambria"
              .fonts=${machineFonts}
            ></mjx-font-picker>`,
            html`<mjx-dropdown id="word-size" label="Font size" value="11" style=${ribbonNarrowFieldStyle}>
              ${['9', '10', '11', '12', '14', '18'].map(
                (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            toggle('Bold', 'text-bold'),
            toggle('Italic', 'text-italic', true),
            toggle('Underline', 'text-underline'),
            html`<mjx-color-picker
              id="word-colour"
              style=${ribbonColourFieldStyle}
              label="Font colour"
              show-automatic
              .themePalette=${documentThemePalette}
              .standardColors=${standardColors}
              .recentColors=${recentColors}
            ></mjx-color-picker>`,
          )}
          ${group(
            'Paragraph',
            'primary',
            { launcher: 'Paragraph settings' },
            toggle('Align left', 'text-align-left', true),
            toggle('Centre', 'text-align-center'),
            toggle('Align right', 'text-align-right'),
            html`<mjx-button label="Bullets" icon="add"></mjx-button>`,
            html`<mjx-button label="Numbering" icon="subtract"></mjx-button>`,
            html`<mjx-button label="Borders" icon="table"></mjx-button>`,
          )}
          ${group(
            'Styles',
            'standard',
            { launcher: 'Styles pane' },
            html`<mjx-gallery
              id="word-styles"
              label="Styles"
              value="normal"
              style=${ribbonGalleryStyle}
            >
              ${styleGalleryItems()}
            </mjx-gallery>`,
          )}
          ${group(
            'Editing',
            'ancillary',
            {},
            html`<mjx-button slot="essential" label="Find" icon="search"></mjx-button>`,
            html`<mjx-button label="Replace" icon="arrow-redo"></mjx-button>`,
            html`<mjx-button label="Select" icon="checkmark"></mjx-button>`,
          )}
        </mjx-ribbon-tab>

        ${stubTab('insert', 'Insert', 'Table', 'table')}
        ${stubTab('layout', 'Layout', 'Margins', 'slide-layout')}
        ${stubTab('references', 'References', 'Insert Citation', 'document')}
        ${stubTab('mailings', 'Mailings', 'Start Mail Merge', 'folder-open')}
        ${stubTab('review', 'Review', 'New Comment', 'comment')}
        ${stubTab('view', 'View', 'Navigation Pane', 'search')}

        <mjx-contextual-tab-set label="Table Tools">
          ${stubTab('table-design', 'Design', 'Table Styles', 'table')}
          ${stubTab('table-layout', 'Layout', 'Merge Cells', 'add')}
        </mjx-contextual-tab-set>
      </mjx-ribbon>
    `,
  );
}

// ── the workspace ────────────────────────────────────────────────────────────

const treeStyle = 'flex:1 1 auto;min-block-size:0;inline-size:100%;border:1px solid var(--theme-border)';

const marginStyle =
  'flex:0 0 auto;box-sizing:border-box;inline-size:22rem;max-inline-size:40%;min-inline-size:0;' +
  'padding:var(--mjx-density-gutter);padding-inline-start:0';

/** The page, its context menu, a selection with a mini toolbar, and the marked scrollbar. */
function pageArea(): TemplateResult {
  return canvasRow(
    canvasArea(
      'word-canvas',
      html`
      <mjx-context-menu id="word-context" style=${contextRegionStyle}>
        ${documentPlaceholder(
          'Page 4 of 20 — “Method”',
          html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
            ${selectionRun('word-selection', 'A selected sentence')} has a mini toolbar above it.
          </p>`,
        )}
        <mjx-menu slot="menu" label="Text" floating>
          <mjx-menu-item label="Cut" icon="cut" shortcut="Ctrl+X"></mjx-menu-item>
          <mjx-menu-item label="Copy" icon="copy" shortcut="Ctrl+C"></mjx-menu-item>
          <mjx-menu-item label="New Comment" icon="comment"></mjx-menu-item>
          <mjx-menu-separator></mjx-menu-separator>
          <mjx-menu-item label="Styles">
            <mjx-menu slot="submenu" label="Styles">
              <mjx-menu-item kind="radio" label="Normal" checked></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Heading 1"></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Quote"></mjx-menu-item>
            </mjx-menu>
          </mjx-menu-item>
          <mjx-menu-separator></mjx-menu-separator>
          <mjx-menu-section label="Review">
            <mjx-menu-item kind="checkbox" label="Track Changes" checked></mjx-menu-item>
            <mjx-menu-item kind="checkbox" label="Show Markup" checked></mjx-menu-item>
          </mjx-menu-section>
        </mjx-menu>
      </mjx-context-menu>
    `,
      html`<mjx-mini-toolbar
        id="word-mini"
        label="Formatting"
        for="word-selection"
        open
        .commands=${textCommands}
      ></mjx-mini-toolbar>`,
    ),
    html`<mjx-scrollbar
      id="word-scroll"
      label="Document"
      controls="word-canvas"
      pages="20"
      page-height="1100"
      viewport="700"
    >
      <mjx-scroll-mark kind="search" page="1" within="0.25" label="fidelity"></mjx-scroll-mark>
      <mjx-scroll-mark kind="search" page="6" within="0.5" label="fidelity"></mjx-scroll-mark>
      <mjx-scroll-mark kind="comment" page="4" label="Ask legal"></mjx-scroll-mark>
      <mjx-scroll-mark kind="comment" page="12" within="0.8" label="Reword"></mjx-scroll-mark>
      <mjx-scroll-mark kind="change" page="9" within="0.4" label="Inserted"></mjx-scroll-mark>
      <mjx-scroll-mark kind="change" page="17" within="0.1" label="Deleted"></mjx-scroll-mark>
    </mjx-scrollbar>`,
  );
}

/** The bar across the foot, and the readings a Word document actually carries. */
function foot(): TemplateResult {
  return statusBar(
    'word-status',
    'Document status',
    [
      { id: 'word-page', label: 'Page', value: '4 of 20', priority: 'essential' },
      { id: 'word-words', label: 'Words', value: '3,182', priority: 'standard' },
      { id: 'word-language', label: 'Language', value: 'English (UK)', priority: 'supplementary' },
      { id: 'word-track', label: 'Track Changes', value: 'On', priority: 'standard' },
      {
        id: 'word-section',
        label: 'Section',
        value: '2',
        priority: 'ancillary',
        region: 'centre',
      },
    ],
    zoom('word-zoom', { width: 794, height: 1123 }),
  );
}

// ── the desktop and tablet shells ────────────────────────────────────────────

/** Everything above the phone. Desktop and tablet are the same assembly at two widths. */
function wideShell(size: 'desktop' | 'tablet'): TemplateResult {
  const fraction = size === 'desktop' ? '0.18' : '0.22';
  return shellFrame(
    'word',
    size,
    ribbon(),
    surface(
      'workspace',
      workspaceStyle,
      html`
        ${navigatorPane(
          'word-nav-pane',
          fraction,
          html`<mjx-tree
            id="word-outline"
            label="Navigation"
            value="method-corpus"
            style=${treeStyle}
            .nodes=${outline}
            .expanded=${[...smallOutlineExpanded, 'findings']}
          ></mjx-tree>`,
        )}
        ${paneSplitter('word-split', 'word-nav-pane', fraction)}
        ${documentColumn(pageArea())}
        <div data-mjx-shell-surface="review-margin" style=${marginStyle}>
          <mjx-review-pane
            id="word-review"
            label="Revisions"
            side="inlineStart"
            selected="a3"
            style="display:block;block-size:100%;border:1px solid var(--theme-border);
                   border-radius:var(--radius-control)"
            .annotations=${annotations}
          ></mjx-review-pane>
        </div>
      `,
    ),
    foot(),
    html`
      <mjx-menu id="word-paste-menu" label="Paste options" floating>
        <mjx-menu-section label="Paste">
          <mjx-menu-item kind="radio" label="Keep Source Formatting" checked></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Merge Formatting"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Keep Text Only"></mjx-menu-item>
        </mjx-menu-section>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
        <mjx-menu-item label="Set Default Paste"></mjx-menu-item>
      </mjx-menu>
      <mjx-dialog id="word-paragraph" label="Paragraph" modal>
        ${paneStack(
          field(
            'word-indent',
            'Indentation before text',
            html`<mjx-measure-input
              id="word-indent"
              value="0"
              unit="cm"
              step="0.25"
            ></mjx-measure-input>`,
          ),
          field(
            'word-after',
            'Space after',
            html`<mjx-measure-input id="word-after" value="8" unit="pt" step="1"></mjx-measure-input>`,
          ),
          field(
            'word-spacing',
            'Line spacing',
            html`<mjx-dropdown id="word-spacing" label="Line spacing" value="1.15">
              ${lineSpacingOptions.map(
                (option) => html`<mjx-option
                  value=${option.value}
                  label=${option.label}
                  description=${option.description ?? ''}
                  ?unavailable=${option.unavailable === true}
                  explanation=${option.explanation ?? ''}
                ></mjx-option>`,
              )}
            </mjx-dropdown>`,
          ),
          html`<mjx-checkbox
            id="word-keep"
            label="Keep with next"
            checked="mixed"
          ></mjx-checkbox>`,
        )}
      </mjx-dialog>
    `,
  );
}

// ── the stories ──────────────────────────────────────────────────────────────

/** The whole application at 1440, with both panes docked. */
export const Desktop: Story = {
  globals: { containerPreset: 'desktop' },
  render: () => wideShell('desktop'),
};

/**
 * **The middle size, and Word's version of it is the hardest of the three.**
 *
 * A navigation pane on one side and a review margin on the other, at 834, leave the page a column.
 * Whether that is acceptable — or whether one of the two should become an overlay here — is a
 * decision for MJXOFF-195, and it is stated in `ui/README.md` rather than taken silently.
 */
export const Tablet: Story = {
  globals: { containerPreset: 'tablet' },
  render: () => wideShell('tablet'),
};

/**
 * **The phone.** No ribbon, no panes: a command rail, a contextual action bar for the selection, and
 * a sheet. Press **Styles** on the rail and the sheet arrives at its half detent.
 *
 * The review pane is here as well, and in its **other** presentation: below the phone-shell width it
 * is a list in flow rather than a packed margin, so the same component that shows two cards beside a
 * desktop page shows every annotation here. Word Mobile's comments view is exactly that, and it is
 * the only place in these nine shells where all three card kinds are on screen at once.
 */
export const Mobile: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    shellFrame(
      'word',
      'mobile',
      phoneBody(
        canvasArea(
          'word-phone-canvas',
          documentPlaceholder(
            'Page 4 of 20 — “Method”',
            html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
              ${selectionRun('word-phone-selection', 'twelve selected words')} is what the bar below
              is about.
            </p>`,
          ),
        ),
        html`<mjx-review-pane
          id="word-phone-review"
          label="Revisions"
          side="inlineStart"
          selected="a2"
          style="display:block;flex:0 0 auto;block-size:16rem;
                 border:1px solid var(--theme-border);border-radius:var(--radius-control)"
          .annotations=${annotations}
        ></mjx-review-pane>`,
      ),
      phoneRails(
        html`<mjx-contextual-action-bar
          id="word-phone-selection-bar"
          selection="text"
          selection-label="12 words"
        ></mjx-contextual-action-bar>`,
        html`<mjx-command-bar
          id="word-phone-commands"
          label="Home"
          .commands=${wordPhoneCommands}
          @mjx-mobile-command=${openSheetOnCommand('styles', 'word-phone-sheet')}
        ></mjx-command-bar>`,
      ),
      html`<mjx-dialog id="word-phone-sheet" label="Styles" modal detent="half">
        <mjx-gallery id="word-phone-styles" label="Styles" value="normal">
          ${styleGalleryItems()}
        </mjx-gallery>
      </mjx-dialog>`,
    ),
};
