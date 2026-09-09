/**
 * Contrast arithmetic, used to *choose* what the a11y gate demonstrates on.
 *
 * ## Why this is computed and not written down
 *
 * MJXOFF-180's brief names a pair — `--color-green` at 3.39 : 1 rejected as body text,
 * `--color-green-deep` at 5.34 : 1 accepted. Both figures are exactly right against
 * `--color-white` in the tree today; this module recomputes them and agrees. But the palette is
 * about to be re-seeded from an existing product of the user's, and a gate that named two tokens
 * would break on the day the names changed — which is precisely the day a contrast gate is most
 * needed.
 *
 * So the exemplars are **derived from whatever palette is generated**: the rejected one is the
 * closest miss (the highest ratio still below the body-text minimum — the most plausible mistake a
 * designer could make), and the accepted one is the narrowest pass (the lowest ratio at or above
 * it — the one whose legality is least obvious). The gate proves the *rule* is enforced; the
 * specific names are an output, not an input.
 *
 * ## And why the arithmetic is here at all when axe already has it
 *
 * Because the test asserts that the two **agree**. axe is the enforcer; this is an independent
 * prediction of what it should say. A gate that only asked axe would pass identically whether the
 * rule was enforced or silently disabled — the disabled case simply reports no violations, which
 * reads the same as a clean palette. Predicting the verdict and comparing is what tells those two
 * apart, and it is the same doctrine `mjx-paint` uses when it refuses to compare a painter with
 * itself.
 *
 * WCAG 2.x relative luminance and contrast ratio, from the definitions in the specification.
 */

import { tokens, customProperties } from '../tokens/tokens.ts';
import { generatedValue } from './token-choice.ts';
import type { TokenPath } from '../src/tokens/resolver.ts';

/** WCAG AA's minimum for normal-size body text. */
export const bodyTextMinimum = 4.5;

/** WCAG AA's minimum for UI components, large text, borders and indicators. */
export const nonTextMinimum = 3;

/** `#rrggbb` or `#rrggbbaa` to its three 8-bit channels. Alpha is ignored: a contrast ratio is
 *  defined over composited colours, and every exemplar here is painted on an opaque background. */
function channels(hex: string): [number, number, number] | undefined {
  const match = /^#([0-9a-fA-F]{6})(?:[0-9a-fA-F]{2})?$/.exec(hex.trim());
  if (match?.[1] === undefined) return undefined;
  const value = Number.parseInt(match[1], 16);
  return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
}

function linearise(channel: number): number {
  const scaled = channel / 255;
  return scaled <= 0.03928 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4;
}

/** WCAG relative luminance, or `undefined` for a value that is not an opaque hex colour. */
export function relativeLuminance(hex: string): number | undefined {
  const parsed = channels(hex);
  if (parsed === undefined) return undefined;
  const [red, green, blue] = parsed;
  return 0.2126 * linearise(red) + 0.7152 * linearise(green) + 0.0722 * linearise(blue);
}

/** WCAG contrast ratio between two opaque hex colours, or `undefined` if either is not one. */
export function contrastRatio(a: string, b: string): number | undefined {
  const first = relativeLuminance(a);
  const second = relativeLuminance(b);
  if (first === undefined || second === undefined) return undefined;
  const lighter = Math.max(first, second);
  const darker = Math.min(first, second);
  return (lighter + 0.05) / (darker + 0.05);
}

/** One colour token measured against a background. */
export interface MeasuredToken {
  readonly path: TokenPath;
  readonly value: string;
  readonly ratio: number;
  /** Whether it clears the body-text minimum against the background it was measured on. */
  readonly legalAsBodyText: boolean;
}

/**
 * Every colour token in the generated table, measured against `background` and ordered by ratio.
 *
 * Scoped to the flat `color` group deliberately: the `theme.*` and `document.*` groups repeat those
 * values under scheme-relative names, and a ledger that listed each colour three times would make
 * the palette look three times larger than it is.
 */
export function measurePalette(background: string): MeasuredToken[] {
  const measured: MeasuredToken[] = [];
  for (const path of Object.keys(customProperties) as TokenPath[]) {
    if (!path.startsWith('color.')) continue;
    const value = generatedValue(path);
    const ratio = contrastRatio(value, background);
    if (ratio === undefined) continue;
    measured.push({ path, value, ratio, legalAsBodyText: ratio >= bodyTextMinimum });
  }
  return measured.sort((a, b) => a.ratio - b.ratio);
}

/** What the gate stories demonstrate on. */
export interface ContrastExemplars {
  readonly background: string;
  /** The closest miss: the highest ratio still below the body-text minimum. */
  readonly rejected: MeasuredToken;
  /** The narrowest pass: the lowest ratio at or above it. */
  readonly accepted: MeasuredToken;
  /** True when the palette had no failing token and `rejected` is a synthetic stand-in. */
  readonly rejectedIsSynthetic: boolean;
}

/**
 * A colour that is guaranteed to fail body-text contrast against white, used only if the palette
 * ever stops containing one of its own. It is not a token and never becomes one — it exists so the
 * gate story cannot silently disappear on the day the palette becomes fully accessible.
 */
const syntheticFailure = '#949494';

/** Choose the exemplars. */
export function contrastExemplars(background: string = tokens.theme.light.surface): ContrastExemplars {
  const measured = measurePalette(background);
  const failing = measured.filter((token) => !token.legalAsBodyText);
  const passing = measured.filter((token) => token.legalAsBodyText);

  const accepted = passing[0];
  if (accepted === undefined) {
    throw new Error(
      'no colour token in the generated palette is legal as body text against ' +
        `${background}. The catalogue cannot show accent-coloured text at all, which is a ` +
        'palette defect and not a harness one.',
    );
  }

  const closestMiss = failing.at(-1);
  if (closestMiss !== undefined) {
    return { background, rejected: closestMiss, accepted, rejectedIsSynthetic: false };
  }

  const ratio = contrastRatio(syntheticFailure, background) ?? 1;
  return {
    background,
    rejected: {
      path: 'color.white' as TokenPath,
      value: syntheticFailure,
      ratio,
      legalAsBodyText: false,
    },
    accepted,
    rejectedIsSynthetic: true,
  };
}

/** `3.39 : 1`, for a caption. */
export function formatRatio(ratio: number): string {
  return `${ratio.toFixed(2)} : 1`;
}
