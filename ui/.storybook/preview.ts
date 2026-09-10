import { html } from 'lit';
import type { Decorator, Preview } from '@storybook/web-components-vite';

import '../tokens/tokens.css';
import { applyColorScheme, type SchemePreference } from '../src/tokens/resolver.ts';
import {
  containerPresetOrder,
  containerPresets,
  defineResizableContainer,
  type ContainerPreset,
} from '../src/harness/resizable-container.ts';
import { defineProbes } from '../dev/probes.ts';
import { defineFoundationProbes } from '../dev/foundation-probes.ts';
import { defineIcon } from '../src/icons/icon.ts';
import { defineSurface } from '../src/foundations/surface.ts';
import { defineControls } from '../src/controls/index.ts';
import { defineRibbonElements } from '../src/ribbon/index.ts';
import { defineMenus } from '../src/menus/index.ts';
import { defineGalleryElements } from '../src/gallery/index.ts';
import { defineInputs, inputDocumentCss } from '../src/inputs/index.ts';
import { definePickers } from '../src/pickers/index.ts';
import { defineSurfaces } from '../src/surfaces/index.ts';
import { surfaceDocumentCss } from '../src/surfaces/surface-model.ts';
import { defineFeedback } from '../src/feedback/index.ts';
import { defineFurniture } from '../src/furniture/index.ts';
import { defineNavigators } from '../src/navigators/index.ts';
import { defineFormulaChrome } from '../src/formula/index.ts';
import { defineAnnotation } from '../src/annotation/index.ts';
import { defineMobile } from '../src/mobile/index.ts';
import { mobileDocumentCss } from '../src/mobile/mobile-sheets.ts';
import { annotationDocumentCss } from '../src/annotation/annotation-sheets.ts';
import { formulaDocumentCss } from '../src/formula/formula-sheets.ts';
import { furnitureDocumentCss } from '../src/furniture/furniture-model.ts';
import { feedbackDocumentCss } from '../src/feedback/feedback-model.ts';
import { galleryDocumentCss } from '../src/gallery/gallery-model.ts';
import { installFoundations } from '../src/foundations/stylesheet.ts';
import type { StoryConventions } from '../src/story/conventions.ts';

defineResizableContainer();
defineProbes();
defineFoundationProbes();
defineIcon();
defineSurface();
// MJXOFF-182's four archetypes. Registered here rather than in each story file for the reason
// `<mjx-icon>` is: a component that is only defined by the story that happens to import it is a
// component whose absence looks like a rendering bug in a *different* story.
defineControls();
// MJXOFF-183's ribbon structure, registered here for the same reason.
defineRibbonElements();
// MJXOFF-184's menus. An unregistered <mjx-menu-item> is an inert element with no role, so a menu
// that was only defined by the story importing it would announce nothing in every other story.
defineMenus();
// MJXOFF-185's gallery. `<mjx-gallery-item>` is a *descriptor* whose art is still in the light DOM
// until it upgrades, so an undefined gallery would paint a wall of unstyled miniatures rather than
// nothing — a failure that reads as a styling bug in whichever story happens to be open.
defineGalleryElements();
// MJXOFF-186's inputs. Registered here for the reason above and one more of their own: an
// <mjx-option> is a *descriptor*, so an unregistered one is an unknown inline element whose
// attributes have nowhere to go — a dropdown that never defined them would render an empty field
// beside a row of nothing, which reads as a data problem rather than as a missing registration.
defineInputs();
// MJXOFF-187's two pickers. Registered here for the reason above and one of their own: an
// unregistered <mjx-color-picker> is an inert element, so its popup never exists — and a colour
// picker that shows no colours reads as a data problem in whichever story happens to be open
// rather than as a missing registration.
definePickers();
// MJXOFF-188's three surfaces. Registered here for the reason above and one of their own: an
// unregistered <mjx-task-pane> is an unknown inline element, so a pane that is *supposed* to be a
// third of the workspace lays out as a run of text beside the document — which reads as a broken
// layout rather than as a missing registration.
defineSurfaces();
// MJXOFF-189's five feedback components. Registered here for the reason above and one of their
// own: an unregistered <mjx-toast> is a descriptor whose attributes have nowhere to go, so a
// catalogue that had not defined it would render a story with an empty notification stack — which
// reads as a queue that dropped its messages rather than as a missing registration.
defineFeedback();
// MJXOFF-190's document furniture. Registered here for the reason above and one of its own: an
// <mjx-scroll-mark> is a descriptor whose attributes have nowhere to go, so a catalogue that had
// not defined it would flash a row of mark labels into the page beside a scrollbar with an empty
// channel -- which reads as a document with no search hits rather than as a missing registration.
defineFurniture();
// MJXOFF-191's four navigators. Registered here for the reason above and one of their own: all
// four take their contents from a PROPERTY rather than from markup, so an unregistered one is an
// empty box with no rows in it -- which reads as a document with nothing in it rather than as a
// missing registration, and is the one failure mode a story cannot show by looking at it.
defineNavigators();
// MJXOFF-192's Excel chrome. Registered here for the reason above and one of its own: an
// unregistered <mjx-name-box> is an inert element with no field in it at all, so a formula bar
// would render with an empty gap where the address goes -- which reads as a workbook with no
// selection rather than as a missing registration.
defineFormulaChrome();
// MJXOFF-193's annotation family. Registered here for the reason above and one of its own: a
// review pane takes its annotations from a PROPERTY, and an unregistered <mjx-comment-card> is an
// unknown inline element -- so a margin column would lay out as a paragraph of run-together author
// names and comment bodies, which reads as a document whose review data arrived mangled rather
// than as a missing registration.
defineAnnotation();
// MJXOFF-194's two mobile bars. Registered here for the reason above and one of their own: the
// command bar takes its commands from a PROPERTY, so an unregistered one is an empty box — which
// reads as a document with no commands available rather than as a missing registration, and is the
// one failure a story cannot show by looking at it.
defineMobile();

