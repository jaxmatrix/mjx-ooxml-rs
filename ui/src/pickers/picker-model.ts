/**
 * The picker model: what a colour *is* to this platform, what a font's availability *is*, and the
 * arithmetic both controls are made of.
 *
 * **Node-importable.** Data, strings and pure functions — no DOM, no custom elements — so
 * `tests/pickers.test.ts` can assert the value grammar, the grid navigation and the indicator
 * contrast without a browser, and `tests/browser/pickers.spec.ts` can then assert that the
 * components *obey* it.
 *
 * ## Three things decided here, and each is the reason one of the two controls exists
 *
 * ### 1. A theme colour is a slot, never the colour it currently happens to be
 *
 * This project's standing rule is that **the user's document wins over our defaults**. A theme
 * colour in OOXML is `<a:schemeClr val="accent1"><a:lumMod val="60000"/><a:lumOff val="40000"/>`,
 * and it means *"whatever accent 1 is, forty per cent lighter"*. Resolving it to `#7fc9a3` at the
 * moment a person clicks the swatch would render off-palette inside a customer's branded document
 * for ever after, and would survive every screenshot review, because on the day it is chosen the
 * literal and the slot are the same colour.
 *
 * So `ColorChoice` has a `theme` arm carrying a **slot and a variant**, `formatColorChoice` never
 * emits a hex for it, and `resolveThemeColor` — the only function in this file that turns a slot
 * into a colour — takes the document's palette as an argument and is used **only to paint a
 * swatch**. `tests/pickers.test.ts` asserts structurally that a parsed theme choice carries no
 * literal, which is the assertion that fires if somebody helpfully adds one.
 *
 * ### 2. A selection indicator on an arbitrary colour cannot be a fixed colour
 *
 * Every other control in this catalogue paints with tokens, so every contrast question it can ask
 * is answered in a state table at build time. A colour picker paints with **the colour a person
 * chose**. Any fixed edge fails against some swatch — trivially, against a swatch of that very
 * colour — so `chooseSwatchIndicator` picks, per swatch, whichever of two token colours reads on
 * it, and `worstSwatchIndicator` is what a gate sweeps the sRGB cube with.
 *
 * ⚠ **This is deliberately not a number written down here.** MJXOFF-269 is the record of what
 * happens when a gate compares the code to a model and the *model* is what is wrong: U03's `on`
 * state is stated at 1.20 : 1 in its own table, its distinctness gate and its correspondence gate
 * are both green, and neither can see that 1.20 : 1 is not an indicator at all. So this file
 * states the **rule** and the **candidates**, and the gate measures the outcome.
 *
 * ### 3. A font list is the substitution manifest, or it is a lie
 *
 * `mjx-text` carries the metric-compatible substitution table and the per-document substitution
 * manifest. A picker that listed four hundred families without saying which of them this machine
 * does not have is a picker that lets a person choose a font and get a different one — silently,
 * with the lines rewrapped. `substitutionNote` is the sentence, `substitutionWord` is the mark in
 * the row, and both are `undefined` for an installed face and defined for every other, which is
 * the whole assertion.
 *
 * ## Literal colours, and why there are none in this file
 *
 * `mjx/no-literal-design-values` forbids a hex colour anywhere in `src/`, and a colour picker is
 * the one control that legitimately handles literal colours — **as data**. The two are kept apart
 * by construction rather than by care: **this component ships no colours at all.** The theme
 * palette, the standard row and the recent row are all supplied by the host, because they belong
 * to the document rather than to us; the only colours the picker names are the two *token members*
 * its indicator may be drawn in. `tests/pickers.test.ts` greps the whole of `src/pickers/` for a
 * hex literal and requires none, which is a stronger statement than the lint rule's, and is proved
 * able to fail against a source that has one.
 */

import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import {
  contrastRatioOrWorst,
  formatHexColor,
  formatRatio,
  nonTextMinimum,
  parseHexColor,
  type Channels,
} from '../tokens/contrast.ts';
import type { ThemeMember } from '../tokens/resolver.ts';
import { themeVariable } from '../controls/control-states.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
  spacingMultiple,
} from '../foundations/density.ts';
import { radiusVariable, surfaceLevels } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { motionRoleClass } from '../foundations/motion.ts';
import { floatingProperties } from '../overlay/floating.ts';
import { nextGalleryIndex, type GalleryRow } from '../gallery/gallery-model.ts';
import { inputBoxProperties, type OptionDescriptor } from '../inputs/input-model.ts';

/** The two elements this child registers. */
export const pickerTags = {
  colorPicker: 'mjx-color-picker',
  fontPicker: 'mjx-font-picker',
} as const;

// ── the theme colour vocabulary ──────────────────────────────────────────────

/**
 * The twelve slots of an OOXML colour scheme, in the order Office's colour gallery draws them.
 *
 * The names are expanded per this project's naming convention — a reader should not need the spec
 * — and the **wire token is preserved exactly** beside each one, because that is the string that
 * goes into `<a:schemeClr val="…">` and a guess there is a corrupted document.
 */
export const themeColorSlotNames = [
  'background1',
  'text1',
  'background2',
  'text2',
  'accent1',
  'accent2',
  'accent3',
  'accent4',
  'accent5',
  'accent6',
  'hyperlink',
  'followedHyperlink',
] as const;

/** One of the twelve. */
export type ThemeColorSlot = (typeof themeColorSlotNames)[number];

/** What each slot is called, and the wire token it serialises as. */
export interface ThemeColorSlotSpec {
  /** What a person sees in the tooltip and hears from a screen reader. */
  readonly label: string;
  /** The exact `ST_SchemeColorVal` token. Never guessed, never abbreviated further. */
  readonly wire: string;
  /** Whether the slot is drawn in the ten-column row of the gallery. */
  readonly inGallery: boolean;
}

export const themeColorSlots: Readonly<Record<ThemeColorSlot, ThemeColorSlotSpec>> = {
  background1: { label: 'Background 1', wire: 'bg1', inGallery: true },
  text1: { label: 'Text 1', wire: 'tx1', inGallery: true },
  background2: { label: 'Background 2', wire: 'bg2', inGallery: true },
  text2: { label: 'Text 2', wire: 'tx2', inGallery: true },
  accent1: { label: 'Accent 1', wire: 'accent1', inGallery: true },
  accent2: { label: 'Accent 2', wire: 'accent2', inGallery: true },
  accent3: { label: 'Accent 3', wire: 'accent3', inGallery: true },
  accent4: { label: 'Accent 4', wire: 'accent4', inGallery: true },
  accent5: { label: 'Accent 5', wire: 'accent5', inGallery: true },
  accent6: { label: 'Accent 6', wire: 'accent6', inGallery: true },
  // The two link colours are part of the scheme and are *not* in the gallery grid, exactly as in
  // Office: a hyperlink's colour is chosen by the style that owns it, never by a person painting a
  // run. They are here so a value that names one still parses and still round-trips.
  hyperlink: { label: 'Hyperlink', wire: 'hlink', inGallery: false },
  followedHyperlink: { label: 'Followed hyperlink', wire: 'folHlink', inGallery: false },
};

/** The ten slots the gallery draws, one per column, in order. */
export const galleryThemeSlots: readonly ThemeColorSlot[] = themeColorSlotNames.filter(
  (slot) => themeColorSlots[slot].inGallery,
);

