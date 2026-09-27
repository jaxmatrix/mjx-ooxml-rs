/**
 * Colour-vision simulation and CIEDE2000 — **the instrument that judges author colours, and the
 * two naive answers it is pointed at.**
 *
 * ## Why this is in `dev/` and not in `src/`
 *
 * Nothing paints with it. `src/tokens/contrast.ts` earned its place in the shipped tree because one
 * control asks a contrast question at the moment a person chooses a colour; this arithmetic is only
 * ever asked at build time about a table that is fixed. `dev/` is the tier that exists for exactly
 * that — a gate instrument, and the one place a literal colour may be written down, which the two
 * naive controls below need.
 *
 * ## What the simulation is
 *
 * Viénot, Brettel & Mollon (1999), *"Digital video colourmaps for checking the legibility of
 * displays by dichromats"* — the LMS-plane projection that is the standard way of asking *what does
 * a dichromat see*. It is applied in **linear** sRGB, which is the half implementations most often
 * get wrong: projecting gamma-encoded channels produces a picture that is plausible, darker than it
 * should be, and wrong by different amounts in different parts of the ramp.
 *
 * It answers for dichromacy — protanopia, deuteranopia, tritanopia — and not for the anomalous
 * trichromacies, which are commoner and milder. That is the right way round for a gate: a pair a
 * dichromat can separate is a pair a protanomalous reader can separate, so the strict case is the
 * one worth asserting.
 *
 * ## And what the metric is
 *
 * CIEDE2000, in full, with the hue-rotation term. `tests/annotation.test.ts` checks it against
 * **nine rows of Sharma, Wu & Dalal's published test data** — the dataset written specifically to
 * catch the implementation mistakes this formula invites, including the hue-angle wrap and the
 * blue-region rotation — so the number the author-colour gate is built on is a number an outside
 * party has already agreed with.
 *
 * Node-importable: numbers and strings, no DOM.
 */

import { parseHexColor, formatHexColor, type Channels } from '../src/tokens/contrast.ts';

/** The three dichromacies the gate asserts against. */
export const deficiencyNames = ['protanopia', 'deuteranopia', 'tritanopia'] as const;

/** One of the three. */
export type Deficiency = (typeof deficiencyNames)[number];

/** The four ways of looking at a colour: normal vision and the three dichromacies. */
export const visionNames = ['normal', ...deficiencyNames] as const;

/** One of the four. */
export type Vision = (typeof visionNames)[number];

/**
 * The projection matrices, in linear sRGB.
 *
 * Each replaces the missing cone's response with the one the other two determine: a protanope's red
 * channel is reconstructed from green and blue, a deuteranope's green from red and blue, and a
 * tritanope's blue from red and green. The rows that are the identity are the channels a dichromat
 * still has, and they are left exactly alone.
 */
const projections: Readonly<Record<Deficiency, readonly (readonly number[])[]>> = {
  protanopia: [
    [0, 1.05118294, -0.05116099],
    [0, 1, 0],
    [0, 0, 1],
  ],
  deuteranopia: [
    [1, 0, 0],
    [0.9513092, 0, 0.04866992],
    [0, 0, 1],
  ],
  tritanopia: [
    [1, 0, 0],
    [0, 1, 0],
    [-0.86744736, 1.86727089, 0],
  ],
};

