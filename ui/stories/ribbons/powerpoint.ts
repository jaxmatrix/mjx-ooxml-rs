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
 * **Design** is unit 5: three groups and four commands, two of them in-ribbon galleries.
 *
 * **Transitions** is unit 6: three groups and nine commands, a gallery and the timing beside it. Unit 7
 * completed it: every transition Office shows, an Effect Options that follows the gallery, Office's whole
 * Sound list, and the *Advance Slide* caption.
 *
 * **Animations** is unit 7: four groups and twelve commands, Transitions' shape applied to one object.
 *
 * **Review** followed Word's, one tab of one application: seven groups and nineteen commands, the tab
 * where a deck is read by somebody else. Every menu on it carries Office's whole list.
 *
 * **View** followed Word's View, one tab of one application again: seven groups and twenty-four commands,
 * the tab that changes how a deck is looked at and never the deck. Three of its groups are exclusive sets,
 * and Switch Windows is its one menu.
 *
 * **Slide Show** followed the three View tabs, one tab of one application again: four groups and fourteen
 * commands, the tab that plays the deck. Three of its commands open menus, and Hide Slide survives a collapse.
 *
 * **Recording** followed Slide Show, one tab of one application again: ten groups and fifteen commands, drawn
 * from two generations of Office's recorder that the census declares on one tab. Eight commands open menus, four
 * of them Insert's, and nothing survives a collapse.
 *
 * **Background Removal** followed Word's, the first PowerPoint view tab authored: Word's two groups and four
 * commands under PowerPoint's ids, from the census's shared functions. It binds nothing and opens no menu.
 *
 * **Print Preview** followed Excel's Background Removal, PowerPoint's second view tab authored: four groups and
 * ten commands, the deck as it will print. Options and Orientation open menus, and Print What and
 * Colour/Greyscale are fields, all from `stories/ribbons/print-preview-menus.ts`, and `Ribbons/PowerPoint`
 * alone binds and renders them.
 *
 * **Slide Master** followed Excel's Print Preview, PowerPoint's third view tab authored: six groups and eighteen
 * commands, the master and its layouts. Edit Theme, Background and Close are declared once for every master view,
 * and every menu is from `stories/ribbons/slide-master-menus.ts`, over Design's own lists.
 *
 * **Slide Master Home** followed Slide Master, PowerPoint's fourth view tab authored: six groups and forty-four
 * commands, the Home tab Slide Master view shows. Clipboard, Font, Paragraph, Drawing and Editing are Home's, from the
 * census's shared functions under this tab's ids, and Master Slides is its own.
 *
 * **Handout Master** followed Slide Master Home, PowerPoint's fifth view tab authored: five groups and fourteen
 * commands, the printed handout page. Edit Theme, Background and Close are Slide Master's functions under this tab's
 * ids; Page Setup's three menus and Placeholders' four checkboxes are its own.
 *
 * **Notes Master** followed Handout Master, PowerPoint's sixth view tab authored: five groups and fifteen commands, the
 * printed notes page. Edit Theme, Background and Close are Slide Master's functions under this tab's ids; Page Setup's
 * two menus and Placeholders' six checkboxes are its own.
 *
 * **Black and White** followed Notes Master, PowerPoint's seventh view tab authored: two groups and eleven commands,
 * how the selected object prints in black and white. Colour Mode's ten toggles are one exclusive set and Close is one
 * button, both from the census's colour-mode functions, which Greyscale shares. It binds nothing and opens no menu.
 *
 * **Greyscale** followed Black and White, PowerPoint's eighth and last view tab authored: the same two groups and
 * eleven commands from the same two functions, under this tab's ids and with its own exclusive set.
 *
 * No core or view tab is a placeholder any more: Greyscale was the last.
 *
 * **Six contextual tabs in four sets are declared**: Table Design and Layout, Picture Format, Shape Format, and Chart
 * Design and Format. `powerpointContextualSets` draws them, one set or all.
 *
 * **Table Design is authored**, PowerPoint's first contextual tab: four groups and nineteen commands, the style a
 * table wears, the WordArt its text wears and the pen Draw Table draws with. Its gallery, fields and menus are in
 * `stories/ribbons/table-tools-menus.ts`, over Word's table art, and its WordArt Styles group's in
 * `stories/ribbons/wordart-styles-menus.ts`, written for Shape Format and Chart Format. **`Ribbons/PowerPoint` alone
 * binds it**, because `Shell/PowerPoint` draws Picture Tools.
 *
 * **Table Layout is authored**, the second: seven groups and twenty-eight commands, the table's rows, columns and
 * cells, their sizes and alignment, the table's size and its place among the slide's objects. Its Select and Delete
 * menus are Word's shared lists, its Text Direction and Cell Margins menus are in the same file, and its Arrange group
 * is the census's `arrangeCommands` for a table. `Ribbons/PowerPoint` alone binds it.
 *
 * **Picture Format is authored**, the third: six groups and twenty-three commands, how a picture on a slide is
 * corrected, framed, described, placed, cropped and sized. It is Word's tab through Word's functions: every menu, the
 * gallery and the thumbnails in `stories/ribbons/picture-tools-menus.ts`, Arrange from `arrangeCommands` and Size from
 * `sizeCommands`. It differs where Office does: no Position or Wrap Text, an Eyedropper under Picture Border, Convert to
 * SmartArt for Picture Layout, a slide's starting measures and Size's *Size and Position* launcher. **Both PowerPoint
 * hosts bind it**, because `Shell/PowerPoint` draws Picture Tools.
 *
 * **Shape Format is authored**, the fourth and the first of Drawing Tools: six groups and twenty-one commands, how a
 * shape on a slide is drawn, changed and merged, styled, dressed as WordArt, described, placed and sized. Its menus,
 * gallery and starting measures are in `stories/ribbons/drawing-tools-menus.ts`, written for Word's and Excel's Shape
 * Format; WordArt Styles is `wordArtStylesCommands`, Arrange `arrangeCommands` and Size `sizeCommands` for a drawing.
 * **`Ribbons/PowerPoint` alone binds it**, because `Shell/PowerPoint` draws Picture Tools.
 *
 * **Chart Design is authored**, the fifth and PowerPoint's first of Chart Tools: four groups and nine commands, which
 * elements a chart on a slide carries and how they are laid out, its colours and style, its data, and its type. It is
 * Word's tab through Word's functions: the census's `chartLayoutsCommands`, `chartStylesCommands`, `chartDataCommands`
 * and `chartTypeCommands`, and every menu and the gallery in `stories/ribbons/chart-tools-menus.ts`; it differs from
 * Word's in no command, because Office's PowerPoint does not. **`Ribbons/PowerPoint` alone binds it**, because
 * `Shell/PowerPoint` draws Picture Tools. **The last, Chart Tools' Format, is a placeholder** until its own unit.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { powerpointRibbonContextualSets, powerpointRibbonTabs, ribbonTab } from '../../dev/ribbons/census.ts';