/** Whether a string names a slot, spelled exactly. */
export function isThemeColorSlot(value: unknown): value is ThemeColorSlot {
  return (themeColorSlotNames as readonly string[]).includes(String(value));
}

/**
 * The slot a person named, however they capitalised it.
 *
 * ⚠ **Separate from `isThemeColorSlot`, and the separation is a bug this file already had.** The
 * value grammar is case-insensitive — `#ABC` and `#abc` are one colour — so the first version of
 * `parseColorChoice` lower-cased the whole string before looking a slot up. Ten of the twelve slot
 * names survive that, because they are already lower case; `followedHyperlink` and its kind do
 * not, and `theme:followedHyperlink` therefore failed to parse **its own canonical output**. The
 * round-trip suite found it, which is the entire argument for round-tripping every value a
 * vocabulary can express rather than a representative sample of it.
 */
export function themeColorSlotFor(text: string): ThemeColorSlot | undefined {
  const wanted = text.trim().toLowerCase();
  return themeColorSlotNames.find((slot) => slot.toLowerCase() === wanted);
}

/**
 * The six rows of the gallery: the slot itself and five luminance variants.
 *
 * The numbers are PowerPoint's own, and they are *percentages of a percentage*: OOXML writes them
 * as thousandths, so `lighter40` is `<a:lumMod val="60000"/><a:lumOff val="40000"/>`. Luminance
 * modulation is defined in HSL — `L' = L × lumMod + lumOff` — which is why `applyLuminance` goes
 * through `rgbToHsl` rather than scaling the channels, and why a scaled-channel implementation
 * would be subtly wrong for every saturated colour.
 */
export const themeColorVariantNames = [
  'base',
  'lighter80',
  'lighter60',
  'lighter40',
  'darker25',
  'darker50',
] as const;

/** One of the six. */
export type ThemeColorVariant = (typeof themeColorVariantNames)[number];

export interface ThemeColorVariantSpec {
  /** What a person sees. The base row has no suffix. */
  readonly label: string;
  /** `lumMod`, as a percentage. 100 leaves the luminance alone. */
  readonly luminanceModulation: number;
  /** `lumOff`, as a percentage. 0 leaves the luminance alone. */
  readonly luminanceOffset: number;
}

export const themeColorVariants: Readonly<Record<ThemeColorVariant, ThemeColorVariantSpec>> = {
  base: { label: '', luminanceModulation: 100, luminanceOffset: 0 },
  lighter80: { label: 'Lighter 80%', luminanceModulation: 20, luminanceOffset: 80 },
  lighter60: { label: 'Lighter 60%', luminanceModulation: 40, luminanceOffset: 60 },
  lighter40: { label: 'Lighter 40%', luminanceModulation: 60, luminanceOffset: 40 },
  darker25: { label: 'Darker 25%', luminanceModulation: 75, luminanceOffset: 0 },
  darker50: { label: 'Darker 50%', luminanceModulation: 50, luminanceOffset: 0 },
};

/** Whether a string names a variant, spelled exactly. */
export function isThemeColorVariant(value: unknown): value is ThemeColorVariant {
  return (themeColorVariantNames as readonly string[]).includes(String(value));
}

/** The variant a person named, however they capitalised it. See `themeColorSlotFor`. */
export function themeColorVariantFor(text: string): ThemeColorVariant | undefined {
  const wanted = text.trim().toLowerCase();
  return themeColorVariantNames.find((variant) => variant.toLowerCase() === wanted);
}

/** `lighter40` → `lumMod 60000, lumOff 40000`, the attributes a writer emits. */
export function themeVariantWire(variant: ThemeColorVariant): string {
  const spec = themeColorVariants[variant];
  const thousandths = (percent: number): string => String(Math.round(percent * 1000));
  if (variant === 'base') return '';
  if (spec.luminanceOffset === 0) return `lumMod ${thousandths(spec.luminanceModulation)}`;
  return `lumMod ${thousandths(spec.luminanceModulation)}, lumOff ${thousandths(spec.luminanceOffset)}`;
}

// ── HSL, and OOXML luminance modulation ──────────────────────────────────────

/** Hue in degrees, saturation and lightness as fractions. */
export interface Hsl {
  readonly hue: number;
  readonly saturation: number;
  readonly lightness: number;
}

/** sRGB channels to HSL. Nothing is rounded: a rounded intermediate is a lost colour. */
export function rgbToHsl(channels: Channels): Hsl {
  const red = channels.red / 255;
  const green = channels.green / 255;
  const blue = channels.blue / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const lightness = (max + min) / 2;
  const span = max - min;
  if (span === 0) return { hue: 0, saturation: 0, lightness };
  const saturation = lightness > 0.5 ? span / (2 - max - min) : span / (max + min);
  let hue: number;
  if (max === red) hue = ((green - blue) / span + (green < blue ? 6 : 0)) * 60;
  else if (max === green) hue = ((blue - red) / span + 2) * 60;
  else hue = ((red - green) / span + 4) * 60;
  return { hue, saturation, lightness };
}

/** HSL back to sRGB channels, rounded to the eight bits a colour actually has. */
export function hslToRgb(colour: Hsl): Channels {
  const hue = ((colour.hue % 360) + 360) % 360;
  const saturation = clampFraction(colour.saturation);
  const lightness = clampFraction(colour.lightness);
  if (saturation === 0) {
    const grey = Math.round(lightness * 255);
    return { red: grey, green: grey, blue: grey };
  }
  const second = lightness < 0.5 ? lightness * (1 + saturation) : lightness + saturation - lightness * saturation;
  const first = 2 * lightness - second;
  const channel = (offset: number): number => {
    let t = (hue / 360 + offset) % 1;
    if (t < 0) t += 1;
    if (t < 1 / 6) return first + (second - first) * 6 * t;
    if (t < 1 / 2) return second;
    if (t < 2 / 3) return first + (second - first) * (2 / 3 - t) * 6;
    return first;
  };
  return {
    red: Math.round(channel(1 / 3) * 255),
    green: Math.round(channel(0) * 255),
    blue: Math.round(channel(-1 / 3) * 255),
  };
}

function clampFraction(value: number): number {
  return Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 0;
}

/**
 * OOXML luminance modulation and offset, applied to a colour.
 *
 * `L' = L × lumMod + lumOff`, both as fractions. This is the transform PowerPoint's gallery rows
 * *are*: the whole six-row ladder is one base colour and five of these.
 */
export function applyLuminance(hex: string, modulation: number, offset: number): string | undefined {
  const parsed = parseHexColor(hex);
  if (parsed === undefined) return undefined;
  const hsl = rgbToHsl(parsed);
  const lightness = clampFraction((hsl.lightness * modulation) / 100 + offset / 100);
  return formatHexColor(hslToRgb({ ...hsl, lightness }));
}

// ── the value ────────────────────────────────────────────────────────────────

/**
 * What a colour picker reports.
 *
 * Four arms and no fifth. `automatic` and `none` are *not* colours and must not be spelled as one:
 * a run with no explicit colour inherits, and a shape with no fill is transparent, and both of
 * those are facts about the document rather than pixels.
 */
export type ColorChoice =
  | { readonly kind: 'automatic' }
  | { readonly kind: 'none' }
  | { readonly kind: 'theme'; readonly slot: ThemeColorSlot; readonly variant: ThemeColorVariant }
  | { readonly kind: 'literal'; readonly hex: string };

