import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { largeGalleryItems } from '../gallery/specimens.ts';
import {
  documentThemePalette,
  machineFonts,
  recentColors,
  standardColors,
} from '../pickers/specimens.ts';
import {
  advanceAfterTimes,
  animationDelays,
  animationStarts,
  copyCounts,
  durationSeconds,
  openDeclaredSurface,
  printerList,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonKeyboard,
  ribbonNarrowFieldStyle,
  ribbonScreenReader,
  ribbonStatesMatrix,
  ribbonTokenDependencies,
  transitionSounds,
  type ControlOverrides,
} from './ribbon-parts.ts';
import {
  backgroundStyleEntries,
  designLayoutMenus,
  themeColourEntries,
  themeEffectEntries,
  themeFontEntries,
  themeGalleryItems,
  variantGalleryItems,
} from './design-layout-menus.ts';
import { drawMenus } from './draw-menus.ts';
import { insertMenus } from './insert-menus.ts';
import {
  followTransitionEffectOptions,
  referencesTransitionsFormulasMenus,
  startingTransition,
  transitionGalleryItems,
} from './references-transitions-formulas-menus.ts';
import {
  animationGalleryFooter,
  animationGalleryItems,
  mailingsAnimationsDataMenus,
  startingAnimation,
} from './mailings-animations-data-menus.ts';
import { powerpointContextualSets, powerpointTabs } from './powerpoint.ts';
import { powerpointPrintColourModes, powerpointPrintWhat, printPreviewMenus } from './print-preview-menus.ts';
import { recordingMenus } from './recording-menus.ts';
import { reviewMenus } from './review-menus.ts';
import { masterViewMenus } from './slide-master-menus.ts';
import { slideShowMenus, slideShowMonitors } from './slide-show-menus.ts';
import {
  powerpointPenStyles,
  powerpointTableStyleGalleryFooter,
  powerpointTableStyleGalleryItems,
  tableLineWeights,
  tableToolsMenus,
} from './table-tools-menus.ts';
import { viewMenus } from './view-menus.ts';
import { wordArtStyleGalleryFooter, wordArtStyleGalleryItems } from './wordart-styles-menus.ts';

/**
 * **PowerPoint's ribbon, tab by tab** — the same functions `Shell/PowerPoint` composes.
 *
 * Nineteen stories, because the census marks eighteen in-scope core tabs and decision 1 of the
 * approved plan makes File a nineteenth. **Eight of the eighteen are view tabs** — the two colour
 * modes, the four masters, Print Preview and Background Removal — which Office shows only inside
 * the view they name. The shell leaves them out and the catalogue does not, because a tab nobody
 * can look at cannot be audited.
 *
 * ⚠ **Two stories are called Home.** `Home` is the ordinary one; `SlideMasterHome` is Office's own
 * Home tab as it appears in Slide Master view, and they are never on screen together there. Here
 * they are, because every story renders every tab, and the duplicate label in the strip is the
 * catalogue's artefact rather than a transcription slip.
 *
 * **Six more stories are the contextual tabs.** **Table Design is authored**, PowerPoint's first. **The other five are
 * placeholders** at the census's own priorities: Layout, Picture Format, Shape Format, Chart Design and Format. Every
 * story draws all four contextual sets so each can be reached; `Shell/PowerPoint` draws Picture Tools alone, which is
 * why Table Design's bindings and menus are written here and nowhere else.
 *
 * **All nineteen core, view and File tabs are authored**: File, Home, Insert, Draw, Design, Transitions, Animations, Slide Show, Recording, Review, View, Background Removal, Print Preview, Slide Master, Slide Master Home, Handout Master, Notes Master, Black and White and Greyscale. See
 * `Ribbons/Word` for what to look at on a File tab, since the three are one tab with three sets of
 * differences rather than three tabs.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Ribbons/PowerPoint',
  parameters: {
    docs: {
      description: {
        component:
          'PowerPoint’s eighteen core tabs, its File tab and its six contextual tabs, each shown selected inside the ' +
          'whole ribbon. All nineteen core, view and File tabs are authored: File, Home, Insert, Draw, Design, Transitions, Animations, Slide Show, Recording, Review, View, Background Removal, Print Preview, Slide Master, Slide Master Home, Handout Master, Notes Master, Black and White and ' +
          'Greyscale. Of the contextual tabs of the four common sets, Table Design is authored; Layout, Picture ' +
          'Format, Shape Format, Chart Design and Format are placeholders carrying the census’s own priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * **Home's host controls, written once for the two tabs PowerPoint calls Home**: the ordinary Home and Slide Master
 * Home, whose Clipboard, Font and Drawing groups are the same census functions under two tab ids.
 *
 * Each binding below is still keyed by its literal command id, so `tests/ribbons.test.ts` reads every key as before;
 * what is shared is the markup. A control that carries a DOM `id` takes it as an argument, so the two tabs, which
 * this story renders at once, never put one id on the page twice. Both Paste split buttons open the one paste menu
 * `ribbon()` renders. Home passes the ids it always had, so its rendered output is unchanged.
 */