function toLinear(channel: number): number {
  const value = channel / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

function fromLinear(value: number): number {
  const encoded = value <= 0.0031308 ? value * 12.92 : 1.055 * Math.max(value, 0) ** (1 / 2.4) - 0.055;
  return Math.min(255, Math.max(0, Math.round(encoded * 255)));
}

/** What a dichromat sees, as a hex colour. `undefined` when the input is not a colour. */
export function simulateDeficiency(hex: string, deficiency: Deficiency): string | undefined {
  const channels = parseHexColor(hex);
  if (channels === undefined) return undefined;
  const linear = [toLinear(channels.red), toLinear(channels.green), toLinear(channels.blue)];
  const matrix = projections[deficiency];
  const projected = matrix.map(
    (row) => (row[0] ?? 0) * (linear[0] ?? 0) + (row[1] ?? 0) * (linear[1] ?? 0) + (row[2] ?? 0) * (linear[2] ?? 0),
  );
  const seen: Channels = {
    red: fromLinear(projected[0] ?? 0),
    green: fromLinear(projected[1] ?? 0),
    blue: fromLinear(projected[2] ?? 0),
  };
  return formatHexColor(seen);
}

/** What a given kind of vision sees. `normal` is the identity, which keeps a sweep one loop. */
export function seenAs(hex: string, vision: Vision): string | undefined {
  return vision === 'normal' ? hex : simulateDeficiency(hex, vision);
}

/** CIE L*a*b* under D65, which is the white point sRGB is defined against. */
export type Lab = readonly [number, number, number];

/** A colour in L*a*b*. `undefined` when the input is not a colour. */
export function toLab(hex: string): Lab | undefined {
  const channels = parseHexColor(hex);
  if (channels === undefined) return undefined;
  const r = toLinear(channels.red);
  const g = toLinear(channels.green);
  const b = toLinear(channels.blue);
  // sRGB to CIEXYZ, then normalised by the D65 white point.
  const x = (r * 0.4124564 + g * 0.3575761 + b * 0.1804375) / 0.95047;
  const y = r * 0.2126729 + g * 0.7151522 + b * 0.072175;
  const z = (r * 0.0193339 + g * 0.119192 + b * 0.9503041) / 1.08883;
  const f = (t: number): number => (t > 216 / 24389 ? Math.cbrt(t) : (841 / 108) * t + 4 / 29);
  const fx = f(x);
  const fy = f(y);
  const fz = f(z);
  return [116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)];
}

const radians = (degrees: number): number => (degrees * Math.PI) / 180;

/**
 * CIEDE2000, with kL = kC = kH = 1.
 *
 * Written from the CIE's own formulation. The three places an implementation goes wrong quietly,
 * and which the Sharma test data is built to catch, are all here: the hue **difference** wraps at
 * ±180°, the mean hue wraps differently again when the two angles straddle 360°, and both collapse
 * to a defined value when either chroma is zero — a neutral has no hue, and averaging the hue of a
 * grey with the hue of a green is how a formula ends up reporting a difference that is not there.
 */
export function deltaE2000(first: Lab, second: Lab): number {
  const [l1, a1, b1] = first;
  const [l2, a2, b2] = second;
  const c1 = Math.hypot(a1, b1);
  const c2 = Math.hypot(a2, b2);
  const meanChroma = (c1 + c2) / 2;
  const g = 0.5 * (1 - Math.sqrt(meanChroma ** 7 / (meanChroma ** 7 + 25 ** 7)));
  const ap1 = (1 + g) * a1;
  const ap2 = (1 + g) * a2;
  const cp1 = Math.hypot(ap1, b1);
  const cp2 = Math.hypot(ap2, b2);
  const angle = (a: number, b: number): number => {
    if (a === 0 && b === 0) return 0;
    const degrees = (Math.atan2(b, a) * 180) / Math.PI;
    return degrees < 0 ? degrees + 360 : degrees;
  };
  const hp1 = angle(ap1, b1);
  const hp2 = angle(ap2, b2);
  const deltaL = l2 - l1;
  const deltaC = cp2 - cp1;

  let deltaHueAngle: number;
  if (cp1 * cp2 === 0) deltaHueAngle = 0;
  else if (Math.abs(hp2 - hp1) <= 180) deltaHueAngle = hp2 - hp1;
  else if (hp2 - hp1 > 180) deltaHueAngle = hp2 - hp1 - 360;
  else deltaHueAngle = hp2 - hp1 + 360;
  const deltaH = 2 * Math.sqrt(cp1 * cp2) * Math.sin(radians(deltaHueAngle) / 2);

  const meanL = (l1 + l2) / 2;
  const meanCp = (cp1 + cp2) / 2;
  let meanHue: number;
  if (cp1 * cp2 === 0) meanHue = hp1 + hp2;
  else if (Math.abs(hp1 - hp2) <= 180) meanHue = (hp1 + hp2) / 2;
  else if (hp1 + hp2 < 360) meanHue = (hp1 + hp2 + 360) / 2;
  else meanHue = (hp1 + hp2 - 360) / 2;

  const t =
    1 -
    0.17 * Math.cos(radians(meanHue - 30)) +
    0.24 * Math.cos(radians(2 * meanHue)) +
    0.32 * Math.cos(radians(3 * meanHue + 6)) -
    0.2 * Math.cos(radians(4 * meanHue - 63));
  const hueRotation = 30 * Math.exp(-(((meanHue - 275) / 25) ** 2));
  const chromaTerm = 2 * Math.sqrt(meanCp ** 7 / (meanCp ** 7 + 25 ** 7));
  const sl = 1 + (0.015 * (meanL - 50) ** 2) / Math.sqrt(20 + (meanL - 50) ** 2);
  const sc = 1 + 0.045 * meanCp;
  const sh = 1 + 0.015 * meanCp * t;
  const rotation = -Math.sin(radians(2 * hueRotation)) * chromaTerm;

  return Math.sqrt(
    (deltaL / sl) ** 2 +
      (deltaC / sc) ** 2 +
      (deltaH / sh) ** 2 +
      rotation * (deltaC / sc) * (deltaH / sh),
  );
}

