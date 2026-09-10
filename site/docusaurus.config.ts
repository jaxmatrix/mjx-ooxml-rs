import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import {themes as prismThemes} from 'prism-react-renderer';

/**
 * The user guide's site.
 *
 * Local only: there is no deployment and no CI job. `npm run start` regenerates the content tree
 * from the committed guide markdown and serves it; `npm run build` does the same and fails on a
 * broken link, which is what holds the generator's link resolution to something.
 */
const config: Config = {
  title: 'mjx-ooxml',
  tagline: 'Make and edit .pptx, .docx and .xlsx — from Rust, Python or TypeScript',
  favicon: 'img/favicon.svg',

  // No deployment yet, so these are what a local build serves from.
  url: 'http://localhost:3000',
  baseUrl: '/',
  organizationName: 'jaxmatrix',
  projectName: 'mjx-ooxml-rs',

  // A dead link is a build failure rather than a warning: the whole reason the generator resolves
  // intra-doc links in Rust is that nothing else can, and a warning nobody reads would make that
  // resolution unchecked.
  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',
  markdown: {hooks: {onBrokenMarkdownLinks: 'throw'}},

  i18n: {defaultLocale: 'en', locales: ['en']},

  presets: [
    [
      'classic',
      {
        docs: {
          // The docs *are* the site. There is no blog and no landing page separate from the guide's
          // own index, which is already written to be the first thing a reader meets.
          routeBasePath: '/',
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/jaxmatrix/mjx-ooxml-rs/tree/main/',
          showLastUpdateTime: false,
        },
        blog: false,
        theme: {customCss: './src/css/custom.css'},
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {respectPrefersColorScheme: true},
    navbar: {
      title: 'mjx-ooxml',
      items: [
        {type: 'docSidebar', sidebarId: 'guide', position: 'left', label: 'Guide'},
        {to: '/install/installing', label: 'Install', position: 'left'},
        {to: '/walkthroughs/build_a_deck', label: 'Walkthroughs', position: 'left'},
        {
          href: 'https://github.com/jaxmatrix/mjx-ooxml-rs',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      copyright:
        'Every code block on this site is a copy of a file cargo, pytest or node --test runs.',
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      // `rust` and `python` ship with Prism's Docusaurus bundle; `toml` and `bash` do not.
      additionalLanguages: ['rust', 'toml', 'bash'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
