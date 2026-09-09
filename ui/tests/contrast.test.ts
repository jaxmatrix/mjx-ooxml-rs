import { describe, expect, it } from 'vitest';

import {
  bodyTextMinimum,
  contrastExemplars,
  contrastRatio,
  formatRatio,
  measurePalette,
  relativeLuminance,
} from '../dev/contrast.ts';
import { tokens } from '../tokens/tokens.ts';

/**
 * The arithmetic behind the a11y gate.
 *
 * The browser tier asserts that **axe agrees with this**; that comparison is only worth anything if
 * this half is independently right, so it is pinned here against WCAG's own worked values rather
 * than against tokens. The literal colours below are the specification's endpoints and a
 * mid-grey — they are not design tokens and must not become any.
 */
describe('contrast arithmetic', () => {
  it('matches WCAG at the endpoints', () => {
    expect(relativeLuminance('#000000')).toBeCloseTo(0, 10);
    expect(relativeLuminance('#ffffff')).toBeCloseTo(1, 10);
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 6);
    expect(contrastRatio('#ffffff', '#ffffff')).toBeCloseTo(1, 6);
  });

  it('is symmetric, because a ratio has no direction', () => {
    expect(contrastRatio('#2e9e63', '#ffffff')).toBeCloseTo(
      contrastRatio('#ffffff', '#2e9e63') ?? 0,
      12,
    );
  });

  it('reproduces the two figures DESIGN_TOKENS.md §2.2 measured', () => {
    // §2.2 states 3.39 : 1 and 5.34 : 1 for #2e9e63 and #1e7a49 on white. Those hexes are quoted
    // from the document, not read from the palette: this test checks the *arithmetic*, and it must
    // keep checking it after the palette is re-seeded and those two values are no longer tokens.
    expect(formatRatio(contrastRatio('#2e9e63', '#ffffff') ?? 0)).toBe('3.39 : 1');
    expect(formatRatio(contrastRatio('#1e7a49', '#ffffff') ?? 0)).toBe('5.34 : 1');
  });

  it('ignores an alpha channel rather than refusing the colour', () => {
    expect(contrastRatio('#2e9e6380', '#ffffff')).toBeCloseTo(
      contrastRatio('#2e9e63', '#ffffff') ?? 0,
      12,
    );
  });

  it('refuses a value that is not an opaque hex colour', () => {
    expect(relativeLuminance('cubic-bezier(0.45, 0, 0.2, 1)')).toBeUndefined();
    expect(contrastRatio('16px', '#ffffff')).toBeUndefined();
  });
});

describe('the gate exemplars', () => {
  const background = tokens.theme.light.surface;

  it('measures every colour token in the generated palette and nothing else', () => {
    const measured = measurePalette(background);
    expect(measured.length).toBeGreaterThan(0);
    expect(measured.every((row) => row.path.startsWith('color.'))).toBe(true);
    // Ordered by ratio, so "the closest miss" and "the narrowest pass" are the ends of two runs.
    for (let index = 1; index < measured.length; index += 1) {
      expect(measured[index]?.ratio).toBeGreaterThanOrEqual(measured[index - 1]?.ratio ?? 0);
    }
  });

  it('chooses a rejected exemplar that really fails and an accepted one that really passes', () => {
    const exemplars = contrastExemplars(background);
    expect(exemplars.rejected.ratio).toBeLessThan(bodyTextMinimum);
    expect(exemplars.accepted.ratio).toBeGreaterThanOrEqual(bodyTextMinimum);
  });

  it('chooses the closest miss and the narrowest pass, not an arbitrary pair', () => {
    // The closest miss is the most plausible mistake a designer could make, and the narrowest pass
    // is the one whose legality is least obvious. An arbitrary pair would demonstrate the gate
    // just as well and teach nothing.
    const exemplars = contrastExemplars(background);
    const measured = measurePalette(background);
    const failing = measured.filter((row) => row.ratio < bodyTextMinimum);
    const passing = measured.filter((row) => row.ratio >= bodyTextMinimum);
    if (failing.length > 0) {
      expect(exemplars.rejected.path).toBe(failing.at(-1)?.path);
      expect(exemplars.rejectedIsSynthetic).toBe(false);
    }
    expect(exemplars.accepted.path).toBe(passing[0]?.path);
  });

  it('produces a genuinely failing exemplar against a dark background too', () => {
    // The exemplars are computed against whatever background they are given, so a dark-scheme
    // story would get a dark-scheme pair rather than the light one relabelled.
    const exemplars = contrastExemplars(tokens.theme.dark.surface);
    expect(exemplars.rejected.ratio).toBeLessThan(bodyTextMinimum);
    expect(exemplars.rejected.legalAsBodyText).toBe(false);
    expect(exemplars.accepted.ratio).toBeGreaterThanOrEqual(bodyTextMinimum);
  });

  it('keeps a stand-in that fails, for the day the palette becomes fully legible', () => {
    // A palette in which every colour clears 4.5 : 1 is a good palette and a gate with nothing to
    // reject. `MachineryCanReject` uses a colour that is not a token for exactly that reason, and
    // the stand-in inside contrastExemplars is the same idea applied to the token story. Both are
    // this grey; if it ever stopped failing, both demonstrations would quietly become vacuous.
    expect(contrastRatio('#949494', '#ffffff') ?? 0).toBeLessThan(bodyTextMinimum);
  });
});