/** How far apart two colours look. `undefined` when either is not a colour. */
export function colourDistance(first: string, second: string): number | undefined {
  const a = toLab(first);
  const b = toLab(second);
  if (a === undefined || b === undefined) return undefined;
  return deltaE2000(a, b);
}

/** The closest pair in a set, and under which vision — the gate's own words for what it found. */
export interface ClosestPair {
  readonly first: number;
  readonly second: number;
  readonly vision: Vision;
  readonly distance: number;
}

/**
 * The smallest distance between any two of the colours, over **all four** ways of seeing them.
 *
 * Normal vision is swept as well as the three deficiencies, deliberately: a palette that is
 * separable only after simulation would be a palette nobody could use.
 */
export function closestPairUnderVision(colours: readonly string[]): ClosestPair | undefined {
  let closest: ClosestPair | undefined;
  for (let i = 0; i < colours.length; i += 1) {
    for (let j = i + 1; j < colours.length; j += 1) {
      for (const vision of visionNames) {
        const a = seenAs(colours[i] ?? '', vision);
        const b = seenAs(colours[j] ?? '', vision);
        if (a === undefined || b === undefined) continue;
        const distance = colourDistance(a, b);
        if (distance === undefined) continue;
        if (closest === undefined || distance < closest.distance) {
          closest = { first: i, second: j, vision, distance };
        }
      }
    }
  }
  return closest;
}

// ── the two naive answers, shipped so a gate has something to reject ───────────────────────────

/**
 * **The naive palette**: `count` hues, evenly spaced, at one saturation and one lightness.
 *
 * This is what an author-colour generator looks like when nobody has asked the CVD question, and it
 * is not a straw man — it is the shape of the answer in most editors that assign colours at all. It
 * scores 9.15 to a normal-vision reader, which is *better* than the palette this catalogue ships,
 * and 0.00 under deuteranopia, where its first two colours simulate to the same byte.
 */
export function naiveHueRotation(count: number, saturation = 0.62, lightness = 0.45): string[] {
  const colours: string[] = [];
  for (let index = 0; index < count; index += 1) {
    colours.push(hslToHex((360 / Math.max(1, count)) * index, saturation, lightness));
  }
  return colours;
}

function hslToHex(hue: number, saturation: number, lightness: number): string {
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const second = chroma * (1 - Math.abs(((hue / 60) % 2) - 1));
  const base = lightness - chroma / 2;
  let rgb: readonly [number, number, number];
  if (hue < 60) rgb = [chroma, second, 0];
  else if (hue < 120) rgb = [second, chroma, 0];
  else if (hue < 180) rgb = [0, chroma, second];
  else if (hue < 240) rgb = [0, second, chroma];
  else if (hue < 300) rgb = [second, 0, chroma];
  else rgb = [chroma, 0, second];
  return formatHexColor({
    red: Math.round((rgb[0] + base) * 255),
    green: Math.round((rgb[1] + base) * 255),
    blue: Math.round((rgb[2] + base) * 255),
  });
}

/**
 * **The naive assignment**: a slot from a hash of the author's name.
 *
 * FNV-1a, which is a perfectly good hash — the defect is not the hash, it is the idea. With eight
 * slots and eight authors the chance that no two collide is 8!/8⁸ ≈ 0.24%, so the interesting
 * question is not *whether* two authors in a real document share a colour but *how many* do.
 * `tests/annotation.test.ts` measures it over a large sample of plausible names rather than
 * quoting the arithmetic.
 */
export function naiveHashSlot(author: string, slots: number): number {
  let hash = 0x811c9dc5;
  const key = author.trim().toLocaleLowerCase();
  for (let index = 0; index < key.length; index += 1) {
    hash ^= key.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash % Math.max(1, slots);
}