/** The value a picker holds when nothing has been chosen. */
export const noColorChosen = '';

/** The canonical spelling of a choice — what goes in the `value` attribute. */
export function formatColorChoice(choice: ColorChoice): string {
  switch (choice.kind) {
    case 'automatic':
      return 'automatic';
    case 'none':
      return 'none';
    case 'theme':
      return choice.variant === 'base'
        ? `theme:${choice.slot}`
        : `theme:${choice.slot}/${choice.variant}`;
    case 'literal':
      return choice.hex;
  }
}

const rgbFunction = /^rgba?\(\s*([^)]*)\)$/i;
const hslFunction = /^hsla?\(\s*([^)]*)\)$/i;
const bareHex = /^[0-9a-fA-F]{3}$|^[0-9a-fA-F]{6}$/;

/**
 * Read a colour a person or a host wrote, in any spelling this control accepts.
 *
 * Accepted: `automatic`, `none`, `theme:<slot>` and `theme:<slot>/<variant>`, `#rgb`, `#rrggbb`,
 * a bare `rgb`/`rrggbb` with no hash, `rgb(r, g, b)` and `hsl(h, s%, l%)` in either the comma or
 * the space syntax. Refused: anything else — including an `rgba()` or `hsla()` with an alpha that
 * is not 1, because a picker that silently dropped the transparency a person typed would be
 * reporting a colour they did not ask for.
 */
export function parseColorChoice(text: string): ColorChoice | undefined {
  const trimmed = text.trim();
  if (trimmed === '') return undefined;
  const lower = trimmed.toLowerCase();

  if (lower === 'automatic' || lower === 'auto') return { kind: 'automatic' };
  if (lower === 'none' || lower === 'no fill' || lower === 'transparent') return { kind: 'none' };

  if (lower.startsWith('theme:')) {
    const parts = trimmed.slice('theme:'.length).split('/');
    if (parts.length > 2) return undefined;
    const slot = themeColorSlotFor(parts[0] ?? '');
    const variant = themeColorVariantFor(parts[1] ?? 'base');
    if (slot === undefined || variant === undefined) return undefined;
    return { kind: 'theme', slot, variant };
  }

  const hex = parseHexColor(trimmed.startsWith('#') ? trimmed : `#${trimmed}`);
  if (hex !== undefined && (trimmed.startsWith('#') || bareHex.test(trimmed))) {
    return { kind: 'literal', hex: formatHexColor(hex) };
  }

  const rgb = rgbFunction.exec(trimmed);
  if (rgb?.[1] !== undefined) {
    const parts = numbersIn(rgb[1]);
    if (parts === undefined) return undefined;
    if (parts.length === 4 && parts[3] !== 1) return undefined;
    if (parts.length !== 3 && parts.length !== 4) return undefined;
    const [red, green, blue] = parts;
    if (red === undefined || green === undefined || blue === undefined) return undefined;
    if (![red, green, blue].every((value) => value >= 0 && value <= 255)) return undefined;
    return { kind: 'literal', hex: formatHexColor({ red, green, blue }) };
  }

  const hsl = hslFunction.exec(trimmed);
  if (hsl?.[1] !== undefined) {
    const parts = numbersIn(hsl[1]);
    if (parts === undefined) return undefined;
    if (parts.length === 4 && parts[3] !== 1) return undefined;
    if (parts.length !== 3 && parts.length !== 4) return undefined;
    const [hue, saturation, lightness] = parts;
    if (hue === undefined || saturation === undefined || lightness === undefined) return undefined;
    if (saturation < 0 || saturation > 100 || lightness < 0 || lightness > 100) return undefined;
    return {
      kind: 'literal',
      hex: formatHexColor(hslToRgb({ hue, saturation: saturation / 100, lightness: lightness / 100 })),
    };
  }

  return undefined;
}

/** The numbers in a function's argument list, in either the comma or the space syntax. */
function numbersIn(body: string): number[] | undefined {
  const cleaned = body.replace(/\//g, ' ').replace(/,/g, ' ').replace(/%/g, ' ');
  const parts = cleaned.trim().split(/\s+/).filter((part) => part !== '');
  const numbers: number[] = [];
  for (const part of parts) {
    if (!/^[+-]?(?:\d+\.?\d*|\.\d+)$/.test(part)) return undefined;
    numbers.push(Number.parseFloat(part));
  }
  return numbers;
}

/** Whether two choices are the same choice. */
export function colorChoicesEqual(a: ColorChoice, b: ColorChoice): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === 'theme' && b.kind === 'theme') return a.slot === b.slot && a.variant === b.variant;
  if (a.kind === 'literal' && b.kind === 'literal') return a.hex === b.hex;
  return true;
}

/** What a person is told a choice is. A slot is named as a slot, and never as its colour. */
export function describeColorChoice(choice: ColorChoice): string {
  switch (choice.kind) {
    case 'automatic':
      return 'Automatic';
    case 'none':
      return 'No fill';
    case 'theme': {
      const slot = themeColorSlots[choice.slot].label;
      const variant = themeColorVariants[choice.variant].label;
      return variant === '' ? slot : `${slot}, ${variant}`;
    }
    case 'literal':
      return choice.hex;
  }
}

// ── resolution, which happens only in order to paint ─────────────────────────

/** The document's own colour scheme: what each slot is, right now, in this document. */
export type ThemeColorPalette = Readonly<Partial<Record<ThemeColorSlot, string>>>;

/**
 * A slot and a variant, as a colour — **for painting a swatch and for nothing else.**
 *
 * Returns `undefined` when the document has not said what the slot is, which is the honest answer
 * and is what stops the picker inventing a palette. A picker with no theme draws no theme row;
 * `tests/browser/pickers.spec.ts` asserts that, because the alternative — falling back to our own
 * accent — is exactly the "author a default where something already exists" failure the project's
 * standing rule names.
 */
export function resolveThemeColor(
  slot: ThemeColorSlot,
  variant: ThemeColorVariant,
  palette: ThemeColorPalette,
): string | undefined {
  const base = palette[slot];
  if (base === undefined) return undefined;
  const spec = themeColorVariants[variant];
  return applyLuminance(base, spec.luminanceModulation, spec.luminanceOffset);
}

/** What a choice paints as, or `undefined` when it paints nothing (`none`) or cannot be resolved. */
export function resolveColorChoice(
  choice: ColorChoice,
  palette: ThemeColorPalette,
  automatic: string | undefined,
): string | undefined {
  switch (choice.kind) {
    case 'automatic':
      return automatic;
    case 'none':
      return undefined;
    case 'theme':
      return resolveThemeColor(choice.slot, choice.variant, palette);
    case 'literal':
      return choice.hex;
  }
}

// ── the swatch indicator, which is measured rather than declared ─────────────

/**
 * The two token members a selection indicator may be drawn in.
 *
 * Two, and both are ends of the scheme's own range: the text colour and the surface colour. That
 * is what makes the worst case bounded — for any colour at all, one of the two extremes of a
 * palette is far from it — and it is why a third candidate would not help.
 */
export const swatchIndicatorCandidates = ['textPrimary', 'surface'] as const;

/** One of the two. */
export type SwatchIndicatorMember = (typeof swatchIndicatorCandidates)[number];

