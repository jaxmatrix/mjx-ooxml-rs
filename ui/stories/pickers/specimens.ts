/**
 * The picker specimens — **and the only place in this child where a literal colour is written.**
 *
 * That is the whole architecture of `<mjx-color-picker>` made visible: the component ships no
 * colours, because a document's theme, a suite's standard row and a person's recent colours all
 * belong to the document rather than to the platform. So the palettes live here, in `stories/`,
 * where `mjx/no-literal-design-values` deliberately does not reach — the rule is about
 * *components*, and the probes and fixtures exist precisely to write values a component may not.
 *
 * `tests/pickers.test.ts` asserts the other half of that sentence by grepping `src/pickers/` for a
 * hex literal and requiring none.
 *
 * The theme palette below is **not** this platform's palette. It is a made-up document's, chosen
 * to be visibly *not* ours — a blue-grey deck — so that an auditor looking at the theme row can
 * see at a glance that the picker is drawing the document's brand and not its own.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  fontAvailabilityNames,
  swatchStateNames,
  swatchStates,
  type FontDescriptor,
  type ThemeColorPalette,
} from '../../src/pickers/index.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

const gridStyle =
  'display:grid;grid-template-columns:repeat(auto-fit,minmax(14rem,1fr));' +
  'gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);' +
  'align-items:start';

const stackStyle =
  'display:grid;gap:calc(var(--mjx-density-gutter) * 2);' +
  'padding:calc(var(--mjx-density-gutter) * 2);max-inline-size:34rem';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:64ch"
    >
      ${text}
    </p>
  `;
}

/** A stage a popup can be placed inside and clipped by. */
export function stage(height: string): string {
  return (
    `position:relative;block-size:${height};padding:calc(var(--mjx-density-gutter) * 2);` +
    'border:1px solid var(--theme-border-subtle);border-radius:var(--radius-card);' +
    'margin:calc(var(--mjx-density-gutter) * 2);overflow:auto'
  );
}

/** A column of fields, which is what a task pane actually is. */
export function stack(...children: TemplateResult[]): TemplateResult {
  return html`<div style=${stackStyle}>${children}</div>`;
}

/** A grid of specimens, one per cell. */
export function grid(...children: TemplateResult[]): TemplateResult {
  return html`<div style=${gridStyle}>${children}</div>`;
}

// ── a document's colours ─────────────────────────────────────────────────────

/**
 * A made-up document's colour scheme.
 *
 * Deliberately nothing like this platform's warm green palette, so that *the theme row is the
 * document's* is a thing an auditor can see rather than a thing they have to be told.
 */
export const documentThemePalette: ThemeColorPalette = {
  background1: '#ffffff',
  text1: '#0f1729',
  background2: '#eef1f6',
  text2: '#2a3550',
  accent1: '#2f5fb0',
  accent2: '#8c3f7d',
  accent3: '#b8622a',
  accent4: '#3f8f6b',
  accent5: '#c0392b',
  accent6: '#6d5bb5',
  hyperlink: '#1a5fb4',
  followedHyperlink: '#7a3f9d',
};

/** A second document, whose theme is missing two slots — so the refused state has a specimen. */
export const partialThemePalette: ThemeColorPalette = {
  background1: '#fffdf5',
  text1: '#241a12',
  accent1: '#a6572d',
  accent2: '#7d6a3a',
  accent3: '#4a6b52',
  accent4: '#3c5c7a',
};

/** Office's standard row, supplied by the host exactly as a real shell would supply it. */
export const standardColors: readonly string[] = [
  '#c00000',
  '#ff0000',
  '#ffc000',
  '#ffff00',
  '#92d050',
  '#00b050',
  '#00b0f0',
  '#0070c0',
  '#002060',
  '#7030a0',
];

/**
 * Recent colours, and the first of them is the one the round-trip gate types.
 *
 * `#123457` is on **no** grid this control draws: it is not a theme slot, not a standard colour,
 * and not a luminance variant of either. That is exactly why it is here — a picker that could only
 * return colours from its own palette would pass every other assertion in the suite.
 */
export const recentColors: readonly string[] = ['#123457', '#7f8c3a', '#38b2ac'];

/** A colour that is on no grid and is never offered — what a person types. */
export const offGridColor = '#123457';

/**
 * The four spellings of one colour, for the round-trip gate.
 *
 * All four must commit the same canonical value. `hsl()` is included because it is the one that
 * cannot survive without going through the same arithmetic twice, and a picker that accepted it
 * and then re-emitted a *nearly* equal colour would drift by one channel every time somebody
 * opened the dialog.
 */
export const oneColourFourWays: readonly string[] = [
  '#123457',
  '#123457'.toUpperCase(),
  'rgb(18, 52, 87)',
  'hsl(214, 65.7%, 20.6%)',
];

