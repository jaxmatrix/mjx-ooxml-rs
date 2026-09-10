/**
 * WCAG 2.x relative luminance and contrast ratio — **shipped, because one control needs it at
 * paint time.**
 *
 * Until MJXOFF-187 this arithmetic lived in `dev/contrast.ts`, where it belonged: every consumer
 * was a gate or a probe, and `dev/` is the tier that exists to write values a component may not.
 * The colour picker is the first consumer that is neither.
 *
 * ## Why a component cannot get away with a table
 *
 * Every other control in this catalogue paints with tokens, so every contrast question it can ask
 * is answerable at build time and is answered in a state table. A colour picker paints with **the
 * colour a person chose**, which is not knowable until they choose it, so the question *"is the
 * selection indicator visible on this swatch"* can only be asked at the moment the swatch is
 * drawn. `chooseSwatchIndicator` in `../pickers/picker-model.ts` asks it, and this is what it asks
 * with.
 *
 * ## The one thing this file is not
 *
 * It is not a second opinion about the palette. `dev/contrast.ts` re-exports these four functions
 * rather than keeping a copy, so `tests/contrast.test.ts`'s comparison against axe is still a
 * comparison between *this* arithmetic and axe's — which is the whole point of that gate, and
 * would have quietly become a comparison between two of our own copies if the move had left one
 * behind.
 *
 * Node-importable: numbers and strings, no DOM.
 */

/** WCAG AA's minimum for normal-size body text. */
export const bodyTextMinimum = 4.5;

/** WCAG AA's minimum for UI components, large text, borders and indicators. */
export const nonTextMinimum = 3;

/** Three 8-bit channels. */
export interface Channels {
  readonly red: number;
  readonly green: number;
  readonly blue: number;
}

/**
 * `#rgb`, `#rrggbb` or `#rrggbbaa` to its three 8-bit channels.
 *
 * Alpha is parsed and discarded: a contrast ratio is defined over *composited* colours, and a
 * caller that has a translucent colour has a compositing question this function cannot answer for
 * it. The short form is expanded the way CSS expands it — each digit doubled, so `#abc` is
 * `#aabbcc` and not `#0a0b0c`.
 */
export function parseHexColor(text: string): Channels | undefined {
  const trimmed = text.trim();
  const short = /^#([0-9a-fA-F]{3})([0-9a-fA-F])?$/.exec(trimmed);
  if (short?.[1] !== undefined) {
    const digits = short[1];
    const doubled = [...digits].map((digit) => digit + digit).join('');
    return channelsOf(Number.parseInt(doubled, 16));
  }
  const long = /^#([0-9a-fA-F]{6})(?:[0-9a-fA-F]{2})?$/.exec(trimmed);
  if (long?.[1] === undefined) return undefined;
  return channelsOf(Number.parseInt(long[1], 16));
}

function channelsOf(value: number): Channels {
  return { red: (value >> 16) & 0xff, green: (value >> 8) & 0xff, blue: value & 0xff };
}

/** Three channels back to `#rrggbb`, lower case, always six digits. */
export function formatHexColor(channels: Channels): string {
  const digits = (value: number): string =>
    Math.max(0, Math.min(255, Math.round(value))).toString(16).padStart(2, '0');
  return `#${digits(channels.red)}${digits(channels.green)}${digits(channels.blue)}`;
}

function linearise(channel: number): number {
  const scaled = channel / 255;
  return scaled <= 0.03928 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4;
}

/** WCAG relative luminance, or `undefined` for a value that is not a hex colour. */
export function relativeLuminance(hex: string): number | undefined {
  const parsed = parseHexColor(hex);
  if (parsed === undefined) return undefined;
  return (
    0.2126 * linearise(parsed.red) +
    0.7152 * linearise(parsed.green) +
    0.0722 * linearise(parsed.blue)
  );
}

/** WCAG contrast ratio between two hex colours, or `undefined` if either is not one. */
export function contrastRatio(a: string, b: string): number | undefined {
  const first = relativeLuminance(a);
  const second = relativeLuminance(b);
  if (first === undefined || second === undefined) return undefined;
  const lighter = Math.max(first, second);
  const darker = Math.min(first, second);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * The contrast ratio, or `1` when either colour could not be read.
 *
 * ⚠ **This exists because of U07's second defect, and it is the opposite of what looks natural.**
 * That child's indicator gate returned `Infinity` for any comparison it could not make, so every
 * *unmeasurable* pair counted as passing and a gate asserting a floor could never fire on one.
 * The safe direction for an unmeasurable comparison is the **failing** one: `1` is the worst ratio
 * that exists, so a colour this arithmetic cannot read fails every assertion loudly instead of
 * satisfying all of them silently.
 */
export function contrastRatioOrWorst(a: string, b: string): number {
  return contrastRatio(a, b) ?? 1;
}

/** `3.39 : 1`, for a caption. */
export function formatRatio(ratio: number): string {
  return `${ratio.toFixed(2)} : 1`;
}
