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
 * **File** is the ribbon programme's unit 1 and is authored; see `powerpointFileTab` for what
 * PowerPoint's own two lists are.
 *
 * **Home** is unit 2: all six in-scope groups — Clipboard, Slides, Font, Paragraph, Drawing and
 * Editing — carrying every command Office's Home tab shows. `GroupSlides` arrives with it, having
 * been declared in the census since unit 0 and rendered by nothing.
 *
 * **Insert** is unit 3: all eleven in-scope groups, the most of any application's Insert tab,
 * because a deck splits Word's Illustrations into Images and Illustrations and adds Camera and Media.
 * Fourteen commands are dropdowns or split buttons a host binds by id.
 *
 * **Draw** is unit 4: nine groups and fifteen commands, declared by the same functions as Word's.
 *
 * **Design** is unit 5: three groups and four commands, two of them in-ribbon galleries. Every other tab
 * is a placeholder until its own unit.
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

/**
 * File: Info, Open, Save, Print, Share, Export, Help.
 *
 * The shape and the reasoning are `stories/ribbons/word.ts`'s — the groups are the census's
 * backstage *destinations*, and Print is the one group with a real dialog behind it. What is
 * PowerPoint's own is in the two lists the census carries: **Share** has *Publish Slides*, which
 * uploads slides one at a time to a library, and **Export** has *Create a Video*, *Package
 * Presentation for CD* and *Create Handouts*. A deck is the only document with a second shape to be
 * printed in and the only one that can be played, which is why this tab is longer than Word's.
 */
export function powerpointFileTab(options: TabOptions = {}): TemplateResult {
  const file = entry('file');
  const controls = options.controls ?? {};
  return tab(
    file.id,
    file.label,
    censusGroup(file, 'TabInfo', {}, controls),
    censusGroup(file, 'TabRecent', {}, controls),
    censusGroup(file, 'TabSave', {}, controls),
    censusGroup(file, 'TabPrint', { launcher: 'Page setup' }, controls),
    censusGroup(file, 'TabShare', {}, controls),
    censusGroup(file, 'TabPublish', {}, controls),
    censusGroup(file, 'TabHelp', {}, controls),
  );
}

/**
 * Home: Clipboard, Slides, Font, Paragraph, Drawing, Editing — the ribbon programme's unit 2, in
 * Office's order.
 *
 * **Slides is the second group and unit 2 is where it arrives**, declared in the census since unit
 * 0 and rendered by nothing until now — which meant a PowerPoint ribbon in this catalogue had no
 * way to add a slide. It sits between Clipboard and Font because that is where Office puts it: the
 * deck's own structure before anything about the text on a slide.
 */
export function powerpointHomeTab(options: TabOptions = {}): TemplateResult {
  const home = entry('home');
  const controls = options.controls ?? {};
  return tab(
    home.id,
    home.label,
    censusGroup(home, 'GroupClipboard', { launcher: 'Clipboard settings' }, controls),
    censusGroup(home, 'GroupSlides', {}, controls),
    censusGroup(home, 'GroupFont', { launcher: 'Font settings' }, controls),
    censusGroup(home, 'GroupParagraph', { launcher: 'Paragraph settings' }, controls),
    censusGroup(home, 'GroupDrawing', { launcher: 'Shape settings' }, controls),
    censusGroup(home, 'GroupEditing', {}, controls),
  );
}

/**
 * Insert: Slides, Tables, Images, Illustrations, Camera, Links, Comments, Text, Symbols, Media, Content
 * — the ribbon programme's unit 3.
 *
 * `GUESS:` **the order of two groups.** Slides through Illustrations, and Links through Media, are
 * Office's order. **Camera** is drawn after Illustrations and **Content** last because that is where
 * the declaration puts them, and no Office build this project can cite puts them anywhere else;
 * `dev/ribbons/census.ts` records what Content is not known to be. Office's Forms, Power BI and
 * Add-ins groups are out of scope in the census and are not drawn.
 *
 * **Fourteen of the tab's twenty-eight commands are bound by the host**: New Slide, Table, Pictures,
 * Screenshot, Photo Album, Shapes, 3D Models, Cameo, Zoom, Link, WordArt, Equation, Video and Audio.
 * Each opens a menu from `stories/ribbons/insert-menus.ts`. Symbol is **not** bound here, although
 * Word's is: PowerPoint's opens the Symbol dialog directly, so the generic button is the right shape.
 *
 * **No dialog launchers**, as in Word, and **no group keeps a survivor**.
 */