// ── a machine's fonts ────────────────────────────────────────────────────────

/**
 * The families a machine reports, with their availability.
 *
 * ⚠ **The `stack` values are generic CSS families rather than face names**, and that is what makes
 * the one-row-tall gate mean something: `serif`, `monospace`, `cursive`, `fantasy` and
 * `system-ui` resolve to genuinely different faces with genuinely different ascenders on every
 * machine this catalogue runs on, whereas a list of Windows face names would silently all fall
 * back to one face in a headless browser and the gate would then be measuring nothing.
 */
export const machineFonts: readonly FontDescriptor[] = [
  {
    family: 'Aptos',
    category: 'Theme fonts',
    availability: 'installed',
    stack: 'system-ui, sans-serif',
  },
  {
    family: 'Aptos Display',
    category: 'Theme fonts',
    availability: 'substituted',
    substitutedBy: 'Segoe UI',
    metricCompatible: true,
    stack: 'sans-serif',
  },
  { family: 'Arial', category: 'All fonts', availability: 'installed', stack: 'sans-serif' },
  {
    family: 'Bookshelf Symbol 7',
    category: 'All fonts',
    availability: 'missing',
    stack: 'fantasy',
  },
  {
    family: 'Calibri',
    category: 'All fonts',
    availability: 'substituted',
    substitutedBy: 'Carlito',
    metricCompatible: true,
    stack: 'sans-serif',
  },
  {
    family: 'Cambria',
    category: 'All fonts',
    availability: 'substituted',
    substitutedBy: 'Caladea',
    metricCompatible: true,
    stack: 'serif',
  },
  { family: 'Comic Sans MS', category: 'All fonts', availability: 'missing', stack: 'cursive' },
  { family: 'Consolas', category: 'All fonts', availability: 'installed', stack: 'monospace' },
  {
    family: 'Constantia',
    category: 'All fonts',
    availability: 'substituted',
    substitutedBy: 'Gelasio',
    metricCompatible: false,
    stack: 'serif',
  },
  { family: 'Courier New', category: 'All fonts', availability: 'installed', stack: 'monospace' },
  { family: 'Georgia', category: 'All fonts', availability: 'installed', stack: 'serif' },
  {
    family: 'Impact',
    category: 'All fonts',
    availability: 'substituted',
    substitutedBy: 'Anton',
    metricCompatible: false,
    stack: 'fantasy',
  },
  { family: 'Palatino Linotype', category: 'All fonts', availability: 'installed', stack: 'serif' },
  {
    family: 'Segoe UI',
    category: 'All fonts',
    availability: 'installed',
    stack: 'system-ui, sans-serif',
  },
  { family: 'Tahoma', category: 'All fonts', availability: 'installed', stack: 'sans-serif' },
  {
    family: 'Times New Roman',
    category: 'All fonts',
    availability: 'substituted',
    substitutedBy: 'Liberation Serif',
    metricCompatible: true,
    stack: 'serif',
  },
  { family: 'Trebuchet MS', category: 'All fonts', availability: 'installed', stack: 'sans-serif' },
  { family: 'Verdana', category: 'All fonts', availability: 'installed', stack: 'sans-serif' },
  {
    family: 'Wingdings',
    category: 'All fonts',
    availability: 'missing',
    stack: 'fantasy',
  },
];

/**
 * A list long enough that virtualisation is the difference between a list and a wall.
 *
 * Larger than the eight rows a list shows and larger than the window plus its overscan, so a
 * built-row count is meaningfully smaller than the option count — an assertion a twenty-item list
 * would satisfy by accident. Every third family is substituted, so the gate can never be looking
 * at a window that happens to contain none.
 */
export const manyFonts: readonly FontDescriptor[] = Array.from({ length: 60 }, (_, index) => {
  const seed = machineFonts[index % machineFonts.length];
  const suffix = index < machineFonts.length ? '' : ` ${String(Math.floor(index / machineFonts.length) + 1)}`;
  return { ...(seed as FontDescriptor), family: `${(seed as FontDescriptor).family}${suffix}` };
});

/** How many families the long list carries, so a gate names a number rather than a length. */
export const manyFontCount = manyFonts.length;

/** The family the warning gate looks at, and the one it looks at for the absence of a warning. */
export const substitutedFamily = 'Cambria';
export const installedFamily = 'Georgia';
export const missingFamily = 'Wingdings';

// ── the declarations every story ships ───────────────────────────────────────

const sharedTokenDependencies: readonly TokenPath[] = [
  'duration.transition',
  'ease.outSoft',
  'ease.spring',
  'fontWeight.bold',
  'fontWeight.medium',
  'leading.snug',
  'leading.tight',
  'radius.chip',
  'radius.control',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'text.sm',
  'text.xs',
];