// The foundations on the *document*, because the typography, surface, density and focus classes an
// author writes land in the light DOM. Each component installs them on its own shadow root too;
// one `CSSStyleSheet` is constructed once and adopted by both.
installFoundations(document);

// One rule, on the document: a gallery item is data and must not flash its art into the page in the
// moment between parsing and upgrading. The component's own sheet says the same thing for the items
// slotted into it; this is the half that applies before there is a component.
const galleryItemRule = document.createElement('style');
galleryItemRule.textContent = galleryDocumentCss;
document.head.append(galleryItemRule);

// The same arrangement for MJXOFF-186's two descriptor elements: data written as markup must not
// flash into the layout in the moment between parsing and upgrading.
const inputDescriptorRule = document.createElement('style');
inputDescriptorRule.textContent = inputDocumentCss;
document.head.append(inputDescriptorRule);

// MJXOFF-188's surfaces, and this one is load-bearing rather than cosmetic: it carries the three
// SCHEME-KEYED custom properties — the scrim, the modal's edge and the sheet's handle — which can
// only be declared on `:root`, because which token carries each of them is a different answer in
// the two schemes. A catalogue without it renders a modal with no scrim at all.
const surfaceRule = document.createElement('style');
surfaceRule.textContent = surfaceDocumentCss;
document.head.append(surfaceRule);

// MJXOFF-189's, and this one is load-bearing for the same class of reason: it carries the
// `@property` registrations for the six `<time>` spans. Without them a screentip's delay resolves
// to the un-substituted text `calc(150ms * 4)` rather than to a time, `resolveDurationMilliseconds`
// reports it cannot read one, and every component falls back to its generated default — which works
// and silently ignores anything a host set.
const feedbackRule = document.createElement('style');
feedbackRule.textContent = feedbackDocumentCss;
document.head.append(feedbackRule);

// MJXOFF-190's, and the same arrangement again: <mjx-scroll-mark> is data written as markup and
// must not flash its attributes into the layout in the moment between parsing and upgrading.
const furnitureRule = document.createElement('style');
furnitureRule.textContent = furnitureDocumentCss;
document.head.append(furnitureRule);

// MJXOFF-192's, and this one is load-bearing rather than cosmetic, for the reason the surfaces'
// rule is: it carries the FOUR SCHEME-KEYED reference colours, which can only be declared on
// :root because which token spells a slot is a different answer in the two schemes. A catalogue
// without it renders a formula bar whose references are all the inherited text colour -- a bar
// that has stopped colouring anything, which is exactly what the contract exists to prevent.
const formulaRule = document.createElement('style');
formulaRule.textContent = formulaDocumentCss;
document.head.append(formulaRule);

// MJXOFF-193's, and load-bearing for exactly the reason the formula bar's is: it carries the EIGHT
// SCHEME-KEYED author colours, which can only be declared on :root because which token spells a
// slot is a different answer in the two schemes. A catalogue without it renders every author band
// in the border colour -- a review pane that has stopped telling authors apart, which is the one
// thing the palette search exists to guarantee.
const annotationRule = document.createElement('style');
annotationRule.textContent = annotationDocumentCss;
document.head.append(annotationRule);

// MJXOFF-194's, and load-bearing for the reason the feedback rule is: it carries the FOUR
// REGISTERED safe-area properties. An unregistered custom property hands getComputedStyle its
// substituted text rather than a length, so a padding built on one resolves to nothing -- and a
// notched phone silently loses the inset that keeps its command bar clear of the home indicator,
// which is precisely the defect that is invisible on every device without a notch.
const mobileRule = document.createElement('style');
mobileRule.textContent = mobileDocumentCss;
document.head.append(mobileRule);