/** Which member reads on a given swatch, and how well. */
export interface SwatchIndicator {
  readonly member: SwatchIndicatorMember;
  readonly ratio: number;
}

/**
 * Choose the indicator for one swatch, given what the two candidates **actually resolve to here**.
 *
 * ⚠ **The comparison is against the swatch, not against the surface.** The ring is drawn *inside*
 * the coloured square, so the only thing it has to be visible against is the colour a person
 * chose, and that colour is not knowable until they choose it. An unreadable swatch resolves to a
 * ratio of 1 rather than to `Infinity` — see `contrastRatioOrWorst`, and U07's second defect.
 *
 * ⚠ **And the candidate colours are an argument rather than a lookup**, because a host that has
 * re-themed the platform by declaring `--theme-text-primary` has changed the answer. A component
 * that measured against the *generated* table would choose correctly for our own palette and
 * wrongly for the palette actually on screen — which is the same class of mistake as hard-coding a
 * token's value, wearing a measurement's costume.
 */
export function chooseSwatchIndicatorAmong(
  swatch: string,
  candidates: Readonly<Record<SwatchIndicatorMember, string>>,
): SwatchIndicator {
  let best: SwatchIndicator | undefined;
  for (const member of swatchIndicatorCandidates) {
    const ratio = contrastRatioOrWorst(candidates[member], swatch);
    if (best === undefined || ratio > best.ratio) best = { member, ratio };
  }
  // `swatchIndicatorCandidates` is a non-empty literal tuple, so `best` is always assigned; the
  // fallback exists because a type-checker cannot know that and a `!` would be a lie about why.
  return best ?? { member: 'textPrimary', ratio: 1 };
}

/** The same, against the generated palette for one scheme. What a gate sweeps with. */
export function chooseSwatchIndicator(swatch: string, scheme: ColorScheme): SwatchIndicator {
  return chooseSwatchIndicatorAmong(swatch, {
    textPrimary: tokens.theme[scheme].textPrimary,
    surface: tokens.theme[scheme].surface,
  });
}

/** The custom property the chosen member is written to, so the ring is one declaration. */
export const swatchIndicatorProperty = '--mjx-swatch-indicator';

/**
 * The custom property the **user's colour** is written to.
 *
 * Deliberately a different property from the indicator, and deliberately named for what it is: a
 * reader of this stylesheet must be able to tell at a glance which declarations are tokens-as-style
 * and which one is colours-as-data. There is exactly one of the latter, and it is this.
 */
export const swatchPaintProperty = '--mjx-swatch-paint';

/** The glyph the chosen swatch carries. A shape, so selection is not colour alone. */
export const swatchSelectedGlyph = { name: 'checkmark', size: 16 } as const;

/** The worst swatch in a set, and how badly its indicator reads. */
export interface WorstIndicator {
  readonly swatch: string;
  readonly indicator: SwatchIndicator;
}

/** The weakest indicator over a set of colours — the number a gate asserts. */
export function worstSwatchIndicator(
  swatches: Iterable<string>,
  scheme: ColorScheme,
): WorstIndicator | undefined {
  let worst: WorstIndicator | undefined;
  for (const swatch of swatches) {
    const indicator = chooseSwatchIndicator(swatch, scheme);
    if (worst === undefined || indicator.ratio < worst.indicator.ratio) {
      worst = { swatch, indicator };
    }
  }
  return worst;
}

/**
 * The colours the indicator rule is proved over.
 *
 * **Every grey**, because the analytic worst case for "the better of two extremes" lies on the
 * grey line — a colour equally badly served by both ends of the palette is a colour with no hue to
 * help it — plus a lattice of the sRGB cube, so a hue-dependent mistake in the arithmetic has
 * somewhere to show up. A gate that swept only the palette in the stories would be measuring the
 * fixture rather than the rule.
 */
export function indicatorSweepColors(cubeStep = 17): string[] {
  const colours: string[] = [];
  for (let grey = 0; grey <= 255; grey += 1) {
    colours.push(formatHexColor({ red: grey, green: grey, blue: grey }));
  }
  for (let red = 0; red <= 255; red += cubeStep) {
    for (let green = 0; green <= 255; green += cubeStep) {
      for (let blue = 0; blue <= 255; blue += cubeStep) {
        colours.push(formatHexColor({ red, green, blue }));
      }
    }
  }
  return colours;
}

/** `3.47 : 1 on #808080`, for a caption and for a failure message. */
export function describeWorstIndicator(worst: WorstIndicator): string {
  return `${formatRatio(worst.indicator.ratio)} on ${worst.swatch} (${worst.indicator.member})`;
}

/** WCAG 1.4.11's floor, re-exported so the picker's gate and its story read the same number. */
export { nonTextMinimum };

// ── the swatch's states ──────────────────────────────────────────────────────

/**
 * The five states a swatch cell takes, and the **two different rings** they are made of.
 *
 * The distinction is the whole design and it is not a detail:
 *
 * * the **cell ring** is drawn outside the coloured square, on the popup's own surface, so it is a
 *   token against a token and its contrast is decidable here, at build time;
 * * the **inset ring** is drawn inside the coloured square, on the colour a person chose, so its
 *   contrast is decidable only at paint time and `chooseSwatchIndicator` is what decides it.
 *
 * A design that used one ring for both jobs would have had to be a fixed colour, and would have
 * been invisible on the swatch of that colour. Which swatch is in the palette is then a matter of
 * luck, and a gate over the stories' palette would have agreed it was fine.
 */
export const swatchStateNames = ['rest', 'active', 'selected', 'selectedActive', 'unavailable'] as const;

/** One of the five. */
export type SwatchState = (typeof swatchStateNames)[number];

export interface SwatchStateSpec {
  /** What puts a cell into it, and what an auditor should look for. */
  readonly description: string;
  /** The selectors that produce it, with `%s` standing for the cell's own selector. */
  readonly matches: readonly string[];
  /** The ring outside the square, on the popup surface. `transparent` draws none. */
  readonly cellRing: ThemeMember | 'transparent';
  /** Whether the measured ring inside the square is drawn. */
  readonly insetRing: boolean;
  /** What says the state is on without using colour. */
  readonly nonColourCue?: string;
}

export const swatchStates: Readonly<Record<SwatchState, SwatchStateSpec>> = {
  rest: {
    description: 'A colour in the grid, neither chosen nor under the keyboard.',
    matches: ['%s'],
    cellRing: 'transparent',
    insetRing: false,
  },
  active: {
    description:
      'The keyboard is on it — what aria-activedescendant points at. Also what the pointer hovers.',
    matches: ['%s[data-active]', '%s:hover:not([aria-disabled="true"])'],
    // accentPressed rather than accent, for the reason the option table gives: the popup's fill is
    // surface-raised in both schemes, and the gate below measures both rather than trusting this.
    cellRing: 'accentPressed',
    insetRing: false,
  },
  selected: {
    description: 'The chosen colour. A ring inside the square, in whichever token reads on it.',
    matches: ['%s[aria-selected="true"]'],
    cellRing: 'transparent',
    insetRing: true,
    nonColourCue: 'a check mark drawn inside the square, in the same measured colour as the ring.',
  },
  selectedActive: {
    description: 'The chosen colour, with the keyboard on it. Both rings, which is the point of two.',
    matches: ['%s[aria-selected="true"][data-active]', '%s[aria-selected="true"]:hover'],
    cellRing: 'accentPressed',
    insetRing: true,
    nonColourCue: 'a check mark, and the cursor ring outside the square as well as the one inside.',
  },
  unavailable: {
    description:
      'A colour the document cannot currently take — a theme slot this document does not define. ' +
      'Still arrow-reachable, still announced, still refused.',
    matches: ['%s[aria-disabled="true"]'],
    cellRing: 'transparent',
    insetRing: false,
    nonColourCue: 'a dashed edge, aria-disabled="true", and the reason it cannot be chosen.',
  },
};

