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
    controls: { matchers: { color: /(background|color)$/i } },
    // The panel's own verdict. The *gate* is `tests/browser/a11y.spec.ts`, which runs axe over
    // every story in the built catalogue and fails the build — a panel a person has to remember to
    // look at is not a gate, and this project's rule is that a check is enforced or it is prose.
    a11y: { test: 'error' },
  },
};

export default preview;
