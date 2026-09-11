/**
 * **PowerPoint's ribbon** — every tab, in Office's order, from one source.
 *
 * The shape and the reasoning are `stories/ribbons/word.ts`'s; what differs is the scale. The
 * census marks **eighteen** in-scope core tabs for PowerPoint, and eight of them are
 * `appearance: 'view'` — the two colour modes, the four masters, Print Preview and Background
 * Removal. Office shows none of those in the ordinary strip, so a shell that had them would be
 * showing a ribbon that does not exist; `powerpointTabs()` therefore leaves them out unless asked.
 *
 * ⚠ **Two tabs are both called Home.** `TabSlideMasterHome` is Office's own name for the Home tab
 * that appears in Slide Master view, and the two are never on screen together there. A catalogue
 * story that renders every tab at once is the one place they collide, and the collision is the
 * catalogue's artefact rather than a transcription slip — see `dev/ribbons/census.ts`.
 *
 * **Home** carries the commands migrated out of `stories/shell/powerpoint.stories.ts`, unchanged.
 * The census's `GroupSlides` is declared there and not rendered here, because it is not on the
 * shell's Home today and authoring it is unit 2's work.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { powerpointRibbonTabs, ribbonTab } from '../../dev/ribbons/census.ts';
import { stubTab } from '../shell/shell-parts.ts';
import {
  censusGroup,
  placeholderTab,
  tab,
  tabsFor,
  type TabOptions,
} from './ribbon-parts.ts';

const entry = (id: string) => ribbonTab('powerpoint', id);

// ── the authored tabs ────────────────────────────────────────────────────────

/** Home: Clipboard, Font, Paragraph, Drawing, Editing — in Office's order. */
export function powerpointHomeTab(options: TabOptions = {}): TemplateResult {
  const home = entry('home');
  const controls = options.controls ?? {};
  return tab(
    home.id,
    home.label,
    censusGroup(home, 'GroupClipboard', { launcher: 'Clipboard settings' }, controls),
    censusGroup(home, 'GroupFont', { launcher: 'Font settings' }, controls),
    censusGroup(home, 'GroupParagraph', { launcher: 'Paragraph settings' }, controls),
    censusGroup(home, 'GroupDrawing', { launcher: 'Shape settings' }, controls),
    censusGroup(home, 'GroupEditing', {}, controls),
  );
}

// ── the tabs their own units author ──────────────────────────────────────────

export function powerpointFileTab(): TemplateResult {
  return placeholderTab(entry('file'));
}

export function powerpointInsertTab(): TemplateResult {
  return placeholderTab(entry('insert'));
}

export function powerpointDrawTab(): TemplateResult {
  return placeholderTab(entry('draw'));
}

export function powerpointDesignTab(): TemplateResult {
  return placeholderTab(entry('design'));
}

export function powerpointTransitionsTab(): TemplateResult {
  return placeholderTab(entry('transitions'));
}

export function powerpointAnimationsTab(): TemplateResult {
  return placeholderTab(entry('animations'));
}

export function powerpointSlideShowTab(): TemplateResult {
  return placeholderTab(entry('slide-show'));
}

export function powerpointRecordingTab(): TemplateResult {
  return placeholderTab(entry('recording'));
}

export function powerpointReviewTab(): TemplateResult {
  return placeholderTab(entry('review'));
}

export function powerpointViewTab(): TemplateResult {
  return placeholderTab(entry('view'));
}

export function powerpointSlideMasterTab(): TemplateResult {
  return placeholderTab(entry('slide-master'));
}

export function powerpointSlideMasterHomeTab(): TemplateResult {
  return placeholderTab(entry('slide-master-home'));
}

export function powerpointHandoutMasterTab(): TemplateResult {
  return placeholderTab(entry('handout-master'));
}

export function powerpointNotesMasterTab(): TemplateResult {
  return placeholderTab(entry('notes-master'));
}

export function powerpointBlackAndWhiteTab(): TemplateResult {
  return placeholderTab(entry('black-and-white'));
}

export function powerpointGreyscaleTab(): TemplateResult {
  return placeholderTab(entry('greyscale'));
}

export function powerpointPrintPreviewTab(): TemplateResult {
  return placeholderTab(entry('print-preview'));
}

export function powerpointBackgroundRemovalTab(): TemplateResult {
  return placeholderTab(entry('background-removal'));
}

// ── the whole ribbon ─────────────────────────────────────────────────────────

/** Which function builds which tab. Keyed by the census's own kebab ids. */
const builders: Readonly<Record<string, (options: TabOptions) => TemplateResult>> = {
  file: powerpointFileTab,
  home: powerpointHomeTab,
  insert: powerpointInsertTab,
  draw: powerpointDrawTab,
  design: powerpointDesignTab,
  transitions: powerpointTransitionsTab,
  animations: powerpointAnimationsTab,
  'slide-show': powerpointSlideShowTab,
  recording: powerpointRecordingTab,
  review: powerpointReviewTab,
  view: powerpointViewTab,
  'slide-master': powerpointSlideMasterTab,
  'slide-master-home': powerpointSlideMasterHomeTab,
  'handout-master': powerpointHandoutMasterTab,
  'notes-master': powerpointNotesMasterTab,
  'black-and-white': powerpointBlackAndWhiteTab,
  greyscale: powerpointGreyscaleTab,
  'print-preview': powerpointPrintPreviewTab,
  'background-removal': powerpointBackgroundRemovalTab,
};

/** Every tab, in Office's order. See `wordTabs` on why `includeViewTabs` is a parameter. */
export function powerpointTabs(
  options: TabOptions & { readonly includeViewTabs?: boolean } = {},
): TemplateResult[] {
  return tabsFor(
    powerpointRibbonTabs,
    (declared) => {
      const build = builders[declared.id];
      if (build === undefined) {
        throw new Error(
          `stories/ribbons/powerpoint.ts has no builder for the '${declared.id}' tab`,
        );
      }
      return build(options);
    },
    options,
  );
}

/** The contextual tab sets the shell declares today. Unit 11's work — see `wordContextualSets`. */
export function powerpointContextualSets(): TemplateResult {
  return html`
    <mjx-contextual-tab-set label="Picture Tools">
      ${stubTab('picture-format', 'Format', 'Crop', 'cut')}
    </mjx-contextual-tab-set>
  `;
}