/** The cascade, in emission order. `unavailable` last, so a refused choice still looks chosen. */
export const swatchStateCascade: readonly SwatchState[] = [
  'rest',
  'active',
  'selected',
  'selectedActive',
  'unavailable',
];

/** The surface a cell ring is drawn on. The popup is the overlay rung, exactly as a menu. */
export const swatchCellBackground: ThemeMember = 'surfaceRaised';

/**
 * The custom property the **hairline around a swatch** is written to.
 *
 * ⚠ **A different property from `swatchIndicatorProperty`, because it answers a different
 * question, and MJXOFF-279 is the ticket that found out the hard way.**
 *
 * The two rings a swatch carries were always distinguished (`swatchStates` says so at length), but
 * the *hairline* — the one-pixel edge every swatch wears, selected or not — was drawn in the
 * measured **indicator**, which is chosen to read against **the colour a person chose**. Its job is
 * the opposite one: it is the outermost edge of the square, its outward neighbour is the popup's
 * own surface, and what it has to be told apart from is *that*.
 *
 * The two coincide at the extremes, which is why the original reasoning looked complete: a swatch
 * the colour of the popup takes `textPrimary` as its indicator, and `textPrimary` reads on the
 * popup. It is the **middle** that breaks. A mid-tone swatch takes `surface` as its indicator —
 * `surface` reads better on it than `textPrimary` does — and `surface` against `surfaceRaised` is
 * 1.11 : 1 in dark. So neither the swatch nor its hairline was told from the popup, and
 * `tests/pickers.test.ts`'s boundary sweep bottomed out at **2.85 : 1 on `#ff2222`** the moment
 * MJXOFF-271's re-seed lifted the dark surfaces.
 *
 * One property per question, each measured against the thing it actually sits next to.
 */
export const swatchHairlineProperty = '--mjx-swatch-hairline';

/**
 * Choose the hairline for a popup, given the surface it is drawn on and what the candidates
 * **actually resolve to here**.
 *
 * The arithmetic is `chooseSwatchIndicatorAmong`'s and is delegated to it rather than repeated —
 * *which of two candidates reads on this colour* is one question. What differs is the subject: the
 * indicator's is the swatch, the hairline's is the popup. A reader who conflates them again will
 * find the paragraph above waiting.
 *
 * It is a **per-popup** value rather than a per-cell one, because nothing about it depends on the
 * swatch. `<mjx-color-picker>` writes it once onto the palette and lets it inherit.
 */
export function chooseSwatchHairlineAmong(
  background: string,
  candidates: Readonly<Record<SwatchIndicatorMember, string>>,
): SwatchIndicator {
  return chooseSwatchIndicatorAmong(background, candidates);
}

/** The same, against the generated palette for one scheme. What a gate sweeps with. */
export function chooseSwatchHairline(scheme: ColorScheme): SwatchIndicator {
  return chooseSwatchHairlineAmong(tokens.theme[scheme][swatchCellBackground], {
    textPrimary: tokens.theme[scheme].textPrimary,
    surface: tokens.theme[scheme].surface,
  });
}

/** Every cell ring that actually paints, with what it is drawn on. The gate measures these. */
export const swatchCellRingPairs: readonly (readonly [ThemeMember, ThemeMember])[] =
  swatchStateCascade
    .map((state) => swatchStates[state].cellRing)
    .filter((ring): ring is ThemeMember => ring !== 'transparent')
    .map((ring) => [ring, swatchCellBackground] as const);

/** The state a live cell is in, from the three facts the cell itself carries. */
export function swatchStateOf(cell: {
  readonly selected: boolean;
  readonly active: boolean;
  readonly unavailable: boolean;
}): SwatchState {
  if (cell.unavailable) return 'unavailable';
  if (cell.selected && cell.active) return 'selectedActive';
  if (cell.selected) return 'selected';
  if (cell.active) return 'active';
  return 'rest';
}

// ── the grid ─────────────────────────────────────────────────────────────────

/** The widest a palette section is drawn. Ten, because a theme scheme has ten gallery slots. */
export const swatchGridColumns = 10;

/**
 * One section of the popup: a run of swatches with its own column count.
 *
 * ⚠ **This is why `galleryRowPlan` is not reused here, and the difference is real rather than a
 * preference.** That planner takes *one* column count for a whole surface, which is right for a
 * gallery and right for a list; a colour popup has a ten-wide theme block, a ten-wide standard
 * row, a ragged recent row and a two-wide row of *Automatic* and *No fill*, and a plan that forced
 * one width on all four would put the arrow keys on cells that are not where they look. The
 * *shape* it produces is still `GalleryRow`, so anything U06 built on that shape still applies.
 */
export interface SwatchSection {
  readonly name: string;
  /** Inclusive index of the section's first option in the flat list. */
  readonly start: number;
  /** Exclusive index of the section's last. */
  readonly end: number;
  /** How many cells a row of this section holds. Never zero. */
  readonly columns: number;
}

/**
 * Group a flat option list into sections, in first-appearance order.
 *
 * A section is as wide as it needs to be and never wider than `swatchGridColumns`, which is what
 * makes the two-item *Reset* row two cells wide rather than two cells and eight holes — and makes
 * `ArrowRight` on *Automatic* land on *No fill* rather than on nothing.
 */
export function swatchSections(options: readonly OptionDescriptor[]): SwatchSection[] {
  const sections: SwatchSection[] = [];
  let index = 0;
  while (index < options.length) {
    const name = options[index]?.category ?? '';
    let end = index;
    while (end < options.length && (options[end]?.category ?? '') === name) end += 1;
    sections.push({ name, start: index, end, columns: Math.min(swatchGridColumns, end - index) });
    index = end;
  }
  return sections;
}

/** The row plan: a heading row per named section, then its runs of cells. */
export function swatchRowPlan(sections: readonly SwatchSection[]): GalleryRow[] {
  const rows: GalleryRow[] = [];
  for (const section of sections) {
    if (section.name !== '') rows.push({ kind: 'heading', category: section.name });
    for (let start = section.start; start < section.end; start += section.columns) {
      rows.push({ kind: 'cells', start, end: Math.min(start + section.columns, section.end) });
    }
  }
  return rows;
}

/** Everything a key press can ask a swatch grid to do. */
export const swatchActionNames = [
  'open',
  'close',
  'commit',
  'revert',
  'inlineNext',
  'inlinePrevious',
  'rowNext',
  'rowPrevious',
  'first',
  'last',
  'sectionNext',
  'sectionPrevious',
  'leave',
] as const;

/** One of the thirteen. */
export type SwatchAction = (typeof swatchActionNames)[number];