const homeBinding = {
  paste: (): TemplateResult => html`<mjx-split-button
    label="Paste"
    icon="clipboard-paste"
    size="large"
    menu-label="Paste options"
    data-opens="ribbons-ppt-paste"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  fontName: (id: string): TemplateResult => html`<mjx-font-picker
    id=${id}
    style=${ribbonFieldStyle}
    label="Font"
    value="Aptos"
    .fonts=${machineFonts}
  ></mjx-font-picker>`,
  fontSize: (id: string): TemplateResult => html`<mjx-dropdown
    id=${id}
    label="Font size"
    value="18"
    style=${ribbonNarrowFieldStyle}
  >
    ${['12', '14', '18', '24', '32', '44'].map(
      (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  fontColour: (id: string): TemplateResult => html`<mjx-color-picker
    id=${id}
    style=${ribbonColourFieldStyle}
    label="Font colour"
    show-automatic
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  shapeStyles: (id: string): TemplateResult => html`<mjx-gallery
    id=${id}
    label="Shape styles"
    value="office-1"
    style=${ribbonGalleryStyle}
  >
    ${largeGalleryItems().slice(0, 24)}
  </mjx-gallery>`,
  arrange: (): TemplateResult => html`<mjx-screentip
    heading="Arrange"
    description="Change how the selected shapes overlap one another, and how they line up."
    shortcut="Alt + J D A"
  >
    <mjx-button label="Arrange" icon="layer"></mjx-button>
  </mjx-screentip>`,
} as const;

/** The catalogue's own bindings. See `Ribbons/Word` on why these are not the shell's. */
const bindings: ControlOverrides = {
  'powerpoint.file.print.printer': html`<mjx-dropdown
    id="ribbons-powerpoint-printer"
    label="Printer"
    value="pdf"
    style=${ribbonFieldStyle}
  >
    ${printerList.map(
      (printer) => html`<mjx-option value=${printer.value} label=${printer.label}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'powerpoint.file.print.copies': html`<mjx-combo-box
    id="ribbons-powerpoint-copies"
    label="Copies"
    value="1"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${copyCounts.map((count) => html`<mjx-option value=${count} label=${count}></mjx-option>`)}
  </mjx-combo-box>`,
  'powerpoint.home.clipboard.paste': homeBinding.paste(),
  'powerpoint.home.font.name': homeBinding.fontName('ribbons-ppt-font'),
  'powerpoint.home.font.size': homeBinding.fontSize('ribbons-ppt-size'),
  'powerpoint.home.font.colour': homeBinding.fontColour('ribbons-ppt-colour'),
  'powerpoint.home.drawing.styles': homeBinding.shapeStyles('ribbons-ppt-shape-styles'),
  'powerpoint.home.drawing.arrange': homeBinding.arrange(),
  // Insert (unit 3). Office draws each of these as a dropdown or a split button, so each opens
  // its menu from `stories/ribbons/insert-menus.ts`: a dropdown is one `<mjx-button>` whose press
  // opens the menu, a split button opens it from its arrow. `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
  'powerpoint.insert.slides.new-slide': html`<mjx-split-button
    label="New Slide"
    icon="slide-add"
    size="large"
    data-opens="ribbons-powerpoint-insert-slides-new-slide"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.tables.table': html`<mjx-button
    label="Table"
    icon="table"
    size="large"
    data-opens="ribbons-powerpoint-insert-tables-table"
  ></mjx-button>`,
  'powerpoint.insert.images.pictures': html`<mjx-button
    label="Pictures"
    icon="image"
    size="large"
    data-opens="ribbons-powerpoint-insert-images-pictures"
  ></mjx-button>`,
  'powerpoint.insert.images.screenshot': html`<mjx-button
    label="Screenshot"
    icon="screenshot"
    size="large"
    data-opens="ribbons-powerpoint-insert-images-screenshot"
  ></mjx-button>`,
  'powerpoint.insert.images.photo-album': html`<mjx-split-button
    label="Photo Album"
    icon="image-multiple"
    size="large"
    data-opens="ribbons-powerpoint-insert-images-photo-album"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.illustrations.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-powerpoint-insert-illustrations-shapes"
  ></mjx-button>`,
  'powerpoint.insert.illustrations.3d-models': html`<mjx-button
    label="3D Models"
    icon="cube"
    size="large"
    data-opens="ribbons-powerpoint-insert-illustrations-3d-models"
  ></mjx-button>`,
  'powerpoint.insert.camera.cameo': html`<mjx-split-button
    label="Cameo"
    icon="camera"
    size="large"
    data-opens="ribbons-powerpoint-insert-camera-cameo"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.links.zoom': html`<mjx-button
    label="Zoom"
    size="small"
    data-opens="ribbons-powerpoint-insert-links-zoom"
  ></mjx-button>`,
  'powerpoint.insert.links.link': html`<mjx-split-button
    label="Link"
    icon="link"
    size="large"
    data-opens="ribbons-powerpoint-insert-links-link"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.text.wordart': html`<mjx-button
    label="WordArt"
    icon="text-effects"
    size="large"
    data-opens="ribbons-powerpoint-insert-text-wordart"
  ></mjx-button>`,
  'powerpoint.insert.symbols.equation': html`<mjx-split-button
    label="Equation"
    icon="math-formula"
    size="large"
    data-opens="ribbons-powerpoint-insert-symbols-equation"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.media-clips.video': html`<mjx-button
    label="Video"
    icon="video"
    size="large"
    data-opens="ribbons-powerpoint-insert-media-clips-video"
  ></mjx-button>`,
  'powerpoint.insert.media-clips.audio': html`<mjx-button
    label="Audio"
    icon="speaker-2"
    size="large"
    data-opens="ribbons-powerpoint-insert-media-clips-audio"
  ></mjx-button>`,
  // Draw (unit 4): Word's six bindings, over `stories/ribbons/draw-menus.ts`. See `Ribbons/Word`.
  'powerpoint.draw.drawing-tools.add-pen': html`<mjx-button
    label="Add Pen"
    size="small"
    data-opens="ribbons-powerpoint-draw-drawing-tools-add-pen"
  ></mjx-button>`,
  'powerpoint.draw.pens.pens': html`<mjx-button
    label="Pens"
    icon="inking-tool"
    size="large"
    data-opens="ribbons-powerpoint-draw-pens-pens"
  ></mjx-button>`,
  'powerpoint.draw.pens.colour': html`<mjx-button
    label="Colour"
    icon="color-line"
    size="small"
    data-opens="ribbons-powerpoint-draw-pens-colour"
  ></mjx-button>`,
  'powerpoint.draw.pens.thickness': html`<mjx-button
    label="Thickness"
    icon="line-thickness"
    size="small"
    data-opens="ribbons-powerpoint-draw-pens-thickness"
  ></mjx-button>`,
  'powerpoint.draw.write.eraser': html`<mjx-split-button
    toggle
    exclusive="powerpoint.draw.write.tools"
    label="Eraser"
    icon="eraser"
    size="large"
    data-opens="ribbons-powerpoint-draw-write-eraser"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.draw.input-mode.touch-mouse-mode': html`<mjx-button
    label="Touch/Mouse Mode"
    size="small"
    data-opens="ribbons-powerpoint-draw-input-mode-touch-mouse-mode"
  ></mjx-button>`,
  // Design (unit 5). Themes and Variants are in-ribbon galleries. Variants' four footer buttons open
  // menus with literal ids written beside this ribbon, because they are gallery footers rather than census
  // commands; Slide Size opens its menu from `stories/ribbons/design-layout-menus.ts`.
  'powerpoint.design.themes.themes': html`<mjx-gallery
    id="ribbons-ppt-themes"
    label="Themes"
    value="office-theme"
    style=${ribbonGalleryStyle}
  >
    ${themeGalleryItems()}
    <mjx-button slot="footer" label="Browse for Themes…"></mjx-button>
    <mjx-button slot="footer" label="Save Current Theme…"></mjx-button>
  </mjx-gallery>`,
  'powerpoint.design.variants.variants': html`<mjx-gallery
    id="ribbons-ppt-variants"
    label="Variants"
    value="variant-1"
    style=${ribbonGalleryStyle}
  >
    ${variantGalleryItems()}
    <mjx-button slot="footer" label="Colours" icon="color" data-opens="ribbons-ppt-variants-colours"></mjx-button>
    <mjx-button slot="footer" label="Fonts" icon="text-font" data-opens="ribbons-ppt-variants-fonts"></mjx-button>
    <mjx-button slot="footer" label="Effects" icon="square-shadow" data-opens="ribbons-ppt-variants-effects"></mjx-button>
    <mjx-button slot="footer" label="Background Styles" icon="color-background" data-opens="ribbons-ppt-variants-background-styles"></mjx-button>
  </mjx-gallery>`,
  'powerpoint.design.customise.slide-size': html`<mjx-button
    label="Slide Size"
    icon="slide-size"
    size="large"
    data-opens="ribbons-powerpoint-design-customise-slide-size"
  ></mjx-button>`,
  // Transitions (unit 6). The gallery is in-ribbon and starts on Fade, and Effect Options follows each commit: its
  // menu is re-rendered from `stories/ribbons/references-transitions-formulas-menus.ts`. Timing is fields over
  // `ribbon-parts.ts`'s lists: a duration is seconds, which a measure input does not carry, so it is a combo box.
  'powerpoint.transitions.transition-styles.transitions': html`<mjx-gallery
    id="ribbons-ppt-transitions"
    label="Transition to This Slide"
    value=${startingTransition}
    @mjx-gallery-commit=${followTransitionEffectOptions('ribbons')}
    style=${ribbonGalleryStyle}
  >
    ${transitionGalleryItems()}
  </mjx-gallery>`,
  'powerpoint.transitions.transition-styles.effect-options': html`<mjx-button
    label="Effect Options"
    size="small"
    data-opens="ribbons-powerpoint-transitions-transition-styles-effect-options"
  ></mjx-button>`,
  'powerpoint.transitions.timing.sound': html`<mjx-dropdown
    id="ribbons-ppt-transition-sound"
    label="Sound"
    value="no-sound"
    style=${ribbonColourFieldStyle}
  >
    ${transitionSounds.map(
      (sound) => html`<mjx-option value=${sound.value} label=${sound.label}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'powerpoint.transitions.timing.duration': html`<mjx-combo-box
    id="ribbons-ppt-transition-duration"
    label="Duration"
    value="00.70"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${durationSeconds.map((duration) => html`<mjx-option value=${duration} label=${duration}></mjx-option>`)}
  </mjx-combo-box>`,
  'powerpoint.transitions.timing.advance-slide': html`<mjx-label>Advance Slide</mjx-label>`,
  'powerpoint.transitions.timing.on-mouse-click': html`<mjx-checkbox id="ribbons-ppt-on-mouse-click" label="On Mouse Click" checked="true"></mjx-checkbox>`,
  'powerpoint.transitions.timing.after': html`<mjx-checkbox id="ribbons-ppt-advance-after-checkbox" label="After"></mjx-checkbox>`,
  'powerpoint.transitions.timing.advance-after': html`<mjx-combo-box
    id="ribbons-ppt-advance-after"
    label="Advance Slide After"
    value="00:00.00"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${advanceAfterTimes.map((time) => html`<mjx-option value=${time} label=${time}></mjx-option>`)}
  </mjx-combo-box>`,
  // Animations (unit 7). The gallery is in-ribbon, starts on Fly In and carries Office's footer. Preview's
  // split button, Effect Options, Add Animation and Trigger open their menus from
  // `stories/ribbons/mailings-animations-data-menus.ts`. Start, Duration and Delay are fields over `ribbon-parts.ts`'s lists.
  'powerpoint.animations.preview.preview': html`<mjx-split-button
    label="Preview"
    size="small"
    data-opens="ribbons-powerpoint-animations-preview-preview"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.animations.animations.animation-styles': html`<mjx-gallery
    id="ribbons-ppt-animation-styles"
    label="Animation Styles"
    value=${startingAnimation}
    style=${ribbonGalleryStyle}
  >
    ${animationGalleryItems()} ${animationGalleryFooter()}
  </mjx-gallery>`,
  'powerpoint.animations.animations.effect-options': html`<mjx-button
    label="Effect Options"
    size="small"
    data-opens="ribbons-powerpoint-animations-animations-effect-options"
  ></mjx-button>`,
  'powerpoint.animations.custom-animation.add-animation': html`<mjx-button
    label="Add Animation"
    icon="star-add"
    size="large"
    data-opens="ribbons-powerpoint-animations-custom-animation-add-animation"
  ></mjx-button>`,
  'powerpoint.animations.custom-animation.trigger': html`<mjx-button
    label="Trigger"
    icon="flash"
    size="small"
    data-opens="ribbons-powerpoint-animations-custom-animation-trigger"
  ></mjx-button>`,
  'powerpoint.animations.timing.start': html`<mjx-dropdown
    id="ribbons-ppt-animation-start"
    label="Start"
    value="on-click"
    style=${ribbonColourFieldStyle}
  >
    ${animationStarts.map((start) => html`<mjx-option value=${start.value} label=${start.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'powerpoint.animations.timing.duration': html`<mjx-combo-box
    id="ribbons-ppt-animation-duration"
    label="Duration"
    value="00.50"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${durationSeconds.map((duration) => html`<mjx-option value=${duration} label=${duration}></mjx-option>`)}
  </mjx-combo-box>`,
  'powerpoint.animations.timing.delay': html`<mjx-combo-box
    id="ribbons-ppt-animation-delay"
    label="Delay"
    value="00.00"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${animationDelays.map((delay) => html`<mjx-option value=${delay} label=${delay}></mjx-option>`)}
  </mjx-combo-box>`,
  // PowerPoint's Review. Split buttons and the Language dropdown open their menus from
  // `stories/ribbons/review-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Show Comments and Hide Ink are split buttons whose face is a toggle, both starting unpressed.
  'powerpoint.review.accessibility.check-accessibility': html`<mjx-split-button
    label="Check Accessibility"
    icon="accessibility-checkmark"
    size="small"
    data-opens="ribbons-powerpoint-review-accessibility-check-accessibility"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.review.language.language': html`<mjx-button
    label="Language"
    icon="local-language"
    size="large"
    data-opens="ribbons-powerpoint-review-language-language"
  ></mjx-button>`,
  'powerpoint.review.comments.delete': html`<mjx-split-button
    label="Delete"
    icon="comment-dismiss"
    size="large"
    data-opens="ribbons-powerpoint-review-comments-delete"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.review.comments.show-comments': html`<mjx-split-button
    toggle
    label="Show Comments"
    icon="comment-multiple"
    size="large"
    data-opens="ribbons-powerpoint-review-comments-show-comments"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.review.compare.accept': html`<mjx-split-button
    label="Accept"
    icon="document-checkmark"
    size="large"
    data-opens="ribbons-powerpoint-review-compare-accept"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.review.compare.reject': html`<mjx-split-button
    label="Reject"
    icon="document-dismiss"
    size="large"
    data-opens="ribbons-powerpoint-review-compare-reject"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.review.ink.hide-ink': html`<mjx-split-button
    toggle
    label="Hide Ink"
    size="small"
    data-opens="ribbons-powerpoint-review-ink-hide-ink"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  // View (PowerPoint's). Show's three are checkboxes, Ruler ticked as the census declares. Switch Windows
  // opens its menu from `stories/ribbons/view-menus.ts`, and `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`. The three exclusive sets are the generic toggles.
  'powerpoint.view.show.ruler': html`<mjx-checkbox id="ribbons-powerpoint-view-ruler" label="Ruler" checked="true"></mjx-checkbox>`,
  'powerpoint.view.show.gridlines': html`<mjx-checkbox id="ribbons-powerpoint-view-gridlines" label="Gridlines"></mjx-checkbox>`,
  'powerpoint.view.show.guides': html`<mjx-checkbox id="ribbons-powerpoint-view-guides" label="Guides"></mjx-checkbox>`,
  'powerpoint.view.window.switch-windows': html`<mjx-button
    label="Switch Windows"
    icon="window-multiple"
    size="large"
    data-opens="ribbons-powerpoint-view-window-switch-windows"
  ></mjx-button>`,
  // Slide Show. Present Online and Custom Slide Show are dropdowns and Record a split button, all opening
  // their menus from `stories/ribbons/slide-show-menus.ts`, with `data-opens` `commandSurfaceId('ribbons',
  // <this key>)`. Monitor is a field over `slideShowMonitors`, and the four checkboxes are ticked as the census
  // declares. Hide Slide is the generic toggle.
  'powerpoint.slide-show.start-slide-show.present-online': html`<mjx-button
    label="Present Online"
    icon="presenter"
    size="large"
    data-opens="ribbons-powerpoint-slide-show-start-slide-show-present-online"
  ></mjx-button>`,
  'powerpoint.slide-show.start-slide-show.custom-slide-show': html`<mjx-button
    label="Custom Slide Show"
    icon="slide-text-multiple"
    size="large"
    data-opens="ribbons-powerpoint-slide-show-start-slide-show-custom-slide-show"
  ></mjx-button>`,
  'powerpoint.slide-show.set-up.record': html`<mjx-split-button
    label="Record"
    icon="slide-record"
    size="large"
    data-opens="ribbons-powerpoint-slide-show-set-up-record"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.slide-show.set-up.play-narrations': html`<mjx-checkbox id="ribbons-powerpoint-slide-show-play-narrations" label="Play Narrations" checked="true"></mjx-checkbox>`,
  'powerpoint.slide-show.set-up.use-timings': html`<mjx-checkbox id="ribbons-powerpoint-slide-show-use-timings" label="Use Timings" checked="true"></mjx-checkbox>`,
  'powerpoint.slide-show.set-up.show-media-controls': html`<mjx-checkbox id="ribbons-powerpoint-slide-show-show-media-controls" label="Show Media Controls" checked="true"></mjx-checkbox>`,
  'powerpoint.slide-show.monitors.monitor': html`<mjx-dropdown
    id="ribbons-powerpoint-slide-show-monitor"
    label="Monitor"
    value="automatic"
    style=${ribbonColourFieldStyle}
  >
    ${slideShowMonitors.map((monitor) => html`<mjx-option value=${monitor.value} label=${monitor.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'powerpoint.slide-show.monitors.use-presenter-view': html`<mjx-checkbox id="ribbons-powerpoint-slide-show-use-presenter-view" label="Use Presenter View" checked="true"></mjx-checkbox>`,
  // Recording. Record and Cameo are split buttons; Screenshot, Video, Audio, Clear Recording, Reset to Cameo and
  // Export are dropdowns. All eight open their menus from `stories/ribbons/recording-menus.ts`, with `data-opens`
  // `commandSurfaceId('ribbons', <this key>)`. The other seven commands are generic buttons.
  'powerpoint.recording.recording.record': html`<mjx-split-button
    label="Record"
    icon="slide-record"
    size="large"
    data-opens="ribbons-powerpoint-recording-recording-record"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.recording.content.screenshot': html`<mjx-button
    label="Screenshot"
    icon="screenshot"
    size="large"
    data-opens="ribbons-powerpoint-recording-content-screenshot"
  ></mjx-button>`,
  'powerpoint.recording.camera.cameo': html`<mjx-split-button
    label="Cameo"
    icon="camera"
    size="large"
    data-opens="ribbons-powerpoint-recording-camera-cameo"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.recording.auto-play-media.video': html`<mjx-button
    label="Video"
    icon="video"
    size="large"
    data-opens="ribbons-powerpoint-recording-auto-play-media-video"
  ></mjx-button>`,
  'powerpoint.recording.auto-play-media.audio': html`<mjx-button
    label="Audio"
    icon="speaker-2"
    size="large"
    data-opens="ribbons-powerpoint-recording-auto-play-media-audio"
  ></mjx-button>`,
  'powerpoint.recording.edit.clear-recording': html`<mjx-button
    label="Clear Recording"
    icon="delete"
    size="large"
    data-opens="ribbons-powerpoint-recording-edit-clear-recording"
  ></mjx-button>`,
  'powerpoint.recording.edit.reset-to-cameo': html`<mjx-button
    label="Reset to Cameo"
    icon="arrow-reset"
    size="large"
    data-opens="ribbons-powerpoint-recording-edit-reset-to-cameo"
  ></mjx-button>`,
  'powerpoint.recording.export.export': html`<mjx-button
    label="Export"
    icon="arrow-export"
    size="large"
    data-opens="ribbons-powerpoint-recording-export-export"
  ></mjx-button>`,
  // Print Preview (a view tab). `Shell/PowerPoint` never draws a view tab, so these four bindings and the two
  // menus they open are written here and nowhere else. Options and Orientation open their menus from
  // `stories/ribbons/print-preview-menus.ts`; Print What and Colour/Greyscale are fields over its two lists.
  'powerpoint.print-preview.print.options': html`<mjx-button
    label="Options"
    icon="settings"
    size="large"
    data-opens="ribbons-powerpoint-print-preview-print-options"
  ></mjx-button>`,
  'powerpoint.print-preview.page-setup.print-what': html`<mjx-dropdown
    id="ribbons-powerpoint-print-preview-print-what"
    label="Print What"
    value="slides"
    style=${ribbonFieldStyle}
  >
    ${powerpointPrintWhat.map((shape) => html`<mjx-option value=${shape.value} label=${shape.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'powerpoint.print-preview.page-setup.orientation': html`<mjx-button
    label="Orientation"
    icon="orientation"
    size="small"
    data-opens="ribbons-powerpoint-print-preview-page-setup-orientation"
  ></mjx-button>`,
  'powerpoint.print-preview.page-setup.colour-greyscale': html`<mjx-dropdown
    id="ribbons-powerpoint-print-preview-colour-greyscale"
    label="Colour/Greyscale"
    value="colour"
    style=${ribbonFieldStyle}
  >
    ${powerpointPrintColourModes.map((mode) => html`<mjx-option value=${mode.value} label=${mode.label}></mjx-option>`)}
  </mjx-dropdown>`,
  // Slide Master (a view tab). `Shell/PowerPoint` never draws a view tab, so these ten bindings and the seven menus
  // they open are written here and nowhere else. Insert Placeholder is a split button; the six dropdowns open their
  // menus from `stories/ribbons/slide-master-menus.ts`; Title, Footers and Hide Background Graphics are checkboxes.
  'powerpoint.slide-master.master-layout.insert-placeholder': html`<mjx-split-button
    label="Insert Placeholder"
    icon="slide-content"
    size="large"
    menu-label="Placeholders"
    data-opens="ribbons-powerpoint-slide-master-master-layout-insert-placeholder"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.slide-master.master-layout.title': html`<mjx-checkbox id="ribbons-powerpoint-slide-master-title" label="Title" checked="true"></mjx-checkbox>`,
  'powerpoint.slide-master.master-layout.footers': html`<mjx-checkbox id="ribbons-powerpoint-slide-master-footers" label="Footers" checked="true"></mjx-checkbox>`,
  'powerpoint.slide-master.edit-theme.themes': html`<mjx-button
    label="Themes"
    icon="style-guide"
    size="large"
    data-opens="ribbons-powerpoint-slide-master-edit-theme-themes"
  ></mjx-button>`,
  'powerpoint.slide-master.edit-theme.colours': html`<mjx-button
    label="Colours"
    icon="color"
    size="small"
    data-opens="ribbons-powerpoint-slide-master-edit-theme-colours"
  ></mjx-button>`,
  'powerpoint.slide-master.edit-theme.fonts': html`<mjx-button
    label="Fonts"
    icon="text-font"
    size="small"
    data-opens="ribbons-powerpoint-slide-master-edit-theme-fonts"
  ></mjx-button>`,
  'powerpoint.slide-master.edit-theme.effects': html`<mjx-button
    label="Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-powerpoint-slide-master-edit-theme-effects"
  ></mjx-button>`,
  'powerpoint.slide-master.background.background-styles': html`<mjx-button
    label="Background Styles"
    icon="color-background"
    size="small"
    data-opens="ribbons-powerpoint-slide-master-background-background-styles"
  ></mjx-button>`,
  'powerpoint.slide-master.background.hide-background-graphics': html`<mjx-checkbox
    id="ribbons-powerpoint-slide-master-hide-background-graphics"
    label="Hide Background Graphics"
  ></mjx-checkbox>`,
  'powerpoint.slide-master.size.slide-size': html`<mjx-button
    label="Slide Size"
    icon="slide-size"
    size="large"
    data-opens="ribbons-powerpoint-slide-master-size-slide-size"
  ></mjx-button>`,
  // Slide Master Home (a view tab). Home's six bindings through `homeBinding`, with this tab's own DOM ids, and
  // Master Slides' two dropdowns over `stories/ribbons/slide-master-menus.ts`. `Shell/PowerPoint` binds none of them.
  'powerpoint.slide-master-home.clipboard.paste': homeBinding.paste(),
  'powerpoint.slide-master-home.master-slides.layout': html`<mjx-button
    label="Layout"
    icon="layout-row-two-split-top"
    size="small"
    data-opens="ribbons-powerpoint-slide-master-home-master-slides-layout"
  ></mjx-button>`,
  'powerpoint.slide-master-home.master-slides.section': html`<mjx-button
    label="Section"
    icon="slide-multiple"
    size="small"
    data-opens="ribbons-powerpoint-slide-master-home-master-slides-section"
  ></mjx-button>`,
  'powerpoint.slide-master-home.font.name': homeBinding.fontName('ribbons-ppt-slide-master-home-font'),
  'powerpoint.slide-master-home.font.size': homeBinding.fontSize('ribbons-ppt-slide-master-home-size'),
  'powerpoint.slide-master-home.font.colour': homeBinding.fontColour('ribbons-ppt-slide-master-home-colour'),
  'powerpoint.slide-master-home.drawing.styles': homeBinding.shapeStyles('ribbons-ppt-slide-master-home-shape-styles'),
  'powerpoint.slide-master-home.drawing.arrange': homeBinding.arrange(),
  // Handout Master (a view tab). `Shell/PowerPoint` never draws a view tab, so these thirteen bindings and the eight
  // menus they open are written here and nowhere else. Page Setup's three and Edit Theme's and Background's five are
  // dropdowns over `stories/ribbons/slide-master-menus.ts`; Placeholders' four and Hide Background Graphics are
  // checkboxes.
  'powerpoint.handout-master.page-setup.handout-orientation': html`<mjx-button
    label="Handout Orientation"
    icon="orientation"
    size="large"
    data-opens="ribbons-powerpoint-handout-master-page-setup-handout-orientation"
  ></mjx-button>`,
  'powerpoint.handout-master.page-setup.slide-size': html`<mjx-button
    label="Slide Size"
    icon="slide-size"
    size="large"
    data-opens="ribbons-powerpoint-handout-master-page-setup-slide-size"
  ></mjx-button>`,
  'powerpoint.handout-master.page-setup.slides-per-page': html`<mjx-button
    label="Slides Per Page"
    icon="layout-cell-four"
    size="large"
    data-opens="ribbons-powerpoint-handout-master-page-setup-slides-per-page"
  ></mjx-button>`,
  'powerpoint.handout-master.placeholders.header': html`<mjx-checkbox
    id="ribbons-powerpoint-handout-master-header"
    label="Header" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.handout-master.placeholders.date': html`<mjx-checkbox
    id="ribbons-powerpoint-handout-master-date"
    label="Date" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.handout-master.placeholders.footer': html`<mjx-checkbox
    id="ribbons-powerpoint-handout-master-footer"
    label="Footer" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.handout-master.placeholders.page-number': html`<mjx-checkbox
    id="ribbons-powerpoint-handout-master-page-number"
    label="Page Number" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.handout-master.edit-theme.themes': html`<mjx-button
    label="Themes"
    icon="style-guide"
    size="large"
    data-opens="ribbons-powerpoint-handout-master-edit-theme-themes"
  ></mjx-button>`,
  'powerpoint.handout-master.edit-theme.colours': html`<mjx-button
    label="Colours"
    icon="color"
    size="small"
    data-opens="ribbons-powerpoint-handout-master-edit-theme-colours"
  ></mjx-button>`,
  'powerpoint.handout-master.edit-theme.fonts': html`<mjx-button
    label="Fonts"
    icon="text-font"
    size="small"
    data-opens="ribbons-powerpoint-handout-master-edit-theme-fonts"
  ></mjx-button>`,
  'powerpoint.handout-master.edit-theme.effects': html`<mjx-button
    label="Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-powerpoint-handout-master-edit-theme-effects"
  ></mjx-button>`,
  'powerpoint.handout-master.background.background-styles': html`<mjx-button
    label="Background Styles"
    icon="color-background"
    size="small"
    data-opens="ribbons-powerpoint-handout-master-background-background-styles"
  ></mjx-button>`,
  'powerpoint.handout-master.background.hide-background-graphics': html`<mjx-checkbox
    id="ribbons-powerpoint-handout-master-hide-background-graphics"
    label="Hide Background Graphics"
  ></mjx-checkbox>`,  // Notes Master (a view tab). `Shell/PowerPoint` never draws a view tab, so these fourteen bindings and the seven
  // menus they open are written here and nowhere else. Page Setup's two and Edit Theme's and Background's five are
  // dropdowns over `stories/ribbons/slide-master-menus.ts`; Placeholders' six and Hide Background Graphics are
  // checkboxes.
  'powerpoint.notes-master.page-setup.notes-page-orientation': html`<mjx-button
    label="Notes Page Orientation"
    icon="orientation"
    size="large"
    data-opens="ribbons-powerpoint-notes-master-page-setup-notes-page-orientation"
  ></mjx-button>`,
  'powerpoint.notes-master.page-setup.slide-size': html`<mjx-button
    label="Slide Size"
    icon="slide-size"
    size="large"
    data-opens="ribbons-powerpoint-notes-master-page-setup-slide-size"
  ></mjx-button>`,
  'powerpoint.notes-master.placeholders.header': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-header"
    label="Header" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.notes-master.placeholders.slide-image': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-slide-image"
    label="Slide Image" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.notes-master.placeholders.footer': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-footer"
    label="Footer" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.notes-master.placeholders.date': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-date"
    label="Date" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.notes-master.placeholders.body': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-body"
    label="Body" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.notes-master.placeholders.page-number': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-page-number"
    label="Page Number" checked="true"
  ></mjx-checkbox>`,
  'powerpoint.notes-master.edit-theme.themes': html`<mjx-button
    label="Themes"
    icon="style-guide"
    size="large"
    data-opens="ribbons-powerpoint-notes-master-edit-theme-themes"
  ></mjx-button>`,
  'powerpoint.notes-master.edit-theme.colours': html`<mjx-button
    label="Colours"
    icon="color"
    size="small"
    data-opens="ribbons-powerpoint-notes-master-edit-theme-colours"
  ></mjx-button>`,
  'powerpoint.notes-master.edit-theme.fonts': html`<mjx-button
    label="Fonts"
    icon="text-font"
    size="small"
    data-opens="ribbons-powerpoint-notes-master-edit-theme-fonts"
  ></mjx-button>`,
  'powerpoint.notes-master.edit-theme.effects': html`<mjx-button
    label="Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-powerpoint-notes-master-edit-theme-effects"
  ></mjx-button>`,
  'powerpoint.notes-master.background.background-styles': html`<mjx-button
    label="Background Styles"
    icon="color-background"
    size="small"
    data-opens="ribbons-powerpoint-notes-master-background-background-styles"
  ></mjx-button>`,
  'powerpoint.notes-master.background.hide-background-graphics': html`<mjx-checkbox
    id="ribbons-powerpoint-notes-master-hide-background-graphics"
    label="Hide Background Graphics"
  ></mjx-checkbox>`,
  // Table Design (a contextual tab, in Table Tools). `Shell/PowerPoint` draws Picture Tools and not Table Tools, so
  // these seventeen bindings and `tableToolsMenus('powerpoint', …)` are written here alone: a binding in a shell that
  // never draws the tab would be a binding to nothing. The two galleries and the two fields are filled from
  // `stories/ribbons/table-tools-menus.ts` and `stories/ribbons/wordart-styles-menus.ts`, the four pickers and every
  // picture from the document's palette; Borders, Effects and Text Effects open that file's menus. Draw Table and
  // Eraser are the generic toggles, one exclusive set that may hold none, and are not bound.
  'powerpoint.table-design.table-style-options.header-row': html`<mjx-checkbox id="ribbons-powerpoint-table-design-header-row" label="Header Row" checked="true"></mjx-checkbox>`,
  'powerpoint.table-design.table-style-options.total-row': html`<mjx-checkbox id="ribbons-powerpoint-table-design-total-row" label="Total Row"></mjx-checkbox>`,
  'powerpoint.table-design.table-style-options.banded-rows': html`<mjx-checkbox id="ribbons-powerpoint-table-design-banded-rows" label="Banded Rows" checked="true"></mjx-checkbox>`,
  'powerpoint.table-design.table-style-options.first-column': html`<mjx-checkbox id="ribbons-powerpoint-table-design-first-column" label="First Column"></mjx-checkbox>`,
  'powerpoint.table-design.table-style-options.last-column': html`<mjx-checkbox id="ribbons-powerpoint-table-design-last-column" label="Last Column"></mjx-checkbox>`,
  'powerpoint.table-design.table-style-options.banded-columns': html`<mjx-checkbox id="ribbons-powerpoint-table-design-banded-columns" label="Banded Columns"></mjx-checkbox>`,
  'powerpoint.table-design.table-styles.gallery': html`<mjx-gallery
    id="ribbons-powerpoint-table-styles"
    label="Table Styles"
    value="medium-style-2-accent-1"
    style=${ribbonGalleryStyle}
  >
    ${powerpointTableStyleGalleryItems(documentThemePalette)} ${powerpointTableStyleGalleryFooter()}
  </mjx-gallery>`,
  'powerpoint.table-design.table-styles.shading': html`<mjx-color-picker
    id="ribbons-powerpoint-table-design-shading"
    style=${ribbonColourFieldStyle}
    label="Shading"
    show-no-fill
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'powerpoint.table-design.table-styles.borders': html`<mjx-split-button
    label="Borders"
    icon="border-all"
    size="small"
    menu-label="Borders"
    data-opens="ribbons-powerpoint-table-design-table-styles-borders"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.table-design.table-styles.effects': html`<mjx-button
    label="Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-powerpoint-table-design-table-styles-effects"
  ></mjx-button>`,
  'powerpoint.table-design.wordart-styles.quick-styles': html`<mjx-gallery
    id="ribbons-powerpoint-table-design-quick-styles"
    label="Quick Styles"
    style=${ribbonGalleryStyle}
  >
    ${wordArtStyleGalleryItems(documentThemePalette)} ${wordArtStyleGalleryFooter()}
  </mjx-gallery>`,
  'powerpoint.table-design.wordart-styles.text-fill': html`<mjx-color-picker
    id="ribbons-powerpoint-table-design-text-fill"
    style=${ribbonColourFieldStyle}
    label="Text Fill"
    value="theme:text1"
    show-no-fill
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'powerpoint.table-design.wordart-styles.text-outline': html`<mjx-color-picker
    id="ribbons-powerpoint-table-design-text-outline"
    style=${ribbonColourFieldStyle}
    label="Text Outline"
    show-no-fill
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'powerpoint.table-design.wordart-styles.text-effects': html`<mjx-button
    label="Text Effects"
    icon="text-effects"
    size="small"
    data-opens="ribbons-powerpoint-table-design-wordart-styles-text-effects"
  ></mjx-button>`,
  'powerpoint.table-design.draw-borders.pen-style': html`<mjx-dropdown
    id="ribbons-powerpoint-table-design-pen-style"
    label="Pen Style"
    value="solid"
    style=${ribbonFieldStyle}
  >
    ${powerpointPenStyles.map((penStyle) => html`<mjx-option value=${penStyle.value} label=${penStyle.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'powerpoint.table-design.draw-borders.pen-weight': html`<mjx-dropdown
    id="ribbons-powerpoint-table-design-pen-weight"
    label="Pen Weight"
    value="1"
    style=${ribbonNarrowFieldStyle}
  >
    ${tableLineWeights.map((weight) => html`<mjx-option value=${weight.value} label=${weight.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'powerpoint.table-design.draw-borders.pen-colour': html`<mjx-color-picker
    id="ribbons-powerpoint-table-design-pen-colour"
    style=${ribbonColourFieldStyle}
    label="Pen Colour"
    value="theme:text1"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
};

/**
 * The whole ribbon with one tab selected, and the paste menu the Home tab opens.
 *
 * ⚠ **No `<mjx-resizable-container>` of its own** — see `Ribbons/Word` for why a second one would
 * put a duplicate width control above every tab in this section.
 */
function ribbon(selected: string): TemplateResult {
  return html`
    <mjx-ribbon label="PowerPoint" selected=${selected} @mjx-activate=${openDeclaredSurface}>
      ${powerpointTabs({ controls: bindings, includeViewTabs: true })}
      ${powerpointContextualSets({ controls: bindings })}
    </mjx-ribbon>
    <mjx-menu id="ribbons-ppt-paste" label="Paste options" floating>
      <mjx-menu-section label="Paste">
        <mjx-menu-item kind="radio" label="Use Destination Theme" checked></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Keep Source Formatting"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Picture"></mjx-menu-item>
      </mjx-menu-section>
      <mjx-menu-separator></mjx-menu-separator>
      <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
    </mjx-menu>

    ${insertMenus('powerpoint', 'ribbons')} ${drawMenus('powerpoint', 'ribbons')}
    ${designLayoutMenus('powerpoint', 'ribbons')} ${referencesTransitionsFormulasMenus('powerpoint', 'ribbons')}
    ${mailingsAnimationsDataMenus('powerpoint', 'ribbons')} ${reviewMenus('powerpoint', 'ribbons')}
    ${viewMenus('powerpoint', 'ribbons')} ${slideShowMenus('powerpoint', 'ribbons')}
    ${recordingMenus('powerpoint', 'ribbons')} ${printPreviewMenus('powerpoint', 'ribbons')}
    ${masterViewMenus('powerpoint', 'ribbons')} ${tableToolsMenus('powerpoint', 'ribbons')}
    <mjx-menu id="ribbons-ppt-variants-colours" label="Colours" floating>${themeColourEntries()}</mjx-menu>
    <mjx-menu id="ribbons-ppt-variants-fonts" label="Fonts" floating>${themeFontEntries()}</mjx-menu>
    <mjx-menu id="ribbons-ppt-variants-effects" label="Effects" floating>${themeEffectEntries()}</mjx-menu>
    <mjx-menu id="ribbons-ppt-variants-background-styles" label="Background Styles" floating>${backgroundStyleEntries()}</mjx-menu>
  `;
}

/**
 * **File** — the same seven groups as Word's, and the one tab where PowerPoint's File page visibly
 * differs from the other two.
 *
 * **Export** carries *Create a Video*, *Package Presentation for CD* and *Create Handouts*: a deck
 * is the only document that can be played and the only one with a second shape to be printed in.
 * **Share** carries *Publish Slides*, which sends slides to a library one at a time rather than the
 * deck as a file.
 *
 * Two of those five carry **no icon** — Package Presentation for CD and Create Handouts — because
 * Fluent draws no CD at 20px and a handout is not a landscape page. That is the rule rather than an
 * oversight: a wrong icon is worse than a missing one, because a person acts on it.
 *
 * See `Ribbons/Word → File` for the collapse order, which is the same here.
 */
export const File: Story = { render: () => ribbon('file') };

/**
 * **Home** — six groups, and the ribbon programme's unit 2. See `Ribbons/Word → Home` for the two
 * rules that shape every Home tab: most of it is drawn icon-only because Office draws it that way,
 * and every state draws pressed while only a declared three per group survive a collapse. What is
 * PowerPoint's own:
 *
 * 1. **Slides arrives with this unit** — New Slide, Layout, Reset, Section — declared in the census
 *    since unit 0 and rendered by nothing, which meant this catalogue's PowerPoint had no way to
 *    add a slide. New Slide is the only `size="large"` command unit 2 adds anywhere.
 * 2. **Drawing is the census's largest Home group at 63 controls and draws six commands.** Sixty
 *    three is the shapes gallery's whole catalogue plus three effect menus and Arrange's fourteen
 *    entries; the face is Shapes, Arrange, the style gallery and the three shape formats. **It keeps
 *    no survivor**: all six open a gallery or a menu, and unit 2's survivor, Shapes, put a gallery
 *    inside the collapsed popup. Slides and Editing keep none for the same reason.
 * 3. **Arrange is drawn with `layer` now, not `slide-layout`** — which was the *Layout* command's
 *    icon, one group to the left, on a button that means something else entirely.
 * 4. **Text Shadow carries no icon**, so it is the one labelled toggle in a row of glyphs. Fluent
 *    draws no shadowed letter, and both candidates already name other commands in this subset. It
 *    draws pressed on shadowed text, as Strikethrough and Justify now do too, and it can never be a
 *    survivor: a survivor has no room for a label.
 */
export const Home: Story = { render: () => ribbon('home') };

/**
 * **Insert** — eleven groups, the most of any application's Insert tab, and the ribbon programme's
 * unit 3. See `Ribbons/Word → Insert` for the two rules that shape every Insert tab: a command that
 * opens something opens a real menu, and no group keeps a survivor because nearly every command
 * opens a surface. What is PowerPoint's own:
 *
 * 1. **A deck splits Word's Illustrations in two.** Pictures, Screenshot and Photo Album are the
 *    Images group, and Shapes, Icons, 3D Models, SmartArt and Chart are Illustrations — all large,
 *    where Word draws three of its seven small. Camera (Cameo) and Media (Video, Audio, Screen
 *    Recording) exist in no other application.
 * 2. **Fourteen of the twenty-eight commands open a menu.** New Slide's arrow is the layout list, as
 *    it is on Home; Zoom opens Summary, Section and Slide Zoom. **Text Box does not**: in PowerPoint
 *    it arms a drawing gesture rather than opening a gallery, so it is the plain button Office draws
 *    — and the gesture is also why it is not a survivor. **Symbol does not either**, although Word's
 *    does: PowerPoint's opens the Symbol dialog directly.
 * 3. **Two group labels are the census's and not Office's.** Office calls the media group *Media*;
 *    the census's `GroupInsertMediaClips` gives *Media Clips*. And **Content** is one control the
 *    census names without describing — it carries its own label and no icon, exactly as Excel's Power
 *    Options does on Home, and `dev/ribbons/census.ts` records what is and is not known.
 * 4. **Camera's and Content's positions are `GUESS:`** — each is drawn where the declaration puts it.
 * 5. **Five commands carry no icon**: Reuse Slides, Zoom, Object, Symbol and Content. Zoom is the one
 *    to look at: Fluent's magnifier is the status bar's view zoom, a different command.
 */
export const Insert: Story = { render: () => ribbon('insert') };

/**
 * **Draw**: nine groups and the ribbon programme's unit 4. See `Ribbons/Word → Draw` for what shapes
 * every Draw tab: two generations of Office's ink tools on one tab, one survivor (Select Objects),
 * and five tools of which one holds at a time, Eraser's split face among them. What is PowerPoint's own:
 *
 * 1. **Word's tab without two groups.** Editing (Ink Editor) and Drawing Canvas are Word's alone, so
 *    the strip goes straight from Stencils to Input Mode.
 * 2. **Every command is declared by the same function as Word's.** The census counts seven controls
 *    in Pens where Word counts six, and the same three commands are drawn. Nothing is padded to the
 *    count.
 * 3. **The same six commands open something**, and Eraser is the same split button, whose face is a
 *    toggle that draws pressed.
 */
export const Draw: Story = { render: () => ribbon('draw') };

/**
 * **Design**: three groups, and PowerPoint's part of the ribbon programme's unit 5. Themes, Variants and
 * Customise. What to look at:
 *
 * 1. **Themes and Variants are each one in-ribbon gallery**, which is Office's own shape for both
 *    groups. Themes holds nine of Office's themes by name. Variants holds the current theme's four.
 *    Each picture is a letter over four accent colours, drawn from the catalogue's one palette.
 * 2. ⚠ **Open the Variants flyout and press Colours at its foot.** Office opens Colours, Fonts, Effects
 *    and Background Styles from the bottom of that gallery, so they are footer buttons here, each
 *    opening its menu. Nobody has watched a menu open from inside an open flyout, so check that the
 *    menu appears beside the button and the flyout behaves. `dev/ribbons/census.ts` marks it `GUESS:`.
 * 3. **The Themes footer holds Browse for Themes and Save Current Theme**, which open dialogs in Office
 *    and do nothing here.
 * 4. **Customise is Slide Size and Format Background, both large.** Slide Size opens Standard,
 *    Widescreen and Custom Slide Size. Format Background opens a pane in Office, so it is a plain
 *    button. Check that *Format Background* fits its large button: *Background* is ten letters, and
 *    nobody has measured it.
 * 5. **Office's Designer group is not here**: the census marks it out of scope. Nothing survives a
 *    collapse, and Themes and Variants are the two primary groups.
 */
export const Design: Story = { render: () => ribbon('design') };

/**
 * **Transitions**: one gallery and the timing beside it, and PowerPoint's part of the ribbon programme's
 * unit 6. Three groups: Preview, Transition Styles and Timing. What to look at:
 *
 * 1. ⚠ **The group labels are the census's, and one does not match its id.** Transition Styles holds
 *    what Office calls *Transition to This Slide*: the gallery and Effect Options. Timing holds Office's
 *    Timing face, six commands where the census counts two. `dev/ribbons/census.ts` marks the reading
 *    `GUESS:`.
 * 2. **The gallery is in-ribbon, starts on Fade, and holds every transition Office shows** (unit 7
 *    completed it): None, then thirteen under *Subtle*, twenty-nine under *Exciting* and seven under
 *    *Dynamic Content*, in Office's order. Open its flyout to see the headings. Each picture is a
 *    pictogram of the motion drawn from the palette: an empty frame for None, a half-covered frame for
 *    Push and Wipe, bars for Cut. `GUESS:` Strips' place, last in Subtle.
 * 3. **Effect Options follows the transition.** It opens Fade's two, Smoothly checked and Through Black.
 *    Pick Push and it opens four edges; pick Split and it opens Vertical Out, Vertical In, Horizontal Out
 *    and Horizontal In; pick Flash and the button goes unavailable, because Office gives Flash no
 *    options. It carries no icon, so it is small where Office draws it large. `GUESS:` many transitions'
 *    entries, from memory of Office rather than a build.
 * 4. **Timing is five fields, a button and a heading.** Sound is a dropdown on *[No Sound]* over Office's
 *    whole list, down to *Other Sound…*. Duration is a combo box on 00.70: pick 01.00, or type 01.25.
 *    Then Apply To All, the *Advance Slide* heading, On Mouse Click ticked, After unticked, and the advance
 *    time on 00:00.00. ⚠ Office stacks these in two columns; the ribbon draws one row at full width, so
 *    the order is Office's and the columns are not. `GUESS:` Duration is a combo box rather than a
 *    measure input, because a measure input carries lengths and a duration is seconds.
 * 5. **Preview is large, with a slide-transition glyph**, and Apply To All is a plain labelled button.
 *    No dialog launchers. Nothing survives a collapse; Preview is ancillary and gives way first.
 */
export const Transitions: Story = { render: () => ribbon('transitions') };

/**
 * **Animations**: one gallery of effects, the tools that stack them, and their timing — PowerPoint's part
 * of the ribbon programme's unit 7. Four groups: Preview, Animations, Custom Animation and Timing. What to
 * look at:
 *
 * 1. ⚠ **Two group labels are the census's.** Office calls them *Animation* and *Advanced Animation*.
 * 2. **The gallery is in-ribbon, starts on Fly In, and holds every effect Office's does**: None, thirteen
 *    Entrance, nineteen Emphasis, thirteen Exit and six Motion Paths. Open the flyout: the headings are
 *    Office's, and the footer carries More Entrance Effects, More Emphasis Effects, More Exit Effects, More
 *    Motion Paths and a greyed OLE Action Verbs. Entrance stars are solid, Emphasis stars sit on a soft
 *    ground, Exit stars are outlined, and a motion path is a dashed line. `GUESS:` the pictures.
 * 3. **Effect Options is Fly In's**: eight directions with From Bottom checked, and three sequences with
 *    As One Object checked. ⚠ Unlike Transitions', it does not follow the gallery: pick Spin and it still
 *    offers Fly In's entries.
 * 4. **Preview is a split button** (Preview, and AutoPreview checked). Add Animation is large and opens
 *    the gallery's effects under the same headings. Trigger opens *On Click of* the slide's shapes.
 *    Animation Pane is a toggle, and Animation Painter a plain button, as Format Painter is.
 * 5. **Timing is three fields and two buttons.** Start is a dropdown on On Click, Duration a combo box on
 *    00.50 and Delay a combo box on 00.00. Move Earlier and Move Later carry arrows and labels. Nothing
 *    survives a collapse, and the one dialog launcher is on the Animations group.
 */
export const Animations: Story = { render: () => ribbon('animations') };

/**
 * **Slide Show**: the tab that plays the deck and says how it is played. Authored after the three View tabs,
 * one tab of one application. Four groups: Start Slide Show, Rehearse, Set Up and Monitors. What to look at,
 * least certain first:
 *
 * 1. ⚠ **Rehearse is `GUESS:` from end to end.** The census names the group, counts one control and names
 *    none, and declares it after Set Up. It is drawn second, as one large Rehearse with Coach (a figure
 *    speaking), where Microsoft 365 draws it. It opens nothing here.
 * 2. ⚠ **The three menus are from memory.** Present Online: Office Presentation Service, Skype for Business.
 *    Custom Slide Show: Custom Shows… alone, because the deck has saved none. Record's arrow: From Current
 *    Slide…, From Beginning…, then a *Clear* section of four. Press Record's face and nothing opens; press its
 *    arrow and the menu does.
 * 3. ⚠ **Hide Slide is the tab's only survivor.** Drag narrow until Set Up collapses: Hide Slide (large, a
 *    dashed slide) stays beside the trigger, and Set Up Slide Show, Rehearse Timings, Record and the three
 *    checkboxes open from it. Press Hide Slide and it draws pressed with a filled glyph. `GUESS:` that the
 *    dashed slide reads with no label.
 * 4. **Glyphs to judge**, all `GUESS:`: From Beginning's stacked slides with an arrow against From Current
 *    Slide's one slide with a play mark, Present Online's presenter (File's), Custom Slide Show's stacked
 *    slides, Set Up Slide Show's slide with a cog, Rehearse Timings' stopwatch and Record's slide with a record
 *    mark.
 * 5. **Monitors is a field and a checkbox.** Monitor reads *Automatic* and lists Automatic and Primary Monitor;
 *    Use Presenter View is ticked.
 * 6. **Set Up's three checkboxes are ticked**: Play Narrations, Use Timings, Show Media Controls, as a new deck
 *    has them. Tick one and it unticks.
 * 7. **Captions & Subtitles is not drawn**: the census marks it out of scope. No dialog launchers, no
 *    exclusive set.
 */
export const SlideShow: Story = { render: () => ribbon('slide-show') };

/**
 * **Recording**: the tab that records a deck with its narration and camera, and saves or exports what was
 * recorded. Authored after Slide Show, one tab of one application. Ten groups: Record, Recording, Content,
 * Camera, Auto-play Media, Edit, Save, Export, Preview and Help. What to look at, least certain first:
 *
 * 1. ⚠ **Recording, Edit, Export, Preview and Help are the newer recorder's groups, and every command in them is
 *    `GUESS:`.** The census names each group and counts its controls and names none. Recording is one large
 *    Record split button (a slide with a record mark), Edit is Clear Recording (a bin) and Reset to Cameo (a
 *    reset loop), Export is one Export dropdown, Preview one play mark, Help one question mark.
 * 2. ⚠ **Record is drawn twice in effect**: the Record group's From Beginning and From Current Slide, then the
 *    Recording group's Record, whose arrow lists From Current Slide… and From Beginning…. Office never draws
 *    both, because they are two generations of the tab; the census declares both groups.
 * 3. ⚠ **The menus written for this tab are from Microsoft's support wording.** Press each arrow or dropdown:
 *    Record lists From Current Slide…, From Beginning…; Clear Recording lists on Current Slide and on All
 *    Slides; Reset to Cameo the same pair; Export lists Export Video and Customize Export. Press Record's face
 *    and nothing opens.
 * 4. **Screenshot, Cameo, Video and Audio open Insert's own menus.** Compare with the Insert story: Screen
 *    Clipping; This Slide and All Slides; This Device…, Stock Videos…, Online Videos…; Audio on My PC…, Record
 *    Audio…. They should be identical.
 * 5. **Glyphs to judge**, all `GUESS:`: From Beginning and From Current Slide share Slide Show's glyphs; Save as
 *    Show draws a save with an arrow; Export to Video a film clip, which must read differently from Video's
 *    camera two groups before it; Export an arrow leaving a box.
 * 6. **Nothing survives a collapse.** Drag narrow: every group collapses to its trigger alone, and each popup
 *    holds every command in declared order.
 * 7. **The order is the census's declaration**, and the group labels *Recording* and *Auto-play Media* are
 *    the census's (Office writes Record and Auto-Play Media). No dialog launchers, no toggles.
 */
export const Recording: Story = { render: () => ribbon('recording') };

/**
 * **Review**: the tab where a deck is read by somebody else, authored after Word's and under the same
 * one-tab-one-application rule. Seven groups: Proofing, Accessibility, Language, Comments, Compare,
 * Activity and Ink. What to look at, least certain first:
 *
 * 1. ⚠ **Activity is one command, Show Changes, and all of it is `GUESS:`.** The census names the group
 *    and counts two controls, nothing more. It is a plain labelled button that opens nothing here.
 * 2. ⚠ **Ink is last, after Activity**, where Microsoft 365 draws it; the census declares it fifth,
 *    before Compare. `GUESS:` both positions. Insights (Smart Lookup) and Chinese Translation are out of
 *    scope in the census and are not drawn.
 * 3. **Show Comments and Hide Ink are split buttons whose face is a toggle, both starting unpressed.**
 *    Press Show Comments' face and it draws pressed with a filled glyph; press its arrow and Comments Pane
 *    and Show Markup (ticked) open, and the face does not move. Hide Ink's arrow: Hide Ink and Delete All
 *    Ink in Presentation. `GUESS:` both arrows' entries and both starting positions.
 * 4. **Compare is PowerPoint's own group, seven commands.** Compare, then Accept and Reject as large split
 *    buttons: Accept Change, Accept All Changes to This Slide, Accept All Changes to the Presentation, and
 *    Reject's three. Then Previous Change, Next Change and Reviewing Pane, a plain toggle with a right-hand
 *    pane glyph (`GUESS:`), then End Review. ⚠ Office greys all but Compare until a comparison is under way;
 *    they are drawn available here so the menus can be audited.
 * 5. **Previous Comment and Next Comment are the tab's only survivors.** Drag narrow until Comments
 *    collapses: the two comment arrows stay beside its trigger. They are small where Office draws them
 *    large, because every survivor in this catalogue is small.
 * 6. **The other menus.** Check Accessibility: Check Accessibility, Alt Text, Reading Order Pane, then
 *    Options: Accessibility. Language: Set Proofing Language, Language Preferences (Word's two). Delete:
 *    Delete, then all comments and ink on this slide, and in this presentation.
 * 7. **Labels, not glyphs**: Compare, Previous Change, Next Change, End Review, Show Changes and Hide Ink
 *    carry no icon. Spelling is large, unlike Word's Spelling & Grammar, because one word fits. Translate
 *    is a plain large button with no arrow. No dialog launchers.
 */
export const Review: Story = { render: () => ribbon('review') };

/**
 * **View**: how a deck is looked at, never the deck itself. Authored after Word's View, one tab of one
 * application. Seven groups: Presentation Views, Master Views, Show, Zoom, Colour/Greyscale, Window and View
 * Direction. What to look at, least certain first:
 *
 * 1. ⚠ **View Direction is `GUESS:` from end to end.** The census names the group and counts three controls,
 *    nothing more. It is drawn last as Left-to-Right (pressed) and Right-to-Left, small, with a letter and an
 *    arrow each way, as the group Office adds for right-to-left editing. Press Right-to-Left and
 *    Left-to-Right releases.
 * 2. ⚠ **Three exclusive sets, each independent.** Normal starts pressed: press Slide Sorter and Normal
 *    releases; press Slide Sorter again and it stays. Colour (pressed), Greyscale and Black and White do the
 *    same, and pressing Greyscale leaves the presentation view alone. Tab to a view and press Space: the same,
 *    by keyboard.
 * 3. ⚠ **Fit to Window is the tab's only survivor.** Drag narrow until Zoom collapses: Fit to Window (large, a
 *    landscape frame in fit corners) stays beside the trigger, and Zoom opens from it. `GUESS:` that the
 *    glyph reads with no label.
 * 4. **Glyphs to judge**, all `GUESS:`: Normal's window with a left pane, Outline View's stepped bars (Word's
 *    Outline), Slide Sorter's grid, Notes Page's notepad, Reading View's open book (Word's Read Mode), Slide
 *    Master's slide with a pencil, Handout Master's stacked pages, Notes Master's notepad with a pencil, Notes'
 *    bottom pane, Zoom's magnifier, Colour's palette, Greyscale's struck palette, Black and White's half
 *    circle, Arrange All's two columns, Cascade's stack and Move Split's four arrows. Toggles fill while pressed.
 * 5. **Three commands are small where Office draws them large**: Outline View, Notes Master and Notes, whose
 *    glyphs Fluent draws at 20 alone. Colour/Greyscale is spelt as the census spells it.
 * 6. **Show is three checkboxes, a toggle and the launcher.** Ruler ticked, Gridlines and Guides unticked;
 *    tick one and it ticks. Notes draws pressed when pressed. The launcher at Show's corner is *Grid
 *    Settings*, `GUESS:` its name.
 * 7. **Switch Windows is the tab's only menu**: one window, *1 Where the time went*, checked. Slide Master,
 *    Handout Master, Notes Master, Arrange All, Cascade and Move Split are plain buttons that open nothing.
 */
export const View: Story = { render: () => ribbon('view') };

/**
 * **Slide Master**: the master and its layouts, and the theme and background every slide inherits. A view tab
 * Office shows only in Slide Master view. Authored after Excel's Print Preview, one tab of one application, and
 * PowerPoint's third view tab. Six groups: Edit Master, Master Layout, Edit Theme, Background, Size and Close.
 * What to look at, least certain first:
 *
 * 1. ⚠ **Colours, Fonts and Effects are in Edit Theme**, small in a column beside a large Themes, as PowerPoint
 *    2010 draws them. `GUESS:` that Microsoft 365 draws them in Background instead; the census's count of 4 fits
 *    this placement.
 * 2. ⚠ **Every theme list is Design's, now whole.** Press Themes: *This Presentation* (Office Theme, checked), then
 *    *Office* with thirty themes from Facet to Wood Type, then Browse for Themes… and Save Current Theme…. Colours
 *    lists twenty-four sets and Customise Colours…; Fonts twenty pairs and Customise Fonts…; Effects fifteen.
 *    Background Styles lists Style 1 (checked) to Style 12, Format Background… and Reset Slide Background.
 *    `GUESS:` every list. Design's Themes gallery and Variants footer, Word's Design and Excel's Page Layout now
 *    draw the same lists.
 * 3. ⚠ **Insert Placeholder is a split button.** The face does nothing here; the arrow opens Content, Content
 *    (Vertical), Text, Text (Vertical), Picture, Chart, Table, SmartArt, Media, Online Image, none with a glyph.
 * 4. **Preserve is a toggle, unpressed.** Press it: the pin fills. Press again: it releases. `GUESS:` the start.
 * 5. **Three checkboxes**: Title and Footers ticked, stacked after Insert Placeholder; Hide Background Graphics
 *    unticked, under Background Styles. Tick one and it ticks.
 * 6. **The launcher at Background's corner is *Format Background*.** No other group has one.
 * 7. **Glyphs to judge**, all `GUESS:`: Insert Slide Master's slide with a title and a plus, Insert Layout's
 *    layout (Home's Layout glyph, now large), the bin, Rename's cursor in a field, Preserve's pin, Master Layout's
 *    slide with a title and a tick, Insert Placeholder's slide with a picture and lines, Themes' swatch book, then
 *    Design's palette, letters, shadowed square, paint bucket and resized frame, and the cross in a square.
 * 8. **No survivor anywhere.** Drag narrow: each group collapses to a trigger with nothing beside it, and every
 *    command opens from its popup.
 * 9. **Not in `Shell/PowerPoint`**: the shell's strip has no Slide Master tab, and no Slide Master menu is on that
 *    page.
 */
export const SlideMaster: Story = { render: () => ribbon('slide-master') };

/**
 * **Slide Master Home**: the *second* tab called Home (see this file's header), the one Slide Master view shows beside
 * Slide Master. A view tab. Authored after Slide Master, one tab of one application, and PowerPoint's fourth view tab.
 * Six groups: Clipboard, Master Slides, Font, Paragraph, Drawing and Editing. What to look at, least certain first:
 *
 * 1. ⚠ **Master Slides is the one group of its own**, second, where Home has Slides: Insert Slide Master and Insert
 *    Layout large, then Layout, Reset and Section small in a column. The census counts 9; five are drawn. `GUESS:`
 *    that reading of the count.
 * 2. ⚠ **Layout's glyph is new**: a frame split into a title row and two panes, not Home's Layout glyph, because
 *    Insert Layout beside it already wears that one. Compare the two: they should not read as one command twice.
 *    `GUESS:` the glyph.
 * 3. ⚠ **Layout and Section open menus here**, where Home's Layout and Section are plain buttons. Press Layout: an
 *    *Office Theme* section with eleven layouts, Title Slide to Vertical Title and Text, none ticked. Press Section:
 *    Add Section, Rename Section, Remove Section, Remove All Sections, a separator, Collapse All and Expand All.
 *    Office greys Section and Reset in master view; here they are available. `GUESS:` every entry and the greying.
 * 4. **Clipboard, Font, Paragraph, Drawing and Editing are Home's.** Switch between this story and `Home`: the five
 *    groups should match command for command, glyph for glyph, pressed state for pressed state (Bold and Align Left
 *    pressed), with the same four dialog launchers. Inspect a command: its id carries `slide-master-home`.
 * 5. **Home's six bindings are here, as their own controls.** Paste is a split button whose arrow opens the paste
 *    menu; the font picker, size, colour picker and Shape styles gallery work; Arrange shows its screentip. Change the
 *    font size here, then open `Home`: Home's field has not changed. Shapes stays a plain button, as on Home.
 * 6. **Survivors are Home's.** Drag narrow: Font keeps Bold, Italic and Underline beside its trigger, Paragraph
 *    keeps Align Left, Centre and Align Right, and Master Slides, Clipboard, Drawing and Editing keep nothing.
 * 7. **Glyphs to judge in Master Slides**: Slide Master's slide with a title and a plus, and its layout, then the new
 *    layout frame, Home's reset loop and Home's stacked slides.
 * 8. **Not in `Shell/PowerPoint`**: the shell's strip has one Home tab, the ordinary one, and no Layout or Section
 *    menu of this tab is on that page.
 */
export const SlideMasterHome: Story = { render: () => ribbon('slide-master-home') };

/**
 * **Handout Master**: the printed handout page, the slide frames laid out on it, and the header, date, footer and page
 * number around them. A view tab Office shows only in Handout Master view. Authored after Slide Master Home, one tab of
 * one application, and PowerPoint's fifth view tab. Five groups: Page Setup, Placeholders, Edit Theme, Background and
 * Close. What to look at, least certain first:
 *
 * 1. ⚠ **Page Setup draws three large dropdowns**, Handout Orientation, Slide Size and Slides Per Page, where the
 *    census counts 11. `GUESS:` that the census counts the eleven choices their menus offer.
 * 2. ⚠ **Slides Per Page's glyph** is a page divided into four frames, the glyph Excel's Arrange All draws for four
 *    tiled windows. Judge whether it reads as slides on a handout. Press it: 1 Slide, 2 Slides, 3 Slides, 4 Slides,
 *    6 Slides (checked), 9 Slides and Outline. `GUESS:` the glyph, the labels and the start.
 * 3. **Handout Orientation** opens Portrait (checked) and Landscape, Layout's list. **Slide Size** opens Standard
 *    (4:3), Widescreen (16:9, checked) and Custom Slide Size…, Design's list.
 * 4. **Four checkboxes in Placeholders**, Header and Date over Footer and Page Number, all ticked. Untick one and it
 *    unticks. `GUESS:` that all four start ticked.
 * 5. **Edit Theme, Background and Close are Slide Master's.** Switch between this story and `SlideMaster`: the three
 *    groups should match command for command, glyph for glyph and list for list, with the one *Format Background*
 *    launcher on Background. Inspect a command: its id carries `handout-master`. Tick Hide Background Graphics here,
 *    then open `SlideMaster`: that tab's checkbox has not changed. Office greys Themes here; it is drawn available.
 * 6. **No survivor anywhere.** Drag narrow: each group collapses to a trigger with nothing beside it, and every
 *    command opens from its popup.
 * 7. **Glyphs to judge**, all `GUESS:` and none new: the turning page, the resized frame, the four frames, then Slide
 *    Master's swatch book, palette, letters, shadowed square, paint bucket and cross in a square.
 * 8. **Not in `Shell/PowerPoint`**: the shell's strip has no Handout Master tab, and no Handout Master menu is on
 *    that page.
 */
export const HandoutMaster: Story = { render: () => ribbon('handout-master') };

/**
 * **Notes Master**: the printed notes page, the slide image at its top, the notes body under it, and the header,
 * date, footer and page number around them. A view tab Office shows only in Notes Master view. Authored after Handout
 * Master, one tab of one application, and PowerPoint's sixth view tab. Five groups: Page Setup, Placeholders, Edit
 * Theme, Background and Close. What to look at, least certain first:
 *
 * 1. ⚠ **Page Setup draws two large dropdowns**, Notes Page Orientation and Slide Size, where the census counts 3.
 *    `GUESS:` that the census counts Slide Size's face and arrow as two, as Slide Master's Size does. Handout
 *    Master's reading of its own Page Setup would give 4 here, so the two counts disagree with each other.
 * 2. ⚠ **Collapse order is the reverse of Handout Master's.** The census makes Background `primary` and Page Setup
 *    `standard` on this tab. Drag narrow: Close goes first, then Page Setup, Placeholders and Edit Theme, and
 *    Background last. Do the same on `HandoutMaster`: there Background goes before Page Setup. Judge whether that
 *    difference is right for Office.
 * 3. **Six checkboxes in Placeholders**: Header, Slide Image, Footer, Date, Body and Page Number, all ticked. Office
 *    draws two columns of three, Header, Slide Image and Footer down the first. Check the order they flow in here, and
 *    untick one: it unticks and no other does. `GUESS:` the order and that all six start ticked.
 * 4. **Notes Page Orientation** opens Portrait (checked) and Landscape, Layout's list. **Slide Size** opens Standard
 *    (4:3), Widescreen (16:9, checked) and Custom Slide Size…, Design's list. Both match `HandoutMaster`'s entry for
 *    entry.
 * 5. **Edit Theme, Background and Close are Slide Master's.** Switch between this story and `SlideMaster`: the three
 *    groups should match command for command, glyph for glyph and list for list, with the one *Format Background*
 *    launcher on Background. Inspect a command: its id carries `notes-master`. Tick Hide Background Graphics here,
 *    then open `HandoutMaster`: that tab's checkbox has not changed. Office greys Themes here; it is drawn available.
 * 6. **No survivor anywhere.** Drag narrow: each group collapses to a trigger with nothing beside it, and every
 *    command opens from its popup.
 * 7. **Glyphs to judge**, all `GUESS:` and none new: the turning page and the resized frame, then Slide Master's
 *    swatch book, palette, letters, shadowed square, paint bucket and cross in a square.
 * 8. **Not in `Shell/PowerPoint`**: the shell's strip has no Notes Master tab, and no Notes Master menu is on that
 *    page.
 */
export const NotesMaster: Story = { render: () => ribbon('notes-master') };

/**
 * **Black and White**: how the selected object prints on a black-and-white printer. A view tab Office shows only while
 * the deck is previewed in black and white, which View's *Black and White* opens. Authored after Notes Master, one tab
 * of one application, and PowerPoint's seventh view tab. Two groups: Colour Mode and Close. What to look at, least
 * certain first:
 *
 * 1. ⚠ **Colour Mode is ten toggles in one exclusive set, not a gallery.** Automatic starts pressed. Press Black: it
 *    fills and Automatic releases. Press Black again: it stays. Tab to Inverse Greyscale and press Space: the same,
 *    by keyboard. Judge whether a row of toggles reads as *choose one setting for this object*. Office greys the ten
 *    with nothing selected and highlights none for a mixed selection. Here one always holds. `GUESS:` Automatic as
 *    the start.
 * 2. ⚠ **Five settings carry no glyph and are small**: Automatic, Grey with White Fill, Black with Greyscale Fill,
 *    Black with White Fill, Black and White. Office draws coloured swatches for them. A one-tint glyph has no grey,
 *    and a pressed toggle draws Fluent's *filled* drawing, which would change a fill's meaning. Check that the small
 *    labels are readable in their columns and that Office's order survives: Automatic, three large, five small,
 *    Don't Show.
 * 3. ⚠ **Four large labels wrap unmeasured**: Light Greyscale, Inverse Greyscale, Don't Show and Back To Colour View
 *    (*Back To* over *Colour View*). Look for a clipped third line. `GUESS:` that each fits, by comparison with Word's
 *    Close Outline View.
 * 4. **Glyphs to judge**, all `GUESS:`: Greyscale's struck palette (View's Greyscale, now at 24), Light Greyscale's
 *    sun (new), Inverse Greyscale's half-dark circle (Word's Switch Modes), Don't Show's struck eye (Excel's Hide),
 *    and Back To Colour View's cross in a square, every view tab's close. Toggles fill while pressed.
 * 5. **The group is labelled Colour Mode**, the census's label, where Office writes *Change Selected Object*. Office's
 *    *Grayscale*, *Gray* and *Color* are spelt Greyscale, Grey and Colour.
 * 6. **The set is this tab's own.** Press White here, then open `View`: its Colour/Greyscale set has not changed.
 *    Open `Greyscale`: its set still holds Automatic.
 * 7. **No survivor.** Drag narrow: Close collapses first, then Colour Mode, and each collapses to a trigger with
 *    nothing beside it.
 * 8. **Not in `Shell/PowerPoint`**: the shell's strip has no Black and White tab.
 */
export const BlackAndWhite: Story = { render: () => ribbon('black-and-white') };

/**
 * **Greyscale**: how the selected object is drawn while the deck is previewed in greyscale. The census's `TabGrayscale`
 * under this catalogue's spelling, a view tab Office shows only in that preview, which View's *Greyscale* opens.
 * Authored after Black and White, one tab of one application, and PowerPoint's eighth and last view tab. Two groups:
 * Colour Mode and Close, **the same two functions as `BlackAndWhite`**, so its items 1 to 5 and 7 hold here unchanged.
 * What to look at, least certain first:
 *
 * 1. ⚠ **This tab's set is its own.** Press Black here: it fills and Automatic releases. Open `BlackAndWhite`: its
 *    set still holds Automatic. Come back: Black still holds. `GUESS:` that Office keeps the two apart; if it keeps
 *    one setting per shape, a document-backed host would press the same member on both tabs.
 * 2. ⚠ **Automatic starts pressed, as on Black and White.** `GUESS:` that Office starts a new shape on Automatic in
 *    greyscale too. Office greys the ten with nothing selected; here one always holds.
 * 3. **Compare it with `BlackAndWhite`.** The two tabs should look identical apart from the tab's name: same labels,
 *    sizes, glyphs and order, with Back To Colour View last. Any difference is a defect.
 * 4. **Check the spelling.** Office's *Grayscale*, *Gray* and *Back To Color View* are spelt Greyscale, Grey and
 *    Back To Colour View, and the group is labelled Colour Mode where Office writes *Change Selected Object*.
 * 5. **No survivor.** Drag narrow: Close collapses first, then Colour Mode, each to a trigger with nothing beside it.
 * 6. **Not in `Shell/PowerPoint`**: the shell's strip has no Greyscale tab.
 */
export const Greyscale: Story = { render: () => ribbon('greyscale') };

/**
 * **Print Preview**: the deck as it will print, in whichever printout shape is chosen, and a view tab Office
 * shows only inside Print Preview. Authored after Excel's Background Removal, one tab of one application, and
 * PowerPoint's second view tab. Four groups: Print, Page Setup, Zoom and Preview. What to look at, least certain
 * first:
 *
 * 1. ⚠ **Colour/Greyscale is a field in Page Setup, not a submenu of Options.** It sits under Print What with
 *    Orientation between them, and lists Colour (selected), Greyscale and Pure Black and White, in the census's
 *    spelling. `GUESS:` that Office 2007 had it inside Options instead; the brief and the census's count of 5
 *    put it here.
 * 2. ⚠ **Options opens a menu, not a dialog**, unlike Word's. Press it: Header and Footer…, then three unticked
 *    checkboxes (Scale to Fit Paper, Frame Slides, Print Comments and Ink Markup), a *Print Order* section with
 *    Horizontal checked and Vertical, and Print Hidden Slides unticked. No Colour/Greyscale entry. `GUESS:`
 *    every entry and tick.
 * 3. ⚠ **Print What lists nine shapes**: Slides (selected), Handouts at 1, 2, 3, 4, 6 and 9 slides per page,
 *    Notes Pages and Outline View. The longest label should fit the field or ellipsise inside it, not grow it.
 * 4. **Orientation is small** under Print What, and opens Layout's list: Portrait (checked), Landscape. Office
 *    greys it while Print What is Slides; here it is available. `GUESS:` the size.
 * 5. **Three survivors.** Drag narrow until Zoom collapses: Fit to Window stays beside the trigger and Zoom opens
 *    from it. Until Preview collapses: Next Page and Previous Page stay, and Close Print Preview opens from it.
 * 6. **Glyphs, every one a glyph the subset already carries for the same command**: a printer, a cog, the
 *    turning page, a magnifier, the landscape frame in fit corners, a page with an arrow down and up, and a cross
 *    in a square. **No dialog launcher** on any group. *Close Print Preview* should wrap without an ellipsis.
 * 7. **Not in `Shell/PowerPoint`**: the shell's strip has no Print Preview tab, and no Print Preview menu is on
 *    that page.
 */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/**
 * **Background Removal**: the two pencils that correct Office's guess at a picture's background, and the two
 * ways out. A view tab Office shows only while a picture's background is being removed. Authored after Word's,
 * one tab of one application, from the same census functions under PowerPoint's ids. Two groups: Refine and
 * Close. What to look at, least certain first:
 *
 * 1. ⚠ **The pencils hold at most one, and start with neither.** Press Mark Areas to Keep: it fills. Press
 *    Mark Areas to Remove: it fills and Keep releases. Press Remove again: it releases, and neither is
 *    pressed. `GUESS:` the release on a second press and the empty start; View's three sets, by contrast,
 *    keep one pressed.
 * 2. **The set is PowerPoint's own.** Inspect a pencil: its `exclusive` attribute is
 *    `powerpoint.background-removal.refine`, not Word's.
 * 3. ⚠ **Refine has two commands, not three.** Delete Mark, which Office 2010 to 2016 drew, is absent because
 *    Microsoft 365 no longer draws it. `GUESS:`.
 * 4. ⚠ **Four circles**: a plus and a minus for the pencils, a cross and a tick for Discard All Changes and
 *    Keep Changes. `GUESS:` every glyph, as on Word's.
 * 5. **All four are large, and the long labels wrap.** *Mark Areas to Remove* and *Discard All Changes* should
 *    wrap to two lines without an ellipsis.
 * 6. **No survivors.** Drag narrow until both groups collapse: each popup trigger stands alone and opens its
 *    commands in order. The pencils' set still holds one at most when pressed inside the popup.
 * 7. **Not in `Shell/PowerPoint`**: the shell's strip has no Background Removal tab. No menus, no dialog
 *    launchers.
 */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };

/**
 * **Table Design**: the style a table on a slide wears, the WordArt its text wears, and the pen Draw Table draws with.
 * Table Tools' first tab, and PowerPoint's first contextual tab authored; Office shows it only while a table is
 * selected. Four groups: Table Style Options, Table Styles, WordArt Styles and Draw Borders. See `Ribbons/Word → Table
 * Design` for the band and the collapse order, which hold on every contextual story. What to look at, least certain
 * first:
 *
 * 1. ⚠ **Text Fill, Text Outline, Shading and Pen Colour are colour fields**, where Office draws a small button with a
 *    coloured bar. Open each: the document's theme colours, standard colours and recent colours, with *No fill* on
 *    the first three. **Office's Eyedropper, More Colours, Picture, Gradient and Texture are not there, and nor are
 *    Text Outline's Weight, Sketched and Dashes.** Judge whether that loss is acceptable; it is the weakest part.
 * 2. ⚠ **The Quick Styles pictures.** Expand Quick Styles: twenty letters *A*, four rows of five, drawn in the
 *    document's palette, with Clear WordArt under them. Judge whether the shadow, glow, bevel, reflection, gradient
 *    and pattern styles read as different styles. Nothing is selected: text wears no WordArt. `GUESS:` every name and
 *    picture.
 * 3. ⚠ **The Table Styles pictures and sections.** Expand Table Styles: *Best Match for Document* (14: the No Style
 *    and Themed styles), *Light* (21), *Medium* (28) and *Dark* (11), one family to a row of seven, **Medium Style 2 -
 *    Accent 1 selected**. No style appears twice. Under the list: Clear Table. `GUESS:` the Best Match reading and
 *    every picture.
 * 4. ⚠ **Draw Table and Eraser are one set that may hold none.** Neither starts pressed. Press Draw Table, then
 *    Eraser: Draw Table releases. Press Eraser again: it releases and neither holds.
 * 5. **Effects and Text Effects open menus of submenus.** Effects: Cell Bevel (No Bevel and twelve bevels), Shadow (No
 *    Shadow, Outer nine, Inner nine, Perspective five, Shadow Options…) and Reflection (No Reflection, nine
 *    variations, Reflection Options…). Text Effects: Shadow and Reflection again, then Glow (twenty-four variations,
 *    More Glow Colours, Glow Options…), Bevel (ending in 3-D Options…), 3-D Rotation (Parallel, Perspective, Oblique)
 *    and Transform (Follow Path four, Warp thirty-two). Hover an entry with a submenu to open it.
 * 6. **Borders is a split button**, small. The face does nothing here; the arrow opens twelve entries, No Border and
 *    All Borders first, then Outside, Inside, the four edges, the two inside lines and the two diagonals.
 * 7. **Six checkboxes, two columns of three**: Header Row, Total Row and Banded Rows down the first; First Column, Last
 *    Column and Banded Columns down the second. **Header Row and Banded Rows are ticked**, PowerPoint's look for an
 *    inserted table, unlike Word's, which also ticks First Column. `GUESS:` the start.
 * 8. **Pen Style and Pen Weight are fields with names.** Pen Style lists No Border and eight dashes, Solid selected;
 *    Pen Weight lists ¼ pt to 6 pt, 1 pt selected. Pen Colour starts on Text 1.
 * 9. **Two launchers**: *Format Text Effects* at WordArt Styles' corner and *Format Shape* at Draw Borders'. `GUESS:`
 *    both, and that Table Style Options and Table Styles have none.
 * 10. **No survivor anywhere.** Drag narrow: each group collapses to a trigger with nothing beside it. The collapse
 *     order is the census's: Table Style Options, WordArt Styles and Draw Borders are `standard` and give way first,
 *     Table Styles `primary` last.
 * 11. **Glyphs to judge**, all reused and all `GUESS:`: Borders' grid, Home's Borders; Effects' shadowed square, Home's
 *     Shape Effects; Text Effects' outlined letter, Insert's WordArt; Draw Table's pencil and Eraser, Word's Table
 *     Layout pair, filled when pressed.
 * 12. **Not in `Shell/PowerPoint`**: its strip draws Picture Tools, so there is no Table Design tab and none of its
 *     menus is on that page.
 */
export const TableDesign: Story = { render: () => ribbon('table-design') };

/**
 * **Layout** — Table Tools' second tab, a placeholder. Seven groups: Table, Rows & Columns, Merge, Cell Size,
 * Alignment, Table Size and Arrange. PowerPoint's has no Draw or Data group and does have Table Size and Arrange,
 * because a slide table is a shape on a canvas rather than a run of text.
 */
export const TableLayout: Story = { render: () => ribbon('table-layout') };

/**
 * **Picture Format** — a placeholder. Six groups: Adjust, Picture Styles, Accessibility, Arrange, Size and Image
 * Play, with Adjust and Picture Styles primary.
 */
export const PictureFormat: Story = { render: () => ribbon('picture-format') };

/**
 * **Shape Format** — a placeholder. Six groups: Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange
 * and Size, with Shape Styles primary.
 */
export const ShapeFormat: Story = { render: () => ribbon('shape-format') };

/**
 * **Chart Design** — a placeholder. Four groups: Chart Layouts, Chart Styles, Data and Type, with Chart Layouts
 * primary.
 */
export const ChartDesign: Story = { render: () => ribbon('chart-design') };

/**
 * **Format** — Chart Tools' second tab, a placeholder. Seven groups: Current Selection, Insert Shapes, Shape Styles,
 * WordArt Styles, Accessibility, Arrange and Size.
 */
export const ChartFormat: Story = { render: () => ribbon('chart-format') };