function pathsFor(members: Iterable<string>): TokenPath[] {
  return [...new Set(members)].map((member) => `theme.light.${member}` as TokenPath);
}

/**
 * The blast radius of a token change, **computed from the swatch table** rather than listed.
 *
 * `textPrimary` and `surface` are added by name because they are the two indicator candidates and
 * do not appear in any state row: the state rows say *whether* the measured ring is drawn, and the
 * measurement says *which of the two*. A change to either moves the indicator on every swatch in
 * the platform, so they belong in the list precisely because the table cannot know about them.
 */
export function colorPickerTokenDependencies(): readonly TokenPath[] {
  const members: string[] = [];
  for (const state of swatchStateNames) {
    const ring = swatchStates[state].cellRing;
    if (ring !== 'transparent') members.push(ring);
  }
  members.push('surfaceRaised', 'surface', 'textPrimary', 'textSecondary', 'border', 'borderSubtle');
  return [...pathsFor(members), ...sharedTokenDependencies].sort((a, b) => a.localeCompare(b));
}

/** The same, for a font picker: a list field's paints, plus nothing of its own. */
export function fontPickerTokenDependencies(): readonly TokenPath[] {
  return [
    ...pathsFor([
      'surface',
      'surfaceRaised',
      'background',
      'border',
      'borderSubtle',
      'textPrimary',
      'textSecondary',
      'accent',
      'accentPressed',
      'accentSurface',
      'accentBorder',
    ]),
    ...sharedTokenDependencies,
  ].sort((a, b) => a.localeCompare(b));
}

/** The swatch states matrix, from the table's own descriptions. */
export function swatchStatesFor(): readonly { name: string; description: string }[] {
  return swatchStateNames.map((name) => ({ name, description: swatchStates[name].description }));
}

/** The font picker's states matrix: one row per availability, plus the field's own two. */
export function fontStatesFor(): readonly { name: string; description: string }[] {
  return [
    ...fontAvailabilityNames.map((name) => ({
      name,
      description:
        name === 'installed'
          ? 'The face is present. The row carries no mark, and the field carries no warning.'
          : name === 'substituted'
            ? 'Something else is drawn in its place. The row carries the word and the glyph; the field carries both and the sentence.'
            : 'Nothing is drawn in its place — a fallback is. The strongest of the three warnings.',
    })),
    {
      name: 'chosen-and-substituted',
      description:
        'The committed family is one that is substituted. This is the state the whole control ' +
        'exists for: the field itself says so, and says so to a screen reader as well.',
    },
    {
      name: 'unavailable',
      description: 'The field cannot be used at all — the run inherits its font from its style.',
    },
  ];
}

export const colorPickerKeyboard = [
  { keys: 'Arrow Down / Arrow Up', does: 'opens the palette, then moves a row within the section' },
  { keys: 'Arrow Left / Arrow Right', does: 'moves one cell, and mirrors under right-to-left' },
  { keys: 'Home / End', does: 'the first and last swatch — unless a value is being typed, when they are caret keys' },
  { keys: 'Page Down / Page Up', does: 'the next and previous section' },
  { keys: 'Enter', does: 'commits the swatch the keyboard is on, or the colour that was typed' },
  { keys: 'Escape', does: 'closes without committing; a second press restores the committed value' },
  { keys: 'Tab', does: 'leaves, and the palette closes. One tab stop for the whole control.' },
  { keys: 'any colour, typed', does: 'commits it — hex, rgb(), hsl(), or a theme slot by name' },
];

export const colorPickerScreenReader =
  'The field announces as a combo box with its label and the name of the colour — "Accent 1, ' +
  'Lighter 40%", never a hex value, because a theme colour is a slot. Each swatch announces its ' +
  'own name and its position in the set; a swatch the document cannot supply announces as ' +
  'disabled with the reason. The palette is a listbox, so focus never leaves the field.';

export const fontPickerKeyboard = [
  { keys: 'Arrow Down / Arrow Up', does: 'opens the list, then moves through it' },
  { keys: 'typing', does: 'filters the families, and the first match becomes the cursor' },
  { keys: 'Home / End', does: 'caret keys — this field is a text box' },
  { keys: 'Enter', does: 'commits the family the cursor is on, or the one that was typed' },
  { keys: 'Escape', does: 'restores the text as it was when the list opened' },
  { keys: 'Tab', does: 'leaves, committing what was typed. One tab stop for the whole control.' },
];

export const fontPickerScreenReader =
  'The field announces as a combo box with its label and the family. When that family is ' +
  'substituted the field carries a description — "Substituted with Caladea. The metrics match, ' +
  'so nothing on the page moves." — which is announced with the field rather than drawn beside ' +
  'it. Each row announces the family, its position in the set, and, for a family that is not ' +
  'installed, the same sentence as part of the row.';
