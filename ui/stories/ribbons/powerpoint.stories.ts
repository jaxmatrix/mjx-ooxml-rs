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
import { reviewMenus } from './review-menus.ts';
import { viewMenus } from './view-menus.ts';

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
 * **File, Home, Insert, Draw, Design, Transitions, Animations, Review and View** are authored; the rest are placeholders at the census's own priorities. See
 * `Ribbons/Word` for why a placeholder says so on its face — and for what to look at on a File tab,
 * since the three are one tab with three sets of differences rather than three tabs.
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
          'PowerPoint’s eighteen core tabs and its File tab, each shown selected inside the whole ' +
          'ribbon. File, Home, Insert, Draw, Design, Transitions, Animations, Review and View are authored; the rest are placeholders carrying the ' +
          'census’s priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

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
  'powerpoint.home.clipboard.paste': html`<mjx-split-button
    label="Paste"
    icon="clipboard-paste"
    size="large"
    menu-label="Paste options"
    data-opens="ribbons-ppt-paste"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.home.font.name': html`<mjx-font-picker
    id="ribbons-ppt-font"
    style=${ribbonFieldStyle}
    label="Font"
    value="Aptos"
    .fonts=${machineFonts}
  ></mjx-font-picker>`,
  'powerpoint.home.font.size': html`<mjx-dropdown
    id="ribbons-ppt-size"
    label="Font size"
    value="18"
    style=${ribbonNarrowFieldStyle}
  >
    ${['12', '14', '18', '24', '32', '44'].map(
      (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'powerpoint.home.font.colour': html`<mjx-color-picker
    id="ribbons-ppt-colour"
    style=${ribbonColourFieldStyle}
    label="Font colour"
    show-automatic
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'powerpoint.home.drawing.styles': html`<mjx-gallery
    id="ribbons-ppt-shape-styles"
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
    <mjx-button label="Arrange" icon="layer"></mjx-button>
  </mjx-screentip>`,
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
      ${powerpointContextualSets()}
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
    ${viewMenus('powerpoint', 'ribbons')}
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

/** Unit 10. */
export const SlideShow: Story = { render: () => ribbon('slide-show') };

/** Unit 10. */
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

/** Unit 10, and a view tab: Office shows it only in Slide Master view. */
export const SlideMaster: Story = { render: () => ribbon('slide-master') };

/** Unit 10, and the *second* tab called Home — see this file's header. */
export const SlideMasterHome: Story = { render: () => ribbon('slide-master-home') };

/** Unit 10, and a view tab. */
export const HandoutMaster: Story = { render: () => ribbon('handout-master') };

/** Unit 10, and a view tab. */
export const NotesMaster: Story = { render: () => ribbon('notes-master') };

/** A view tab: Office shows it only while a deck is being previewed in black and white. */
export const BlackAndWhite: Story = { render: () => ribbon('black-and-white') };

/** A view tab, and the census's `TabGrayscale` under this catalogue's spelling. */
export const Greyscale: Story = { render: () => ribbon('greyscale') };

/** Unit 10, and a view tab. */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