/** What the swatch key map needs to know. */
export interface SwatchKeyContext {
  readonly open: boolean;
  /** The writing direction. The **inline** arrows mirror under it and the block arrows never do. */
  readonly direction: 'ltr' | 'rtl';
}

/**
 * The grid's key map.
 *
 * Two things differ from a listbox's and both follow from the surface being two-dimensional:
 * the inline arrows exist at all, and they **mirror under right-to-left** while `ArrowDown` and
 * `ArrowUp` never do. `tests/pickers.test.ts` asserts the mirroring by requiring the two
 * directions to disagree about the same key — an assertion that a symmetric implementation would
 * fail, which a "the pointer ends up in the right place" assertion would not.
 */
export function swatchKeyAction(key: string, context: SwatchKeyContext): SwatchAction | undefined {
  const { open, direction } = context;
  const mirrored = direction === 'rtl';
  switch (key) {
    // ⚠ `undefined` while the popup is shut, and not `open`. The field is a text box a person may
    // be typing a hex value into, and the inline arrows are its caret keys — a picker that opened
    // its palette when somebody pressed Left to fix a typo would be a picker nobody could type in.
    // The block arrows have no such meaning in a one-line field, so those do open it.
    case 'ArrowRight':
      return open ? (mirrored ? 'inlinePrevious' : 'inlineNext') : undefined;
    case 'ArrowLeft':
      return open ? (mirrored ? 'inlineNext' : 'inlinePrevious') : undefined;
    case 'ArrowDown':
      return open ? 'rowNext' : 'open';
    case 'ArrowUp':
      return open ? 'rowPrevious' : 'open';
    case 'PageDown':
      return open ? 'sectionNext' : undefined;
    case 'PageUp':
      return open ? 'sectionPrevious' : undefined;
    case 'Home':
      return open ? 'first' : undefined;
    case 'End':
      return open ? 'last' : undefined;
    case 'Enter':
      return 'commit';
    case 'Escape':
      return open ? 'close' : 'revert';
    case 'Tab':
      return 'leave';
    default:
      return undefined;
  }
}

/** The section an index belongs to, or `undefined` when it belongs to none. */
export function sectionOf(
  sections: readonly SwatchSection[],
  index: number,
): SwatchSection | undefined {
  return sections.find((section) => index >= section.start && index < section.end);
}

/**
 * Where a movement lands.
 *
 * **Within a section** it is U06's `nextGalleryIndex` over that section's own range and column
 * count — the same arithmetic a gallery navigates with, including its ragged-tail clamp, rather
 * than a second set of off-by-one errors written for colours.
 *
 * **Between sections** the block arrows step out: `ArrowDown` from the last row of the theme block
 * lands in the standard row, in the same column if that row has one. Nothing wraps: a grid that
 * jumped from the last recent colour back to *Automatic* would be a grid that lost somebody's
 * place, which is the same reason `nextGalleryIndex` refuses to wrap.
 */
export function nextSwatchIndex(
  action: SwatchAction,
  index: number,
  sections: readonly SwatchSection[],
): number {
  const first = sections[0];
  const last = sections.at(-1);
  if (first === undefined || last === undefined) return -1;
  if (action === 'first') return first.start;
  if (action === 'last') return last.end - 1;

  const current = Math.min(Math.max(index, first.start), last.end - 1);
  const section = sectionOf(sections, current);
  if (section === undefined) return first.start;
  const position = sections.indexOf(section);
  const local = current - section.start;
  const count = section.end - section.start;
  const grid = { count, columns: section.columns, rowsPerPage: 1 };

  switch (action) {
    case 'inlineNext':
      return section.start + nextGalleryIndex('next', local, grid);
    case 'inlinePrevious':
      return section.start + nextGalleryIndex('previous', local, grid);
    case 'rowNext': {
      if (local + section.columns < count) {
        return section.start + nextGalleryIndex('rowDown', local, grid);
      }
      const next = sections[position + 1];
      if (next === undefined) return section.start + nextGalleryIndex('rowDown', local, grid);
      const column = local % section.columns;
      return Math.min(next.start + column, next.end - 1);
    }
    case 'rowPrevious': {
      if (local - section.columns >= 0) {
        return section.start + nextGalleryIndex('rowUp', local, grid);
      }
      const previous = sections[position - 1];
      if (previous === undefined) return current;
      const column = local % section.columns;
      const width = previous.columns;
      const lastRowStart = previous.start + Math.floor((previous.end - 1 - previous.start) / width) * width;
      return Math.min(lastRowStart + column, previous.end - 1);
    }
    case 'sectionNext': {
      const next = sections[position + 1];
      return next === undefined ? last.end - 1 : next.start;
    }
    case 'sectionPrevious': {
      // Deliberately *this* section's start when the cursor is not already on it — the same
      // behaviour a page-up has in a document, and it makes two presses reach the section above
      // rather than one press skipping past whatever the cursor was in the middle of.
      if (local > 0) return section.start;
      const previous = sections[position - 1];
      return previous === undefined ? first.start : previous.start;
    }
    default:
      return current;
  }
}

// ── the font picker ──────────────────────────────────────────────────────────

/** What this machine can actually do with a family the document asks for. */
export const fontAvailabilityNames = ['installed', 'substituted', 'missing'] as const;

/** One of the three. */
export type FontAvailability = (typeof fontAvailabilityNames)[number];

/**
 * One family, as the font engine reports it.
 *
 * This is the shape `mjx-text`'s per-document substitution manifest projects into the chrome. The
 * picker never decides any of it: a control that guessed at availability would be a control that
 * told a person their document was fine.
 */
export interface FontDescriptor {
  /** The family name. Also the value the control reports. */
  readonly family: string;
  /** The section it is listed under — theme fonts, recently used, all fonts. */
  readonly category?: string;
  readonly availability: FontAvailability;
  /** The family actually used in its place. Absent when nothing stood in for it. */
  readonly substitutedBy?: string;
  /** Whether the stand-in has the same metrics, so nothing on the page moves. */
  readonly metricCompatible?: boolean;
  /** The CSS families the row's own name is drawn in. */
  readonly stack?: string;
}

/** Whether a family is drawn with something other than itself. */
export function isSubstituted(font: FontDescriptor): boolean {
  return font.availability !== 'installed';
}

/**
 * The word the row carries beside the family name, or `undefined` for a face that is present.
 *
 * ⚠ **A word and not only a colour.** `--theme-text-secondary` is 4.32 : 1 on
 * `--theme-border-subtle`, which is what an option row fills with under the keyboard cursor, so a
 * grey note would go illegible exactly when a person pointed at it. U05 and U07 both settled this
 * the same way and so does this: the note is **primary text at the dense size**, told apart by
 * size and by a glyph, never by colour.
 */
export function substitutionWord(font: FontDescriptor): string | undefined {
  switch (font.availability) {
    case 'installed':
      return undefined;
    case 'substituted':
      return 'Substituted';
    case 'missing':
      return 'Missing';
  }
}

/**
 * The whole sentence — what a screen reader announces and what the field shows under itself.
 *
 * Metric compatibility is the half that decides whether this is a cosmetic difference or a
 * repagination, so it is the half the sentence leads with.
 */