/**
 * The attribute the a11y sweep reads to learn what a story expects of itself.
 *
 * `storybook-static/index.json` carries a story's id, title and tags — not its parameters. The
 * sweep needs the parameters, because that is where a gate story declares *which* axe rule it is
 * supposed to violate, and a declaration the checker cannot see is a comment. Rather than reach
 * into Storybook's preview internals, the decorator below writes the declaration onto the root
 * element, where a Playwright test reads it like any other DOM state. Decorators see parameters;
 * this is the supported path.
 */
export const expectationAttribute = 'data-mjx-expect-violations';

/** Written for every story that carries the conventions, so their absence is detectable too. */
export const conventionsAttribute = 'data-mjx-conventions';

/**
 * Theme, applied to the *root* element.
 *
 * `tokens.css`'s scheme layer is three rules on `:root` — the light scheme, then
 * `prefers-color-scheme` unless the host asked for light, then an explicit `data-theme`. So a
 * theme toolbar has exactly one job: choose between those three by writing or removing one
 * attribute. `'system'` removes it, which is what makes the middle rule reachable in the catalogue
 * rather than only in someone's operating system.
 */
const withTheme: Decorator = (story, context) => {
  const preference = (context.globals['theme'] ?? 'light') as SchemePreference;
  applyColorScheme(preference, document.documentElement);
  document.body.style.background = 'var(--theme-background)';
  document.body.style.color = 'var(--theme-text-primary)';
  document.body.style.fontFamily = 'var(--font-sans)';
  return story();
};

/**
 * Every story renders inside a container, never at a viewport width.
 */
const withContainer: Decorator = (story, context) => {
  const preset = (context.globals['containerPreset'] ?? 'desktop') as ContainerPreset;
  return html`<mjx-resizable-container preset=${preset}>${story()}</mjx-resizable-container>`;
};

/**
 * Publish the story's conventions to the DOM, so the browser gates can read them.
 */
const withConventions: Decorator = (story, context) => {
  const conventions = context.parameters['mjx'] as StoryConventions | undefined;
  const root = document.documentElement;
  if (conventions === undefined) {
    root.removeAttribute(conventionsAttribute);
    root.removeAttribute(expectationAttribute);
  } else {
    root.setAttribute(conventionsAttribute, String(conventions.tokenDependencies.length));
    const expected = conventions.expectViolations;
    if (expected === undefined) root.removeAttribute(expectationAttribute);
    else root.setAttribute(expectationAttribute, expected.rules.join(' '));
  }
  return story();
};

const preview: Preview = {
  // Order matters: conventions outermost so they are published before anything renders, then the
  // theme (which writes on :root), then the container (which wraps the story itself).
  decorators: [withContainer, withTheme, withConventions],

  globalTypes: {
    theme: {
      description: 'Colour scheme. `system` removes data-theme and lets prefers-color-scheme rule.',
      toolbar: {
        title: 'Theme',
        icon: 'paintbrush',
        items: [
          { value: 'light', title: 'Light' },
          { value: 'dark', title: 'Dark' },
          { value: 'system', title: 'System' },
        ],
        dynamicTitle: true,
      },
    },
    containerPreset: {
      description: 'Container width. This resizes the container, never the viewport.',
      toolbar: {
        title: 'Container',
        icon: 'ruler',
        items: containerPresetOrder.map((preset) => ({
          value: preset,
          title: `${preset} · ${String(containerPresets[preset])}px`,
        })),
        dynamicTitle: true,
      },
    },
  },

  initialGlobals: { theme: 'light', containerPreset: 'desktop' },

  parameters: {
    layout: 'fullscreen',
    // ⚠ **`Shell` first, and it is the deliverable rather than a preference** (MJXOFF-274). The
    // catalogue is sixteen alphabetised folders and the assembly would otherwise land between
    // `Ribbon` and `Surfaces` — which is exactly where a reviewer opening Storybook to look at the
    // whole application would not think to look. The ticket asks for the nine shells to be
    // *reachable from one obvious place*, and a sort order is the only mechanism Storybook offers
    // for saying which place that is.
    options: { storySort: { order: ['Shell', '*'] } },
    controls: { matchers: { color: /(background|color)$/i } },
    // The panel's own verdict. The *gate* is `tests/browser/a11y.spec.ts`, which runs axe over
    // every story in the built catalogue and fails the build — a panel a person has to remember to
    // look at is not a gate, and this project's rule is that a check is enforced or it is prose.
    a11y: { test: 'error' },
  },
};

export default preview;