import {
  censusGroup,
  contextualSetsFor,
  placeholderTab,
  tab,
  tabsFor,
  type ContextualSetOptions,
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

/**
 * Transitions: Preview, Transition Styles (Office's *Transition to This Slide*), Timing — the ribbon
 * programme's unit 6, in Office's order.
 *
 * ⚠ **The census's two larger groups do not line up with Office's by id.** The labels are the census's
 * and the contents are Office's under those labels: Transition Styles holds the gallery and Effect
 * Options, and Timing holds Sound, Duration, Apply To All, On Mouse Click, After and the advance time.
 * `dev/ribbons/census.ts` records the other reading and why this one was taken.
 *
 * **Eight of the tab's ten entries are bound by the host**: the in-ribbon gallery, Effect Options as a
 * dropdown over `stories/ribbons/references-transitions-formulas-menus.ts` whose entries follow the
 * gallery's commit, the five fields, and the *Advance Slide* caption as `<mjx-label>`. Preview and Apply To
 * All are the generic buttons.
 *
 * **No dialog launchers**: Office puts none on its Transitions tab. **No survivor.**
 */
export function powerpointTransitionsTab(options: TabOptions = {}): TemplateResult {
  const transitions = entry('transitions');
  const controls = options.controls ?? {};
  return tab(
    transitions.id,
    transitions.label,
    censusGroup(transitions, 'GroupPreviewTransitions', {}, controls),
    censusGroup(transitions, 'GroupTransitionStyles', {}, controls),
    censusGroup(transitions, 'GroupTransitionToThisSlide', {}, controls),
  );
}

/**
 * Animations: Preview, Animations (Office's *Animation*), Custom Animation (Office's *Advanced
 * Animation*), Timing — the ribbon programme's unit 7, in Office's order, which is also the census's.
 *
 * **Eight of the tab's twelve commands are bound by the host**: Preview is a split button, the Animation
 * Styles gallery is in-ribbon with Office's footer, Effect Options, Add Animation and Trigger are
 * dropdowns over `stories/ribbons/mailings-animations-data-menus.ts`, and Start, Duration and Delay are
 * fields. Animation Pane is the generic toggle.
 *
 * **One dialog launcher, on Animations**, because Office has one there: it opens the effect's own dialog.
 * `GUESS:` its name. **No survivor.**
 */
export function powerpointAnimationsTab(options: TabOptions = {}): TemplateResult {
  const animations = entry('animations');
  const controls = options.controls ?? {};
  return tab(
    animations.id,
    animations.label,
    censusGroup(animations, 'GroupPreview', {}, controls),
    censusGroup(animations, 'GroupAnimations', { launcher: 'Show additional effect options' }, controls),
    censusGroup(animations, 'GroupAnimationCustom', {}, controls),
    censusGroup(animations, 'GroupAnimationTiming', {}, controls),
  );
}

/**
 * Review: Proofing, Accessibility, Language, Comments, Compare, Activity, Ink, in **Office's** order.
 *
 * ⚠ **Ink is last here and fifth in the census's declaration**, where Microsoft 365 draws it after
 * Compare; `wordReviewTab` makes the same call. Activity is drawn before it. `GUESS:` both positions, and
 * `dev/ribbons/census.ts` records the disagreement. Office's Insights (Smart Lookup) and Chinese
 * Translation are out of scope in the census and are not drawn.
 *
 * **Seven of the tab's nineteen commands are bound by the host**: Check Accessibility, Delete, Accept and
 * Reject are split buttons; Show Comments and Hide Ink are split buttons whose face is a toggle; Language
 * is a dropdown. All seven open menus from `stories/ribbons/review-menus.ts`. Reviewing Pane is the
 * generic toggle.
 *
 * **No dialog launchers**, because Office puts none here. **Two survivors**, Previous Comment and Next
 * Comment, and `dev/ribbons/census.ts` gives the reason.
 */
export function powerpointReviewTab(options: TabOptions = {}): TemplateResult {
  const review = entry('review');
  const controls = options.controls ?? {};
  return tab(
    review.id,
    review.label,
    censusGroup(review, 'GroupProofing', {}, controls),
    censusGroup(review, 'GroupAccessibility', {}, controls),
    censusGroup(review, 'GroupLanguage', {}, controls),
    censusGroup(review, 'GroupComments', {}, controls),
    censusGroup(review, 'GroupReviewCompare', {}, controls),
    censusGroup(review, 'GroupActivity', {}, controls),
    censusGroup(review, 'GroupInk', {}, controls),
  );
}

/**
 * View: Presentation Views, Master Views, Show, Zoom, Colour/Greyscale, Window, View Direction, in the
 * census's order, which is Office's for the six groups Office draws.
 *
 * ⚠ **View Direction is the census's alone**: no Microsoft 365 build this project can cite draws it on an
 * ordinary View tab, so its contents, its position last and its labels are `GUESS:`; `dev/ribbons/census.ts`
 * records the reading. Office's **Macros** is out of scope in the census and is not drawn.
 *
 * **Four of the tab's twenty-four commands are bound by the host**: Ruler, Gridlines and Guides are
 * checkboxes, and Switch Windows is a dropdown over `stories/ribbons/view-menus.ts`. Everything else is the
 * generic toggle or button, including the three exclusive sets.
 *
 * **One dialog launcher, on Show**, where Office opens Grid and Guides. `GUESS:` its name. **One survivor**,
 * Fit to Window, in Zoom.
 */
export function powerpointViewTab(options: TabOptions = {}): TemplateResult {
  const view = entry('view');
  const controls = options.controls ?? {};
  return tab(
    view.id,
    view.label,
    censusGroup(view, 'GroupPresentationViews', {}, controls),
    censusGroup(view, 'GroupMasterViews', {}, controls),
    censusGroup(view, 'GroupViewShowHide', { launcher: 'Grid Settings' }, controls),
    censusGroup(view, 'GroupZoom', {}, controls),
    censusGroup(view, 'GroupColorGrayscale', {}, controls),
    censusGroup(view, 'GroupWindow', {}, controls),
    censusGroup(view, 'GroupViewDirection', {}, controls),
  );
}

/**
 * Slide Show: Start Slide Show, Rehearse, Set Up, Monitors, in **Office's** order.
 *
 * ⚠ **Rehearse is second here and third in the census's declaration**, where Microsoft 365 draws Rehearse with
 * Coach right after Custom Slide Show. `GUESS:` that position, and the command itself, since the census counts
 * one control and names none; `dev/ribbons/census.ts` records both. Office's **Captions & Subtitles**
 * (`GroupLiveSubtitles`) is out of scope in the census and is not drawn.
 *
 * **Eight of the tab's fourteen commands are bound by the host**: Present Online and Custom Slide Show are
 * dropdowns and Record is a split button, all three over `stories/ribbons/slide-show-menus.ts`; Monitor is a
 * field; Play Narrations, Use Timings, Show Media Controls and Use Presenter View are checkboxes. Hide Slide is
 * the generic toggle.
 *
 * **No dialog launchers**, because Office puts none here. **One survivor**, Hide Slide, in Set Up.
 */
export function powerpointSlideShowTab(options: TabOptions = {}): TemplateResult {
  const slideShow = entry('slide-show');
  const controls = options.controls ?? {};
  return tab(
    slideShow.id,
    slideShow.label,
    censusGroup(slideShow, 'GroupSlideShowStart', {}, controls),
    censusGroup(slideShow, 'GroupRehearse', {}, controls),
    censusGroup(slideShow, 'GroupSlideShowSetup', {}, controls),
    censusGroup(slideShow, 'GroupMonitors', {}, controls),
  );
}

/**
 * Recording: Record, Recording, Content, Camera, Auto-play Media, Edit, Save, Export, Preview, Help, in the
 * **census's declared** order.
 *
 * ⚠ **The census declares two generations of the tab**, the Recording tab Microsoft 365 has shipped since 2017 and
 * the newer recorder's Record tab, and Office never draws both. `GUESS:` that reading, and the order, since no
 * Office build draws the union; the declaration keeps the older tab's Record, Content, Auto-Play Media and Save
 * in Office's order. `dev/ribbons/census.ts` records every disagreement, including the one command drawn twice
 * (Record, beside From Beginning and From Current Slide).
 *
 * **Eight of the tab's fifteen commands are bound by the host**: Record and Cameo are split buttons; Screenshot,
 * Video, Audio, Clear Recording, Reset to Cameo and Export are dropdowns. All eight open menus from
 * `stories/ribbons/recording-menus.ts`, four of which call Insert's lists. Camera is declared by Insert's
 * `cameraCommands`.
 *
 * **No dialog launchers**, because Office puts none here, and **no survivor**.
 */
export function powerpointRecordingTab(options: TabOptions = {}): TemplateResult {
  const recording = entry('recording');
  const controls = options.controls ?? {};
  return tab(
    recording.id,
    recording.label,
    censusGroup(recording, 'GroupRecord', {}, controls),
    censusGroup(recording, 'GroupRecordTabRecord', {}, controls),
    censusGroup(recording, 'GroupContentRecording', {}, controls),
    censusGroup(recording, 'GroupChunkCameoCamera', {}, controls),
    censusGroup(recording, 'GroupAutoPlayMediaRecording', {}, controls),
    censusGroup(recording, 'GroupEditTabRecord', {}, controls),
    censusGroup(recording, 'GroupSaveRecording', {}, controls),
    censusGroup(recording, 'GroupExportTabRecord', {}, controls),
    censusGroup(recording, 'GroupPreviewTabRecord', {}, controls),
    censusGroup(recording, 'GroupHelpTabRecord', {}, controls),
  );
}

/**
 * Background Removal: Refine, Close — the first PowerPoint view tab authored, in **Office's** order, which is
 * also the census's.
 *
 * **Word's tab with PowerPoint's ids**: the census row calls `backgroundRemovalRefineCommands('powerpoint')` and
 * `backgroundRemovalCloseCommands('powerpoint')`, so the shape and every disagreement are `wordBackgroundRemovalTab`'s,
 * recorded once in `dev/ribbons/census.ts`. PowerPoint adds none.
 *
 * ⚠ **A view tab: Office shows it only while a picture's background is being removed**, so `powerpointTabs()`
 * leaves it out unless `includeViewTabs` is asked for.
 *
 * **Nothing is bound by the host**: two generic toggles in one exclusive set that may hold none, and two generic
 * buttons. **No menus, no dialog launchers, no survivors.**
 */
export function powerpointBackgroundRemovalTab(options: TabOptions = {}): TemplateResult {
  const backgroundRemoval = entry('background-removal');
  const controls = options.controls ?? {};
  return tab(
    backgroundRemoval.id,
    backgroundRemoval.label,
    censusGroup(backgroundRemoval, 'GroupBackgroundRemovalMode', {}, controls),
    censusGroup(backgroundRemoval, 'GroupBackgroundRemovalClose', {}, controls),
  );
}

/**
 * Print Preview: Print, Page Setup, Zoom, Preview — PowerPoint's second view tab authored, in **Office's**
 * order, which is also the census's.
 *
 * ⚠ **A view tab: Office shows it only inside Print Preview**, so `powerpointTabs()` leaves it out unless
 * `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks, which is why the tab's bindings and menus
 * are written there and nowhere else. `dev/ribbons/census.ts` records every disagreement, Colour/Greyscale's
 * group and Options' shape among them.
 *
 * **Four of the tab's ten commands are bound by the host**: Options and Orientation are dropdowns over
 * `stories/ribbons/print-preview-menus.ts`, and Print What and Colour/Greyscale are fields over its two option
 * lists. Everything else is the generic button.
 *
 * **No dialog launcher.** **Three survivors**: Fit to Window in Zoom, as on View, and Next Page and Previous
 * Page in Preview, as on Word's Print Preview.
 */
export function powerpointPrintPreviewTab(options: TabOptions = {}): TemplateResult {
  const printPreview = entry('print-preview');
  const controls = options.controls ?? {};
  return tab(
    printPreview.id,
    printPreview.label,
    censusGroup(printPreview, 'GroupPrintPreviewPrint', {}, controls),
    censusGroup(printPreview, 'GroupPrintPreviewPageSetup', {}, controls),
    censusGroup(printPreview, 'GroupZoom', {}, controls),
    censusGroup(printPreview, 'GroupPrintPreviewPreview', {}, controls),
  );
}

/**
 * Slide Master: Edit Master, Master Layout, Edit Theme, Background, Size, Close — PowerPoint's third view tab
 * authored, in **Office's** order, which is also the census's.
 *
 * ⚠ **A view tab: Office shows it only in Slide Master view**, so `powerpointTabs()` leaves it out unless
 * `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks, which is why the tab's bindings and menus are
 * written there and nowhere else. `dev/ribbons/census.ts` records every disagreement, where Colours, Fonts and
 * Effects sit among them.
 *
 * **Ten of the tab's eighteen commands are bound by the host**: Insert Placeholder is a split button; Themes,
 * Colours, Fonts, Effects, Background Styles and Slide Size are dropdowns over
 * `stories/ribbons/slide-master-menus.ts`; Title, Footers and Hide Background Graphics are checkboxes. Preserve is
 * the generic toggle, and everything else the generic button.
 *
 * **One dialog launcher, on Background**, which opens the Format Background pane. **No survivor.**
 */
export function powerpointSlideMasterTab(options: TabOptions = {}): TemplateResult {
  const slideMaster = entry('slide-master');
  const controls = options.controls ?? {};
  return tab(
    slideMaster.id,
    slideMaster.label,
    censusGroup(slideMaster, 'GroupMasterEdit', {}, controls),
    censusGroup(slideMaster, 'GroupMasterLayout', {}, controls),
    censusGroup(slideMaster, 'GroupMasterEditTheme', {}, controls),
    censusGroup(slideMaster, 'GroupBackground', { launcher: 'Format Background' }, controls),
    censusGroup(slideMaster, 'GroupSlideSize', {}, controls),
    censusGroup(slideMaster, 'GroupMasterClose', {}, controls),
  );
}

/**
 * Slide Master Home: Clipboard, Master Slides, Font, Paragraph, Drawing, Editing — PowerPoint's fourth view tab
 * authored, in **Office's** order, which is also the census's.
 *
 * ⚠ **The second tab labelled Home.** Office shows it only in Slide Master view, beside Slide Master, so
 * `powerpointTabs()` leaves it out unless `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks, which is
 * why its bindings are written there and nowhere else.
 *
 * **Five groups are Home's.** The census row calls Home's five shared functions with `'slide-master-home'`, so the
 * commands, their shapes and their survivors are `powerpointHomeTab`'s, and so are the four dialog launchers,
 * named as Home names them. **Master Slides is this tab's own**: Insert Slide Master and Insert Layout, then Layout,
 * Reset and Section. `dev/ribbons/census.ts` records every disagreement.
 *
 * **Eight commands are bound by the host**: Home's six (Paste, Font, Font size, Font colour, Shape styles and
 * Arrange), through the same helpers Home's bindings use, and Layout and Section as dropdowns over
 * `stories/ribbons/slide-master-menus.ts`. **Survivors**: Home's six, and none in Master Slides.
 */
export function powerpointSlideMasterHomeTab(options: TabOptions = {}): TemplateResult {
  const slideMasterHome = entry('slide-master-home');
  const controls = options.controls ?? {};
  return tab(
    slideMasterHome.id,
    slideMasterHome.label,
    censusGroup(slideMasterHome, 'GroupClipboard', { launcher: 'Clipboard settings' }, controls),
    censusGroup(slideMasterHome, 'GroupMasterSlides', {}, controls),
    censusGroup(slideMasterHome, 'GroupFont', { launcher: 'Font settings' }, controls),
    censusGroup(slideMasterHome, 'GroupParagraph', { launcher: 'Paragraph settings' }, controls),
    censusGroup(slideMasterHome, 'GroupDrawing', { launcher: 'Shape settings' }, controls),
    censusGroup(slideMasterHome, 'GroupEditing', {}, controls),
  );
}

/**
 * Handout Master: Page Setup, Placeholders, Edit Theme, Background, Close — PowerPoint's fifth view tab authored, in
 * **Office's** order, which is also the census's.
 *
 * ⚠ **A view tab: Office shows it only in Handout Master view**, so `powerpointTabs()` leaves it out unless
 * `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks, which is why the tab's bindings and menus are
 * written there and nowhere else. `dev/ribbons/census.ts` records every disagreement.
 *
 * **Edit Theme, Background and Close are Slide Master's**, from the census's master-view functions under this tab's
 * ids. **Page Setup and Placeholders are this tab's own.**
 *
 * **Thirteen of the tab's fourteen commands are bound by the host**: Handout Orientation, Slide Size, Slides Per Page,
 * Themes, Colours, Fonts, Effects and Background Styles are dropdowns over `stories/ribbons/slide-master-menus.ts`;
 * Header, Date, Footer, Page Number and Hide Background Graphics are checkboxes. Close Master View is the generic
 * button.
 *
 * **One dialog launcher, on Background**, which opens the Format Background pane. **No survivor.**
 */
export function powerpointHandoutMasterTab(options: TabOptions = {}): TemplateResult {
  const handoutMaster = entry('handout-master');
  const controls = options.controls ?? {};
  return tab(
    handoutMaster.id,
    handoutMaster.label,
    censusGroup(handoutMaster, 'GroupPageSetupHandoutMaster', {}, controls),
    censusGroup(handoutMaster, 'GroupPlaceholdersHandoutMaster', {}, controls),
    censusGroup(handoutMaster, 'GroupMasterEditTheme', {}, controls),
    censusGroup(handoutMaster, 'GroupBackground', { launcher: 'Format Background' }, controls),
    censusGroup(handoutMaster, 'GroupMasterClose', {}, controls),
  );
}

/**
 * Notes Master: Page Setup, Placeholders, Edit Theme, Background, Close — PowerPoint's sixth view tab authored, in
 * **Office's** order, which is also the census's.
 *
 * ⚠ **A view tab: Office shows it only in Notes Master view**, so `powerpointTabs()` leaves it out unless
 * `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks, which is why the tab's bindings and menus are
 * written there and nowhere else. `dev/ribbons/census.ts` records every disagreement.
 *
 * ⚠ **Background is this tab's `primary` group and Page Setup `standard`**, the census's priorities and the reverse
 * of Handout Master's.
 *
 * **Edit Theme, Background and Close are Slide Master's**, from the census's master-view functions under this tab's
 * ids. **Page Setup and Placeholders are this tab's own.**
 *
 * **Fourteen of the tab's fifteen commands are bound by the host**: Notes Page Orientation, Slide Size, Themes,
 * Colours, Fonts, Effects and Background Styles are dropdowns over `stories/ribbons/slide-master-menus.ts`; Header,
 * Slide Image, Footer, Date, Body, Page Number and Hide Background Graphics are checkboxes. Close Master View is the
 * generic button.
 *
 * **One dialog launcher, on Background**, which opens the Format Background pane. **No survivor.**
 */
export function powerpointNotesMasterTab(options: TabOptions = {}): TemplateResult {
  const notesMaster = entry('notes-master');
  const controls = options.controls ?? {};
  return tab(
    notesMaster.id,
    notesMaster.label,
    censusGroup(notesMaster, 'GroupPageSetupNotesMaster', {}, controls),
    censusGroup(notesMaster, 'GroupPlaceholdersNotesMaster', {}, controls),
    censusGroup(notesMaster, 'GroupMasterEditTheme', {}, controls),
    censusGroup(notesMaster, 'GroupBackground', { launcher: 'Format Background' }, controls),
    censusGroup(notesMaster, 'GroupMasterClose', {}, controls),
  );
}

/**
 * Black and White: Colour Mode, Close — PowerPoint's seventh view tab authored, in **Office's** order, which is also
 * the census's.
 *
 * ⚠ **A view tab: Office shows it only while the deck is previewed in black and white**, so `powerpointTabs()` leaves
 * it out unless `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks. `dev/ribbons/census.ts` records every
 * disagreement.
 *
 * **Both groups are the colour-mode tabs' shared functions** under this tab's ids, which Greyscale calls too.
 * **Colour Mode is ten toggles in one exclusive set**, Automatic pressed, and Close is one button. **Nothing is bound
 * by a host**: every command is the generic toggle or button. **No launcher, no menu, no survivor.**
 */
export function powerpointBlackAndWhiteTab(options: TabOptions = {}): TemplateResult {
  const blackAndWhite = entry('black-and-white');
  const controls = options.controls ?? {};
  return tab(
    blackAndWhite.id,
    blackAndWhite.label,
    censusGroup(blackAndWhite, 'GroupColorModeSetting', {}, controls),
    censusGroup(blackAndWhite, 'GroupColorModeClose', {}, controls),
  );
}

/**
 * Greyscale: Colour Mode, Close — PowerPoint's eighth and last view tab authored, in **Office's** order, which is also
 * the census's.
 *
 * ⚠ **A view tab: Office shows it only while the deck is previewed in greyscale**, so `powerpointTabs()` leaves it out
 * unless `includeViewTabs` is asked for. Only `Ribbons/PowerPoint` asks. `dev/ribbons/census.ts` records every
 * disagreement.
 *
 * **Both groups are Black and White's functions** under this tab's ids. **Colour Mode is its own exclusive set**,
 * `powerpoint.greyscale.colour-mode`, Automatic pressed, and Close is one button. **Nothing is bound by a host.**
 * **No launcher, no menu, no survivor.**
 */
export function powerpointGreyscaleTab(options: TabOptions = {}): TemplateResult {
  const greyscale = entry('greyscale');
  const controls = options.controls ?? {};
  return tab(
    greyscale.id,
    greyscale.label,
    censusGroup(greyscale, 'GroupColorModeSetting', {}, controls),
    censusGroup(greyscale, 'GroupColorModeClose', {}, controls),
  );
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

// ── the contextual tabs ──────────────────────────────────────────────────────

/**
 * Table Design: Table Style Options, Table Styles, WordArt Styles, Draw Borders — PowerPoint's first contextual tab
 * authored, in **Office's** order, which is also the census's. It sits under the *Table Tools* band.
 *
 * ⚠ **A contextual tab: Office shows it only while a table on a slide is selected.** `Ribbons/PowerPoint` draws every
 * contextual set and binds it; **`Shell/PowerPoint` draws Picture Tools alone**, so it binds none of this tab and
 * renders none of its menus, and `tests/ribbons.test.ts` refuses a shell that opens a menu of a set it never draws.
 * `dev/ribbons/census.ts` records every disagreement.
 *
 * **Fourteen of the tab's nineteen commands are bound by a host**: the six Table Style Options checkboxes; the Table
 * Styles and Quick Styles galleries; Shading, Text Fill, Text Outline and Pen Colour, colour pickers; Pen Style and
 * Pen Weight, fields. Borders, Effects and Text Effects open menus from `stories/ribbons/table-tools-menus.ts`, as a
 * split button and two dropdowns, which makes seventeen. Draw Table and Eraser are the generic toggles, one exclusive
 * set that may hold none.
 *
 * **WordArt Styles is the census's shared `wordArtStylesCommands`**, which Shape Format and Chart Format will call.
 *
 * **Two dialog launchers**: Format Text Effects on WordArt Styles and Format Shape on Draw Borders. **No survivor.**
 */
export function powerpointTableDesignTab(options: TabOptions = {}): TemplateResult {
  const tableDesign = entry('table-design');
  const controls = options.controls ?? {};
  return tab(
    tableDesign.id,
    tableDesign.label,
    censusGroup(tableDesign, 'GroupTableStyleOptionsPowerPoint', {}, controls),
    censusGroup(tableDesign, 'GroupTableStylesPowerPoint', {}, controls),
    censusGroup(tableDesign, 'GroupTextStylesTable', { launcher: 'Format Text Effects' }, controls),
    censusGroup(tableDesign, 'GroupDrawBorders', { launcher: 'Format Shape' }, controls),
  );
}

/**
 * Table Layout: Table, Rows & Columns, Merge, Cell Size, Alignment, Table Size, Arrange — PowerPoint's second
 * contextual tab authored, in **Office's** order, which is also the census's. It sits under the *Table Tools* band
 * beside Table Design, and its label is Office's *Layout*.
 *
 * ⚠ **A contextual tab: Office shows it only while a table on a slide is selected.** `Ribbons/PowerPoint` alone binds
 * it, for Table Design's reason: `Shell/PowerPoint` draws Picture Tools. `dev/ribbons/census.ts` records every
 * disagreement.
 *
 * **Twelve of the tab's twenty-eight commands are bound by a host**: Select, Delete, Text Direction and Cell Margins,
 * dropdowns that open their menus from `stories/ribbons/table-tools-menus.ts`; Bring Forward and Send Backward, split
 * buttons, and Align, a dropdown, over `stories/ribbons/design-layout-menus.ts`' Arrange lists; the two Height and
 * Width pairs, measure fields; Lock Aspect Ratio, a checkbox. Every other command is the generic toggle or button:
 * two exclusive sets of exactly one (the three horizontal and the three vertical alignments), View Gridlines and
 * Selection Pane, and seven buttons.
 *
 * **Arrange is the census's shared `arrangeCommands`**, for a table: no Group or Rotate.
 *
 * **No dialog launcher** (`GUESS:`). **Nine survivors in four groups**, each group's reason in the census.
 */
export function powerpointTableLayoutTab(options: TabOptions = {}): TemplateResult {
  const tableLayout = entry('table-layout');
  const controls = options.controls ?? {};
  return tab(
    tableLayout.id,
    tableLayout.label,
    censusGroup(tableLayout, 'GroupTable', {}, controls),
    censusGroup(tableLayout, 'GroupTableRowsAndColumns', {}, controls),
    censusGroup(tableLayout, 'GroupMerge', {}, controls),
    censusGroup(tableLayout, 'GroupTableCellSize', {}, controls),
    censusGroup(tableLayout, 'GroupAlignment', {}, controls),
    censusGroup(tableLayout, 'GroupTableSize', {}, controls),
    censusGroup(tableLayout, 'GroupArrange', {}, controls),
  );
}

/**
 * Picture Format: Adjust, Picture Styles, Accessibility, Arrange, Size, Image Play — PowerPoint's third contextual tab
 * authored and the seventh of all, in **Office's** order, which is also the census's. It sits under the *Picture Tools*
 * band.
 *
 * ⚠ **A contextual tab: Office shows it only while a picture on a slide is selected.** `Ribbons/PowerPoint` draws every
 * contextual set and binds it, and **`Shell/PowerPoint` draws Picture Tools alone and binds it too**, because its deck's
 * selection is a picture: it is PowerPoint's first contextual tab the shell shows authored. `dev/ribbons/census.ts`
 * records every disagreement.
 *
 * **Eighteen of the tab's twenty-three commands are bound by a host**: Corrections, Colour, Artistic Effects,
 * Transparency, Change Picture, Picture Effects, Convert to SmartArt, Align, Group and Rotate, dropdowns; Reset
 * Picture, Bring Forward and Send Backward, split buttons, and Crop, a split toggle; the Quick Styles gallery; Picture
 * Border, a colour picker with an Eyedropper beneath it; Height and Width, measure fields. Every menu is in
 * `stories/ribbons/picture-tools-menus.ts`. Remove Background and Compress Pictures are plain buttons, and Alt Text,
 * Selection Pane and Play Animation generic toggles.
 *
 * **Two dialog launchers**: Format Picture on Picture Styles, Size and Position on Size. **No survivor.**
 */
export function powerpointPictureFormatTab(options: TabOptions = {}): TemplateResult {
  const pictureFormat = entry('picture-format');
  const controls = options.controls ?? {};
  return tab(
    pictureFormat.id,
    pictureFormat.label,
    censusGroup(pictureFormat, 'GroupPictureTools', {}, controls),
    censusGroup(pictureFormat, 'GroupPictureStyles', { launcher: 'Format Picture' }, controls),
    censusGroup(pictureFormat, 'GroupAltText', {}, controls),
    censusGroup(pictureFormat, 'GroupArrangeWith3DEditor', {}, controls),
    censusGroup(pictureFormat, 'GroupPictureSize', { launcher: 'Size and Position' }, controls),
    censusGroup(pictureFormat, 'GroupImagePlay', {}, controls),
  );
}

/**
 * Shape Format: Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size — PowerPoint's fourth
 * contextual tab authored and the ninth of all, in **Office's** order, which is also the census's. It sits under the
 * *Drawing Tools* band.
 *
 * ⚠ **A contextual tab: Office shows it only while a shape, a text box or a WordArt on a slide is selected.**
 * `Ribbons/PowerPoint` draws every contextual set and binds it. **`Shell/PowerPoint` draws Picture Tools alone**,
 * because its deck's selection is a picture, so it binds none of this tab and renders none of its menus.
 * `dev/ribbons/census.ts` records every disagreement.
 *
 * **Eighteen of the tab's twenty-one commands are bound by the host**: Shapes, Edit Shape, Merge Shapes, Shape Effects,
 * Text Effects, Align, Group and Rotate, dropdowns; Bring Forward and Send Backward, split buttons; the Theme Styles and
 * Quick Styles galleries, the first with Other Theme Fills under it; Shape Fill, Shape Outline, Text Fill and Text
 * Outline, colour pickers; Height and Width, measure fields. Every menu is in `stories/ribbons/drawing-tools-menus.ts`.
 * Text Box is a plain button, and Alt Text and Selection Pane generic toggles.
 *
 * **Three dialog launchers**: Format Shape on Shape Styles, Format Text Effects on WordArt Styles, Size and Position on
 * Size. **No survivor.**
 */
export function powerpointShapeFormatTab(options: TabOptions = {}): TemplateResult {
  const shapeFormat = entry('shape-format');
  const controls = options.controls ?? {};
  return tab(
    shapeFormat.id,
    shapeFormat.label,
    censusGroup(shapeFormat, 'GroupShapes', {}, controls),
    censusGroup(shapeFormat, 'GroupShapeStyles', { launcher: 'Format Shape' }, controls),
    censusGroup(shapeFormat, 'GroupWordArtStyles', { launcher: 'Format Text Effects' }, controls),
    censusGroup(shapeFormat, 'GroupAltText', {}, controls),
    censusGroup(shapeFormat, 'GroupArrangeWith3DEditor', {}, controls),
    censusGroup(shapeFormat, 'GroupSize', { launcher: 'Size and Position' }, controls),
  );
}

/**
 * Chart Design: Chart Layouts, Chart Styles, Data, Type — PowerPoint's fifth contextual tab authored and the thirteenth
 * of all, in **Office's** order, which is also the census's. It sits under the *Chart Tools* band.
 *
 * ⚠ **A contextual tab: Office shows it only while a chart on a slide is selected.** `Ribbons/PowerPoint` draws every
 * contextual set and binds it; **`Shell/PowerPoint` draws Picture Tools alone**, so it binds none of this tab and
 * renders none of its menus. `dev/ribbons/census.ts` records every disagreement.
 *
 * **Word's Chart Design, under PowerPoint's ids.** Six of the nine commands are bound by the host: Add Chart Element,
 * Quick Layout, Change Colours and Change Chart Type, large dropdowns; Edit Data, a large split button; the Chart Styles
 * gallery. Every menu and the gallery's pictures are in `stories/ribbons/chart-tools-menus.ts`. Switch Row/Column,
 * Select Data and Refresh Data are plain large buttons.
 *
 * **No dialog launcher**, as Microsoft 365 draws none on this tab. **No survivor.**
 */
export function powerpointChartDesignTab(options: TabOptions = {}): TemplateResult {
  const chartDesign = entry('chart-design');
  const controls = options.controls ?? {};
  return tab(
    chartDesign.id,
    chartDesign.label,
    censusGroup(chartDesign, 'GroupChartLayouts', {}, controls),
    censusGroup(chartDesign, 'GroupChartStyles', {}, controls),
    censusGroup(chartDesign, 'GroupChartData', {}, controls),
    censusGroup(chartDesign, 'GroupChartType', {}, controls),
  );
}

/**
 * Which function builds which contextual tab. **Table Design, Table Layout, Picture Format, Shape Format and Chart
 * Design are authored; Chart Tools' Format is `placeholderTab` today** — see Word's.
 */
const contextualBuilders: Readonly<Record<string, (options: TabOptions) => TemplateResult>> = {
  'table-design': powerpointTableDesignTab,
  'table-layout': powerpointTableLayoutTab,
  'picture-format': powerpointPictureFormatTab,
  'shape-format': powerpointShapeFormatTab,
  'chart-design': powerpointChartDesignTab,
  'chart-format': () => placeholderTab(entry('chart-format')),
};

/**
 * **PowerPoint's contextual tab sets**, from the census. See `wordContextualSets`: `Ribbons/PowerPoint` draws every
 * built set, and `Shell/PowerPoint` names `picture-tools` alone, the set its deck's selection shows.
 */
export function powerpointContextualSets(options: ContextualSetOptions = {}): TemplateResult {
  return html`${contextualSetsFor(
    powerpointRibbonContextualSets,
    (declared) => {
      const build = contextualBuilders[declared.id];
      if (build === undefined) {
        throw new Error(`stories/ribbons/powerpoint.ts has no builder for the '${declared.id}' contextual tab`);
      }
      return build(options);
    },
    options,
  )}`;
}
