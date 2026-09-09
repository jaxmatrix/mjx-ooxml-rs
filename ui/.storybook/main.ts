import type { StorybookConfig } from '@storybook/web-components-vite';

/**
 * The catalogue's Storybook.
 *
 * `BUILD_PLAN_LOOP_1.md` §3 fixes the framework: `@storybook/web-components-vite`, because the
 * components are framework-free custom elements and *"the catalogue and the eventual application
 * consume exactly the same artefacts — there is no 'Storybook version' of a component to drift"*.
 *
 * ## The addons, and the one that is deliberately absent
 *
 * `addon-a11y` and `addon-docs` are here. **The viewport addon is not**, and its absence is a
 * decision rather than an omission: the chrome is container-query driven, so responsiveness is
 * proved by `<mjx-resizable-container>` — a real container whose width the auditor changes while
 * the window stays put. A viewport toolbar beside it would offer a second, *wrong* mechanism, and
 * the wrong one is the easier one to reach for.
 *
 * ## No network at runtime
 *
 * Nothing here fetches a font, an icon or a script from a CDN. `--font-sans` names *Nunito Sans*
 * and falls back to `system-ui` until a self-hosted face is added by a later child; that fallback
 * is visible in the catalogue rather than papered over with a Google Fonts link.
 */
const config: StorybookConfig = {
  stories: ['../stories/**/*.stories.ts'],
  addons: ['@storybook/addon-a11y', '@storybook/addon-docs'],
  framework: { name: '@storybook/web-components-vite', options: {} },
  // The plate manifest and its PNGs, so the plate loader has something real to load. See
  // `public/plates/README.md` for where a genuine one comes from.
  staticDirs: ['../public'],
  core: { disableTelemetry: true },
};

export default config;