export function substitutionNote(font: FontDescriptor): string | undefined {
  switch (font.availability) {
    case 'installed':
      return undefined;
    case 'substituted': {
      const stand = font.substitutedBy;
      if (stand === undefined || stand === '') {
        return 'Substituted. This machine does not have the face, and the document does not carry it.';
      }
      return font.metricCompatible === true
        ? `Substituted with ${stand}. The metrics match, so nothing on the page moves.`
        : `Substituted with ${stand}. The metrics differ, so lines may rewrap.`;
    }
    case 'missing':
      return 'Not available. Text in this font is drawn with a fallback, and lines may rewrap.';
  }
}

/** The mark a substituted row and a substituted field both carry. */
export const substitutionGlyph = { name: 'warning', size: 16 } as const;

/**
 * The families a row's own name is drawn in.
 *
 * A substituted family is drawn in **what it will actually be drawn in**, which is the stand-in,
 * because a list that previewed a face this machine does not have would be previewing a lie — the
 * browser would fall back silently and the row would look installed.
 */
export function fontPreviewStack(font: FontDescriptor): string {
  if (font.stack !== undefined && font.stack !== '') return font.stack;
  const shown = font.availability === 'installed' ? font.family : (font.substitutedBy ?? font.family);
  return `${quoteFamily(shown)}, var(--font-sans)`;
}