export function powerpointInsertTab(options: TabOptions = {}): TemplateResult {
  const insert = entry('insert');
  const controls = options.controls ?? {};
  return tab(
    insert.id,
    insert.label,
    censusGroup(insert, 'GroupSlides2', {}, controls),
    censusGroup(insert, 'GroupInsertTables', {}, controls),
    censusGroup(insert, 'GroupImages', {}, controls),
    censusGroup(insert, 'GroupInsertIllustrations', {}, controls),
    censusGroup(insert, 'GroupChunkCameoCamera', {}, controls),
    censusGroup(insert, 'GroupInsertLinks', {}, controls),
    censusGroup(insert, 'GroupInsertComments', {}, controls),
    censusGroup(insert, 'GroupInsertText', {}, controls),
    censusGroup(insert, 'GroupInsertSymbols', {}, controls),
    censusGroup(insert, 'GroupInsertMediaClips', {}, controls),
    censusGroup(insert, 'GroupContent', {}, controls),
  );
}

/**
 * Draw: Drawing Tools, Pens, Write, Stencils, Input Mode, Draw with Touch, Replay, Help, Close — the
 * ribbon programme's unit 4.
 *
 * Word's Draw tab without Editing (Ink Editor is Word's) and Drawing Canvas (a deck has no canvas to
 * insert). Every group here is declared by the same function as Word's, so the reasoning is
 * `wordDrawTab`'s and `dev/ribbons/census.ts`'s — including `GUESS:` **the order is the
 * declaration's**. The census counts **seven** controls in PowerPoint's Pens where the other two count
 * six, and it draws the same three commands: nothing is padded in.
 *
 * **Six of the tab's fifteen commands are bound by the host**, the same six as Word's, over the menus
 * in `stories/ribbons/draw-menus.ts`. **No dialog launchers**, and **one survivor**, Select Objects.
 */
export function powerpointDrawTab(options: TabOptions = {}): TemplateResult {
  const draw = entry('draw');
  const controls = options.controls ?? {};
  return tab(
    draw.id,
    draw.label,
    censusGroup(draw, 'GroupDrawingTools', {}, controls),
    censusGroup(draw, 'GroupPens2', {}, controls),
    censusGroup(draw, 'GroupWrite', {}, controls),
    censusGroup(draw, 'GroupStencils', {}, controls),
    censusGroup(draw, 'GroupInputMode', {}, controls),
    censusGroup(draw, 'GroupDrawWithTouch', {}, controls),
    censusGroup(draw, 'InkReplay', {}, controls),
    censusGroup(draw, 'GroupPenAndInkHelp', {}, controls),
    censusGroup(draw, 'GroupInkClose', {}, controls),
  );
}

/**
 * Design: Themes, Variants, Customise — the ribbon programme's unit 5, in Office's order.
 *
 * **Two of the three groups are one gallery each**, which is Office's own shape: the Themes strip and
 * the Variants strip, each bound by the host as `<mjx-gallery>`. Variants' Colours, Fonts, Effects and
 * Background Styles are **buttons in that gallery's footer**, where Office puts them, and each opens a
 * menu. Office's Designer group is out of scope in the census and is not drawn.
 *
 * **Three of the four commands are bound by the host**: the two galleries, and Slide Size as a dropdown
 * over `stories/ribbons/design-layout-menus.ts`. Format Background opens a pane and is the generic button.
 *
 * **No dialog launchers**: Office puts none on its Design tab. **No survivor.**
 */
export function powerpointDesignTab(options: TabOptions = {}): TemplateResult {
  const design = entry('design');
  const controls = options.controls ?? {};
  return tab(
    design.id,
    design.label,
    censusGroup(design, 'GroupSlideThemes', {}, controls),
    censusGroup(design, 'GroupThemeVariants', {}, controls),
    censusGroup(design, 'GroupCustomizeThemeOptions', {}, controls),
  );
}

// ── the tabs their own units author ──────────────────────────────────────────

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