/** A family name as CSS wants it. Quoted always: a bare `Segoe UI` is two idents and an error. */
export function quoteFamily(family: string): string {
  return `"${family.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;
}

/** The families that need a warning — the whole reason this control is not a dropdown of strings. */
export function fontsNeedingWarning(fonts: readonly FontDescriptor[]): FontDescriptor[] {
  return fonts.filter((font) => substitutionNote(font) !== undefined);
}

/** A family, as an option the shared list machinery understands. */
export function fontOption(font: FontDescriptor): OptionDescriptor {
  return {
    value: font.family,
    label: font.family,
    ...(font.category === undefined ? {} : { category: font.category }),
  };
}

/**
 * ⚠ **A substituted font is never `unavailable`.**
 *
 * It is choosable, and that is the point: a document may legitimately ask for a face this machine
 * does not have, and the person editing it may legitimately want to keep asking. What the control
 * owes them is to *say so*, not to refuse. A picker that greyed out every uninstalled family would
 * quietly rewrite documents on machines that were missing a font.
 */
export const substitutedFontsRemainChoosable = true;

// ── typography, motion and the sheets ────────────────────────────────────────

/** The type roles this child's parts use. */
export const pickerTypeRoles = {
  fieldValue: 'control',
  swatchHeading: 'label',
  fontName: 'control',
  /** The substitution note. Dense, which is how it is told apart from the name beside it. */
  substitution: 'dense',
  message: 'dense',
} as const;

/** `fontName` → `mjx-type-control`. */
export function pickerTypeClass(part: keyof typeof pickerTypeRoles): string {
  return typeRoleClass(pickerTypeRoles[part]);
}

/** The class the popup wears when it enters. The panel's motion, exactly as a list's. */
export const pickerMotionClass = motionRoleClass('panelEnter');

/** The custom properties the pickers' boxes read. Layout only; no paint goes through one. */
export const pickerBoxProperties = {
  /** The side of one swatch cell. Floored at the accessible minimum, whatever the density says. */
  swatchCellSize: '--mjx-swatch-cell-size',
  /** How many cells a section's rows hold. Written per section, read by the grid. */
  swatchColumns: '--mjx-swatch-columns',
  /** The thickness of the ring drawn inside a swatch. */
  swatchRing: '--mjx-swatch-ring',
} as const;

const overlay = surfaceLevels.overlay;

/**
 * The colour popup's rules.
 *
 * ⚠ **Nothing here paints a state.** Every fill, edge and ring comes from a `:where()` rule in
 * `swatchStatesCss` at (0,0,0), for the reason `inputBaseCss` states at length: a single
 * `border-color:` in this block would score (0,1,0) and out-specify the whole table, which is
 * MJXOFF-181's specificity accident and the one MJXOFF-183 actually shipped.
 *
 * The one colour a cell does carry is `background` on `.swatch-paint`, and it is not a paint in
 * that sense at all: it is the **user's own colour**, written as an inline custom property by the
 * component, and it is the entire content of the control.
 */
export const colorPickerCss = `
  :host {
    display: inline-block;
    vertical-align: middle;
    ${pickerBoxProperties.swatchCellSize}: max(
      var(${densityProperties.hitTarget}),
      ${String(accessibleHitTargetMinimum)}px
    );
    ${pickerBoxProperties.swatchRing}: ${spacingMultiple(0.5)};
  }
  :host([hidden]) { display: none; }

  .preview {
    flex: 0 0 auto;
    display: inline-block;
    box-sizing: border-box;
    inline-size: ${spacingMultiple(4)};
    block-size: ${spacingMultiple(4)};
    border-width: 1px;
    border-style: solid;
    border-radius: ${radiusVariable('chip')};
    background: var(${swatchPaintProperty}, transparent);
    /* The same edge question as the popup's hairline, asked about the field the preview sits in
     * rather than about the popup. Not the indicator: this border's outward neighbour is the
     * field's own fill, and the indicator is chosen against the colour inside the square. */
    border-color: var(${swatchHairlineProperty}, ${themeVariable('border')});
  }

  /* No fill, and Automatic before a host has said what it resolves to: a diagonal rule rather than
   * an empty box, because an empty box is indistinguishable from a white one. */
  .preview[data-empty] {
    background-image: linear-gradient(
      to bottom right,
      transparent calc(50% - 1px),
      ${themeVariable('textSecondary')} calc(50% - 1px),
      ${themeVariable('textSecondary')} calc(50% + 1px),
      transparent calc(50% + 1px)
    );
  }

  .palette {
    box-sizing: border-box;
    margin: 0;
    padding-block: var(${densityProperties.step});
    padding-inline: 0;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: max-content;
    max-inline-size: var(${floatingProperties.maxInlineSize}, none);
    max-block-size: min(
      var(${inputBoxProperties.listMaxBlockSize}, 100vh),
      var(${floatingProperties.maxBlockSize}, 100vh)
    );
    display: flex;
    flex-direction: column;
    gap: var(${densityProperties.step});
    overflow-y: auto;
    overscroll-behavior: contain;
    background: ${overlay.background};
    border: ${overlay.border};
    box-shadow: ${overlay.shadow};
    border-radius: ${radiusVariable(overlay.radius)};
    color: ${themeVariable('textPrimary')};
    z-index: 1;
  }

  .palette[data-open='false'] { display: none; }

  .section-heading {
    flex: 0 0 auto;
    box-sizing: border-box;
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    color: ${themeVariable('textSecondary')};
  }

  /*
   * A section's rows, as a grid of its own width.
   *
   * overflow-x is auto rather than clip: below about ten accessible targets the popup cannot show
   * a ten-wide block, and the honest answer is that the block scrolls sideways. Narrowing the
   * cells instead would put them under the 24px floor, which is the one thing a density mode in
   * this catalogue is never allowed to do.
   */
  .section-grid {
    flex: 0 0 auto;
    display: grid;
    grid-template-columns: repeat(
      var(${pickerBoxProperties.swatchColumns}, ${String(swatchGridColumns)}),
      minmax(var(${pickerBoxProperties.swatchCellSize}), 1fr)
    );
    gap: var(${densityProperties.step});
    padding-inline: var(${densityProperties.gutter});
    overflow-x: auto;
  }

  .swatch {
    display: grid;
    place-items: center;
    box-sizing: border-box;
    color: inherit;
    margin: 0;
    padding: var(${densityProperties.step});
    min-inline-size: var(${pickerBoxProperties.swatchCellSize});
    min-block-size: var(${pickerBoxProperties.swatchCellSize});
    border-width: 1px;
    border-radius: ${radiusVariable('chip')};
    appearance: none;
    -webkit-appearance: none;
    background: transparent;
    cursor: pointer;
    transition-property: border-color, box-shadow, opacity;
  }

  .swatch[aria-disabled='true'] { cursor: help; }

  /* The colour itself. The only place in this component where a colour that is not a token is
   * painted, and it arrives as an inline custom property written by the element. */
  .swatch-paint {
    grid-area: 1 / 1;
    display: grid;
    place-items: center;
    box-sizing: border-box;
    inline-size: 100%;
    block-size: 100%;
    border-radius: ${radiusVariable('chip')};
    background: var(${swatchPaintProperty}, transparent);
    /* The hairline, measured against **the popup** rather than against the swatch: a swatch the
     * colour of the popup is told from it by its edge, and a swatch nothing like the popup is told
     * from it by itself. See swatchHairlineProperty for why the two are different properties —
     * borrowing the indicator here left mid-tone swatches at 2.85 : 1 against the popup. */
    box-shadow: inset 0 0 0 1px var(${swatchHairlineProperty});
  }

  .swatch-paint[data-empty] {
    background-image: linear-gradient(
      to bottom right,
      transparent calc(50% - 1px),
      ${themeVariable('textSecondary')} calc(50% - 1px),
      ${themeVariable('textSecondary')} calc(50% + 1px),
      transparent calc(50% + 1px)
    );
  }

  /*
   * ⚠ At (0,0,0) on purpose. The state table turns the mark on with a descendant selector under
   * :where(), which scores (0,1,0), so the table wins by SPECIFICITY rather than by order — and
   * emission order only ever arbitrates between rules of equal specificity, which is precisely the
   * accident MJXOFF-183 shipped and MJXOFF-181 wrote down.
   *
   * It lives inside .swatch-paint, which already centres its contents, so one rule places it in
   * both the square-only cell and the labelled chip.
   */
  :where(.swatch-mark) {
    display: none;
    color: var(${swatchIndicatorProperty});
  }

  /*
   * ⚠ **Automatic and No fill are chips with words on them, and the word is NEVER drawn on the
   * colour.**
   *
   * It was, and axe caught it: the Automatic chip fills with the document's own text colour, so
   * its label came out at 1.17 : 1. The obvious repair — draw the word in the measured indicator,
   * like the ring — is WRONG, and the reason is the distinction this whole child turns on. The
   * indicator rule is WCAG 1.4.11, a **non-text** rule with a floor of 3 : 1, and the measured
   * worst case over the sRGB cube is 3.47 : 1. Text needs 4.5 : 1. So a measurement that licenses
   * a ring does not license a word, and a chip that put one on an arbitrary colour would have been
   * illegible for some documents and fine for ours.
   *
   * So the square shrinks and the word sits beside it, on the popup's own surface, in the popup's
   * own text colour — a token against a token, at 11.75 : 1 in light and 12.63 : 1 in dark.
   */
  .swatch[data-chip] {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: var(${densityProperties.step});
  }

  .swatch[data-chip] .swatch-paint {
    flex: 0 0 auto;
    inline-size: ${spacingMultiple(4)};
    block-size: ${spacingMultiple(4)};
  }

  .swatch-label {
    flex: 1 1 auto;
    min-inline-size: 0;
    text-align: start;
    color: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
`;

/** The state rules for one swatch selector, in cascade order and all at (0,0,0). */
export function swatchStatesCss(selector: string): string {
  return swatchStateCascade
    .map((state) => {
      const spec = swatchStates[state];
      const where = spec.matches.map((match) => match.replaceAll('%s', selector)).join(', ');
      const declarations = [
        `  border-color: ${spec.cellRing === 'transparent' ? 'transparent' : themeVariable(spec.cellRing)};`,
        spec.insetRing
          ? `  box-shadow: inset 0 0 0 var(${pickerBoxProperties.swatchRing}) var(${swatchIndicatorProperty});`
          : '  box-shadow: none;',
        state === 'unavailable' ? '  border-style: dashed;' : '  border-style: solid;',
      ].join('\n');
      const mark = spec.insetRing ? `\n:where(${where}) .swatch-mark {\n  display: grid;\n}` : '';
      return `:where(${where}) {\n${declarations}\n}${mark}`;
    })
    .join('\n');
}

/**
 * The font picker's own rules, over the list field's.
 *
 * ⚠ **`.font-name` has an explicit `block-size` and `overflow: hidden`, and both are load-bearing.**
 * Each family is drawn in its own face, and a face decides its own ascender and descender — so a
 * row whose height came from its content would be a *different height per row*, which is exactly
 * the precondition U06's virtualiser has and U07 discovered was unstated. The row's height comes
 * from `--mjx-option-block-size` as it does for every other list in this catalogue; the face
 * changes the glyphs inside it and nothing else. `tests/browser/pickers.spec.ts` measures every
 * built row of a list of faces with deliberately different metrics and requires one height.
 */
export const fontPickerCss = `
  .option-label.font-name {
    display: block;
    box-sizing: border-box;
    block-size: 100%;
    align-content: center;
    overflow: hidden;
    /*
     * normal, and NOT a fixed ratio. A preview of a face should use that face's own line box, and
     * there is a gate reason as well: a fixed leading makes every row the same height whatever the
     * face is, so the one-row-tall assertion would have stayed green with the row's block-size
     * deleted. What holds the height is the row; what makes losing the row VISIBLE is this.
     */
    line-height: normal;
  }

  .substitution {
    grid-column: 3;
    grid-row: 1 / -1;
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    /* Primary text, at the dense size. Never --theme-text-secondary: an option's fill changes
     * under the keyboard cursor and grey on that fill is 4.32 : 1. Size, not colour. */
    color: inherit;
    white-space: nowrap;
  }

  .field-warning {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    color: inherit;
  }

  .substitution-message {
    display: block;
    margin-block-start: var(${densityProperties.step});
    color: ${themeVariable('textPrimary')};
  }

  .substitution-message[hidden] { display: none; }
`;

// ── the catalogue contract ───────────────────────────────────────────────────

/** Where each picker's stories live. Spelled again as literals in the story files — CSF is static. */
export const pickerStoryTitles = {
  colorPicker: 'Pickers/Colour Picker',
  fontPicker: 'Pickers/Font Picker',
} as const;

/** The story every table publishes its whole states matrix under. */
export const pickerStatesMatrixStoryName = 'The States Matrix';

/**
 * The attribute a swatch matrix cell is found by.
 *
 * ⚠ Unlike an option row, a swatch **can** be wrapped by a story: the states matrix draws bare
 * cells outside a popup, because all five states of a swatch are producible without opening
 * anything. The live rows are asserted as well — the gate reads each cell's own `aria-selected`,
 * `data-active` and `aria-disabled` and computes the expected state from those — so the matrix is
 * the picture and the live grid is the proof.
 */
export const swatchStateCellAttribute = 'data-swatch-state-cell';
