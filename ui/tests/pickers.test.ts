import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { ESLint } from 'eslint';
import tseslint from 'typescript-eslint';
import { describe, expect, test } from 'vitest';

import designValues from '../eslint-rules/design-values.js';
import { tokens, type ColorScheme } from '../tokens/tokens.ts';
import { contrastRatio, contrastRatioOrWorst, nonTextMinimum, parseHexColor } from '../src/tokens/contrast.ts';
import {
  applyLuminance,
  chooseSwatchIndicator,
  chooseSwatchIndicatorAmong,
  colorChoicesEqual,
  colorPickerCss,
  describeColorChoice,
  describeWorstIndicator,
  fontPreviewStack,
  fontsNeedingWarning,
  formatColorChoice,
  galleryThemeSlots,
  hslToRgb,
  indicatorSweepColors,
  isSubstituted,
  nextSwatchIndex,
  parseColorChoice,
  quoteFamily,
  resolveColorChoice,
  resolveThemeColor,
  rgbToHsl,
  substitutedFontsRemainChoosable,
  substitutionNote,
  substitutionWord,
  swatchCellBackground,
  swatchCellRingPairs,
  swatchGridColumns,
  swatchIndicatorCandidates,
  swatchKeyAction,
  swatchRowPlan,
  swatchSections,
  swatchStateCascade,
  swatchStateNames,
  swatchStateOf,
  swatchStates,
  swatchStatesCss,
  themeColorSlotNames,
  themeColorSlots,
  themeColorVariantNames,
  themeColorVariants,
  themeVariantWire,
  worstSwatchIndicator,
  type ColorChoice,
  type FontDescriptor,
  type SwatchAction,
  type ThemeColorPalette,
} from '../src/pickers/picker-model.ts';
import type { OptionDescriptor } from '../src/inputs/input-model.ts';

/**
 * The pickers, in Node.
 *
 * MJXOFF-187 names four traps and three of them are *arithmetic* rather than appearance, which is
 * why so much of this child's proof lives here rather than in a browser:
 *
 * * a colour picker's job is to return the colour a person chose — so every representation it
 *   accepts is round-tripped, including one that is on no grid;
 * * a theme colour is a **slot**, so a theme choice is asserted structurally to carry no literal,
 *   and the same choice is shown resolving to two different colours under two different documents
 *   while reporting one value;
 * * a selection indicator on an arbitrary colour cannot be a fixed colour, so the worst case is
 *   **measured over four thousand colours** rather than asserted from a table.
 *
 * That last one is the newest lesson in the programme and the one this child was warned about:
 * **a gate that compares the code to a model cannot tell you the model is wrong.** So the indicator
 * suite never asks "does the code agree with `swatchStates`"; it asks "what is the worst ratio this
 * rule actually produces", and separately proves that the four fixed alternatives all fail.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

// ── the value grammar ────────────────────────────────────────────────────────

/** Every choice the vocabulary can express, plus a lattice of literals. */
function everyChoice(): ColorChoice[] {
  const choices: ColorChoice[] = [{ kind: 'automatic' }, { kind: 'none' }];
  for (const slot of themeColorSlotNames) {
    for (const variant of themeColorVariantNames) choices.push({ kind: 'theme', slot, variant });
  }
  for (let red = 0; red <= 255; red += 51) {
    for (let green = 0; green <= 255; green += 51) {
      for (let blue = 0; blue <= 255; blue += 51) {
        const hex = `#${[red, green, blue].map((v) => v.toString(16).padStart(2, '0')).join('')}`;
        choices.push({ kind: 'literal', hex });
      }
    }
  }
  return choices;
}

describe('the colour value', () => {
  test('every choice the vocabulary can express survives a round trip', () => {
    const choices = everyChoice();
    // 2 + 12 slots × 6 variants + 6³ literals. Named so a shrinking fixture is a failure.
    expect(choices).toHaveLength(2 + 12 * 6 + 216);
    for (const choice of choices) {
      const written = formatColorChoice(choice);
      const read = parseColorChoice(written);
      expect(read, `${written} did not parse back`).toBeDefined();
      expect(read).toEqual(choice);
      expect(colorChoicesEqual(choice, read as ColorChoice)).toBe(true);
    }
  });

  test('one colour, four spellings, one canonical value — including one on no grid', () => {
    const spellings = ['#123457', '#123457'.toUpperCase(), '123457', 'rgb(18, 52, 87)', 'rgb(18 52 87)'];
    for (const spelling of spellings) {
      const parsed = parseColorChoice(spelling);
      expect(parsed, `${spelling} was refused`).toEqual({ kind: 'literal', hex: '#123457' });
      // And the canonical form re-parses to itself, which is what makes it canonical.
      expect(parseColorChoice(formatColorChoice(parsed as ColorChoice))).toEqual(parsed);
    }
  });

  test('the short hex form is expanded the way CSS expands it, not the way it looks', () => {
    expect(parseColorChoice('#abc')).toEqual({ kind: 'literal', hex: '#aabbcc' });
    expect(parseColorChoice('abc')).toEqual({ kind: 'literal', hex: '#aabbcc' });
  });

  test('hsl() goes through the same arithmetic in both directions', () => {
    const parsed = parseColorChoice('hsl(214, 65.7%, 20.6%)');
    expect(parsed?.kind).toBe('literal');
    // Whatever it resolves to, feeding the canonical form back must not move it — the property
    // that stops a colour drifting one channel every time somebody opens the dialog.
    const first = formatColorChoice(parsed as ColorChoice);
    expect(formatColorChoice(parseColorChoice(first) as ColorChoice)).toBe(first);
  });

  test('what it refuses, and each refusal is a refusal rather than a guess', () => {
    for (const text of [
      '',
      '   ',
      'chartreuse',
      '#12',
      '#1234567',
      'theme:accent7',
      'theme:accent1/lighter30',
      'theme:',
      'rgb(18, 52)',
      'rgb(300, 0, 0)',
      'rgb(18, 52, 87, 0.5)',
      'hsl(214, 150%, 20%)',
      'hsl(214, 66%, 20%, 0.2)',
      'rgb(a, b, c)',
    ]) {
      expect(parseColorChoice(text), `${text} was accepted`).toBeUndefined();
    }
  });

  test('an alpha of exactly 1 is not a refusal, because it is not a transparency', () => {
    expect(parseColorChoice('rgba(18, 52, 87, 1)')).toEqual({ kind: 'literal', hex: '#123457' });
  });

  test('automatic and none are not colours and are never spelled as one', () => {
    for (const text of ['automatic', 'auto', 'AUTOMATIC']) {
      expect(parseColorChoice(text)).toEqual({ kind: 'automatic' });
    }
    for (const text of ['none', 'No fill', 'transparent']) {
      expect(parseColorChoice(text)).toEqual({ kind: 'none' });
    }
    expect(formatColorChoice({ kind: 'automatic' })).toBe('automatic');
    expect(formatColorChoice({ kind: 'none' })).toBe('none');
  });

  test('a choice describes itself in words, and a theme choice describes itself as a slot', () => {
    expect(describeColorChoice({ kind: 'theme', slot: 'accent1', variant: 'base' })).toBe('Accent 1');
    expect(describeColorChoice({ kind: 'theme', slot: 'accent1', variant: 'lighter40' })).toBe(
      'Accent 1, Lighter 40%',
    );
    expect(describeColorChoice({ kind: 'literal', hex: '#123457' })).toBe('#123457');
  });
});

// ── a theme colour is a slot ─────────────────────────────────────────────────

const brandA: ThemeColorPalette = { accent1: '#2f5fb0', accent2: '#8c3f7d', background1: '#ffffff' };
const brandB: ThemeColorPalette = { accent1: '#a6572d', accent2: '#7d6a3a', background1: '#fffdf5' };

describe('a theme colour is a slot, never the colour it currently happens to be', () => {
  test('a parsed theme choice carries no literal at all — structurally, not by inspection', () => {
    for (const slot of themeColorSlotNames) {
      for (const variant of themeColorVariantNames) {
        const parsed = parseColorChoice(formatColorChoice({ kind: 'theme', slot, variant }));
        expect(parsed?.kind).toBe('theme');
        // The assertion that fires if somebody helpfully resolves it: a theme arm with a colour on
        // it would pass every equality test in this file and would still be the defect.
        expect(Object.keys(parsed ?? {}).sort()).toEqual(['kind', 'slot', 'variant']);
      }
    }
  });

  test('the same choice is two colours under two brands and one value under both', () => {
    const choice: ColorChoice = { kind: 'theme', slot: 'accent1', variant: 'lighter40' };
    const value = formatColorChoice(choice);
    const underA = resolveColorChoice(choice, brandA, undefined);
    const underB = resolveColorChoice(choice, brandB, undefined);
    expect(underA).toBeDefined();
    expect(underB).toBeDefined();
    expect(underA).not.toBe(underB);
    // One value. This is the whole rule: open the deck somewhere else and the run follows the
    // brand, because what was stored was the slot.
    expect(formatColorChoice(parseColorChoice(value) as ColorChoice)).toBe(value);
  });

  test('a slot the document does not define resolves to nothing rather than to our accent', () => {
    expect(resolveThemeColor('accent6', 'base', brandA)).toBeUndefined();
    expect(resolveThemeColor('accent1', 'base', {})).toBeUndefined();
    // And nothing anywhere in the model reaches for the platform's own palette to fill the hole.
    expect(resolveColorChoice({ kind: 'theme', slot: 'accent6', variant: 'base' }, brandA, '#000000'))
      .toBeUndefined();
  });

  test('a literal is a literal, and none paints nothing', () => {
    expect(resolveColorChoice({ kind: 'literal', hex: '#123457' }, brandA, undefined)).toBe('#123457');
    expect(resolveColorChoice({ kind: 'none' }, brandA, '#000000')).toBeUndefined();
    expect(resolveColorChoice({ kind: 'automatic' }, brandA, '#0f1729')).toBe('#0f1729');
  });

  test('the wire tokens are the schema’s, exactly, and never abbreviated further', () => {
    expect(themeColorSlots.background1.wire).toBe('bg1');
    expect(themeColorSlots.text1.wire).toBe('tx1');
    expect(themeColorSlots.background2.wire).toBe('bg2');
    expect(themeColorSlots.text2.wire).toBe('tx2');
    expect(themeColorSlots.hyperlink.wire).toBe('hlink');
    expect(themeColorSlots.followedHyperlink.wire).toBe('folHlink');
    for (let index = 1; index <= 6; index += 1) {
      expect(themeColorSlots[`accent${String(index)}` as 'accent1'].wire).toBe(`accent${String(index)}`);
    }
    // Every wire token is distinct: two slots sharing one is a document that loses a colour.
    const wires = themeColorSlotNames.map((slot) => themeColorSlots[slot].wire);
    expect(new Set(wires).size).toBe(wires.length);
  });

  test('the luminance ladder is PowerPoint’s own numbers, in thousandths', () => {
    expect(themeVariantWire('base')).toBe('');
    expect(themeVariantWire('lighter80')).toBe('lumMod 20000, lumOff 80000');
    expect(themeVariantWire('lighter40')).toBe('lumMod 60000, lumOff 40000');
    expect(themeVariantWire('darker25')).toBe('lumMod 75000');
    expect(themeVariantWire('darker50')).toBe('lumMod 50000');
  });

  test('the gallery draws ten slots and the two link colours are not among them', () => {
    expect(galleryThemeSlots).toHaveLength(swatchGridColumns);
    expect(galleryThemeSlots).not.toContain('hyperlink');
    expect(galleryThemeSlots).not.toContain('followedHyperlink');
  });
});

// ── the arithmetic underneath ────────────────────────────────────────────────

describe('HSL and luminance modulation', () => {
  test('a colour survives a trip through HSL and back, over the whole cube lattice', () => {
    for (let red = 0; red <= 255; red += 17) {
      for (let green = 0; green <= 255; green += 17) {
        for (let blue = 0; blue <= 255; blue += 17) {
          const channels = { red, green, blue };
          expect(hslToRgb(rgbToHsl(channels))).toEqual(channels);
        }
      }
    }
  });

  test('the base row is an identity, and the ladder actually goes both ways', () => {
    const base = '#2f5fb0';
    expect(applyLuminance(base, 100, 0)).toBe(base);
    const lighter = applyLuminance(base, 20, 80);
    const darker = applyLuminance(base, 50, 0);
    expect(lighter).toBeDefined();
    expect(darker).toBeDefined();
    const lightnessOf = (hex: string): number => rgbToHsl(parseHexColor(hex) ?? { red: 0, green: 0, blue: 0 }).lightness;
    expect(lightnessOf(lighter as string)).toBeGreaterThan(lightnessOf(base));
    expect(lightnessOf(darker as string)).toBeLessThan(lightnessOf(base));
  });

  test('the ladder is monotonic, so the six rows are six distinguishable steps', () => {
    const lightnesses = themeColorVariantNames.map((variant) => {
      const spec = themeColorVariants[variant];
      const resolved = applyLuminance('#2f5fb0', spec.luminanceModulation, spec.luminanceOffset) ?? '';
      return rgbToHsl(parseHexColor(resolved) ?? { red: 0, green: 0, blue: 0 }).lightness;
    });
    expect(new Set(lightnesses.map((value) => value.toFixed(4))).size).toBe(themeColorVariantNames.length);
  });

  test('modulation is a hue-preserving operation, which channel scaling would not be', () => {
    const base = '#c0392b';
    const lighter = applyLuminance(base, 60, 40) ?? '';
    const hueOf = (hex: string): number => rgbToHsl(parseHexColor(hex) ?? { red: 0, green: 0, blue: 0 }).hue;
    expect(hueOf(lighter)).toBeCloseTo(hueOf(base), 0);
  });
});

// ── the indicator, measured rather than declared ─────────────────────────────

describe('the swatch indicator', () => {
  const sweep = indicatorSweepColors();

  test('the sweep is the whole grey line and a cube lattice, not somebody’s palette', () => {
    expect(sweep.length).toBeGreaterThan(4000);
    expect(sweep).toContain('#000000');
    expect(sweep).toContain('#ffffff');
    // The analytic worst case for "the better of two extremes" lies on the grey line, so a sweep
    // that lost the greys would silently stop being able to find it.
    expect(sweep.filter((colour) => /^#(..)\1\1$/.test(colour)).length).toBeGreaterThanOrEqual(256);
  });

  for (const scheme of schemes) {
    test(`the worst indicator over ${String(sweep.length)} colours clears 3 : 1 in ${scheme}`, () => {
      const worst = worstSwatchIndicator(sweep, scheme);
      expect(worst, 'the sweep produced no measurement at all').toBeDefined();
      expect(
        (worst as { indicator: { ratio: number } }).indicator.ratio,
        `worst case was ${describeWorstIndicator(worst as Parameters<typeof describeWorstIndicator>[0])}`,
      ).toBeGreaterThanOrEqual(nonTextMinimum);
    });

    test(`every fixed alternative fails the same sweep in ${scheme}`, () => {
      // ⚠ **This is the failability proof, and it is the point of the whole design.** A fixed
      // indicator is invisible on a swatch of that colour, so each of these bottoms out at 1.00.
      // Without this the "clears 3 : 1" assertion above would be satisfied by a sweep that could
      // not detect anything.
      for (const member of ['accent', 'accentPressed', 'textPrimary', 'surface'] as const) {
        const fixed = tokens.theme[scheme][member];
        let lowest = Infinity;
        for (const swatch of sweep) lowest = Math.min(lowest, contrastRatioOrWorst(fixed, swatch));
        expect(lowest, `a fixed ${member} indicator would have passed`).toBeLessThan(nonTextMinimum);
      }
    });

    test(`a swatch is told from the popup's own surface, in ${scheme}`, () => {
      // Either the swatch differs from the surface it sits on, or its hairline does — the hairline
      // being the same measured member. The minimum of that maximum is what a person actually
      // needs, and it is a different question from the one above.
      const surface = tokens.theme[scheme][swatchCellBackground];
      let lowest = Infinity;
      let at = '';
      for (const swatch of sweep) {
        const member = chooseSwatchIndicator(swatch, scheme).member;
        const best = Math.max(
          contrastRatioOrWorst(swatch, surface),
          contrastRatioOrWorst(tokens.theme[scheme][member], surface),
        );
        if (best < lowest) {
          lowest = best;
          at = swatch;
        }
      }
      expect(lowest, `worst boundary was ${lowest.toFixed(2)} : 1 on ${at}`).toBeGreaterThanOrEqual(
        nonTextMinimum,
      );
    });

    test(`the cursor ring is a token against a token and clears 3 : 1 in ${scheme}`, () => {
      expect(swatchCellRingPairs.length).toBeGreaterThan(0);
      for (const [ring, on] of swatchCellRingPairs) {
        const ratio = contrastRatioOrWorst(tokens.theme[scheme][ring], tokens.theme[scheme][on]);
        expect(ratio, `${ring} on ${on} is ${ratio.toFixed(2)} : 1`).toBeGreaterThanOrEqual(nonTextMinimum);
      }
    });

    test(`that ring gate can fail: a subtle border on the same surface does not clear it, ${scheme}`, () => {
      const ratio = contrastRatioOrWorst(
        tokens.theme[scheme].borderSubtle,
        tokens.theme[scheme][swatchCellBackground],
      );
      expect(ratio).toBeLessThan(nonTextMinimum);
    });
  }

  test('an unreadable colour is the WORST ratio and not the best — U07’s second defect, inverted', () => {
    // A helper whose failure mode is "passes" makes every ceiling assertion vacuous. `Infinity`
    // here would have made every unmeasurable comparison count as clearing 3 : 1.
    expect(contrastRatio('not a colour', '#ffffff')).toBeUndefined();
    expect(contrastRatioOrWorst('not a colour', '#ffffff')).toBe(1);
    const chosen = chooseSwatchIndicator('not a colour', 'light');
    expect(chosen.ratio).toBe(1);
  });

  test('the candidates are the two ends of the scheme, and there are exactly two', () => {
    expect(swatchIndicatorCandidates).toEqual(['textPrimary', 'surface']);
  });

  test('the choice follows the host’s tokens, not the generated table', () => {
    // A host that re-themed the platform has changed the answer. A component measuring against the
    // shipped palette would be right for us and wrong for the palette on screen — the same class
    // of mistake as hard-coding a token, wearing a measurement's costume.
    const inverted = { textPrimary: '#ffffff', surface: '#000000' };
    expect(chooseSwatchIndicatorAmong('#ffffff', inverted).member).toBe('surface');
    expect(chooseSwatchIndicator('#ffffff', 'light').member).toBe('textPrimary');
  });
});

// ── the grid ─────────────────────────────────────────────────────────────────

function options(spec: readonly (readonly [string, number])[]): OptionDescriptor[] {
  const built: OptionDescriptor[] = [];
  for (const [category, count] of spec) {
    for (let index = 0; index < count; index += 1) {
      built.push({ value: `${category}-${String(index)}`, label: `${category} ${String(index)}`, category });
    }
  }
  return built;
}

/** Reset (2), theme (10 × 3 rows), standard (10), recent (3): the shape the popup actually has. */
const gridFixture = options([
  ['', 2],
  ['Theme colours', 30],
  ['Standard colours', 10],
  ['Recent colours', 3],
]);

describe('the swatch grid', () => {
  test('a section is as wide as it needs to be and never wider than ten', () => {
    const sections = swatchSections(gridFixture);
    expect(sections.map((section) => [section.name, section.start, section.end, section.columns])).toEqual([
      ['', 0, 2, 2],
      ['Theme colours', 2, 32, 10],
      ['Standard colours', 32, 42, 10],
      ['Recent colours', 42, 45, 3],
    ]);
  });

  test('the row plan is one heading per named section and then its rows of cells', () => {
    const rows = swatchRowPlan(swatchSections(gridFixture));
    expect(rows.filter((row) => row.kind === 'heading')).toHaveLength(3);
    // 1 reset row + 3 theme rows + 1 standard row + 1 recent row.
    expect(rows.filter((row) => row.kind === 'cells')).toHaveLength(6);
    // Every cell appears exactly once, in order — a plan that lost or duplicated one would draw a
    // grid whose arrow keys land on cells that are not where they look.
    const covered: number[] = [];
    for (const row of rows) {
      if (row.kind === 'cells') for (let index = row.start; index < row.end; index += 1) covered.push(index);
    }
    expect(covered).toEqual(gridFixture.map((_, index) => index));
  });

  test('the inline arrows move one cell and stop at the section’s ends, never wrapping', () => {
    const sections = swatchSections(gridFixture);
    expect(nextSwatchIndex('inlineNext', 0, sections)).toBe(1);
    // The end of the two-cell reset section: staying put, not falling into the theme block.
    expect(nextSwatchIndex('inlineNext', 1, sections)).toBe(1);
    expect(nextSwatchIndex('inlinePrevious', 0, sections)).toBe(0);
    expect(nextSwatchIndex('inlineNext', 44, sections)).toBe(44);
  });

  test('a block arrow moves a whole row within a section', () => {
    const sections = swatchSections(gridFixture);
    // Theme block starts at 2 and is ten wide.
    expect(nextSwatchIndex('rowNext', 2, sections)).toBe(12);
    expect(nextSwatchIndex('rowPrevious', 12, sections)).toBe(2);
  });

  test('a block arrow steps between sections, keeping the column', () => {
    const sections = swatchSections(gridFixture);
    // The last theme row is 22…31. Down from 25 lands on the standard row's fourth cell.
    expect(nextSwatchIndex('rowNext', 25, sections)).toBe(32 + 3);
    // Up from the standard row's fourth cell returns to the last theme row's fourth cell.
    expect(nextSwatchIndex('rowPrevious', 35, sections)).toBe(22 + 3);
    // And into a section too narrow to hold that column, the answer is its last cell.
    expect(nextSwatchIndex('rowNext', 39, sections)).toBe(44);
  });

  test('nothing wraps around either end of the whole grid', () => {
    const sections = swatchSections(gridFixture);
    expect(nextSwatchIndex('rowPrevious', 0, sections)).toBe(0);
    expect(nextSwatchIndex('rowNext', 44, sections)).toBe(44);
    expect(nextSwatchIndex('first', 44, sections)).toBe(0);
    expect(nextSwatchIndex('last', 0, sections)).toBe(44);
  });

  test('the page keys move by section, and the first press lands on the section’s own start', () => {
    const sections = swatchSections(gridFixture);
    expect(nextSwatchIndex('sectionNext', 0, sections)).toBe(2);
    expect(nextSwatchIndex('sectionPrevious', 12, sections)).toBe(2);
    expect(nextSwatchIndex('sectionPrevious', 2, sections)).toBe(0);
  });

  test('an empty grid answers -1 rather than throwing or landing somewhere', () => {
    expect(nextSwatchIndex('inlineNext', 0, [])).toBe(-1);
    expect(swatchSections([])).toEqual([]);
    expect(swatchRowPlan([])).toEqual([]);
  });

  test('the inline arrows mirror under right-to-left and the block arrows never do', () => {
    // ⚠ Asserted as a **disagreement** rather than as a landing place. U05's first diagonal test
    // passed with its tolerance removed, because the pointer finished inside the submenu either
    // way; an assertion that the two directions produce the same action for one key would fail on
    // a symmetric implementation, and one about where the cursor ends up would not.
    const ltr = { open: true, direction: 'ltr' } as const;
    const rtl = { open: true, direction: 'rtl' } as const;
    expect(swatchKeyAction('ArrowRight', ltr)).toBe('inlineNext');
    expect(swatchKeyAction('ArrowRight', rtl)).toBe('inlinePrevious');
    expect(swatchKeyAction('ArrowRight', ltr)).not.toBe(swatchKeyAction('ArrowRight', rtl));
    expect(swatchKeyAction('ArrowLeft', ltr)).not.toBe(swatchKeyAction('ArrowLeft', rtl));
    expect(swatchKeyAction('ArrowDown', ltr)).toBe(swatchKeyAction('ArrowDown', rtl));
    expect(swatchKeyAction('ArrowUp', ltr)).toBe(swatchKeyAction('ArrowUp', rtl));
  });

  test('a shut palette leaves the caret keys alone and still opens on the block arrows', () => {
    const shut = { open: false, direction: 'ltr' } as const;
    expect(swatchKeyAction('ArrowLeft', shut)).toBeUndefined();
    expect(swatchKeyAction('ArrowRight', shut)).toBeUndefined();
    expect(swatchKeyAction('Home', shut)).toBeUndefined();
    expect(swatchKeyAction('End', shut)).toBeUndefined();
    expect(swatchKeyAction('ArrowDown', shut)).toBe('open');
    expect(swatchKeyAction('ArrowUp', shut)).toBe('open');
    expect(swatchKeyAction('Escape', shut)).toBe('revert');
    expect(swatchKeyAction('Tab', shut)).toBe('leave');
  });

  test('every movement the key map can produce is one the index arithmetic answers', () => {
    const sections = swatchSections(gridFixture);
    const produced = new Set<SwatchAction>();
    for (const key of ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown']) {
      const action = swatchKeyAction(key, { open: true, direction: 'ltr' });
      if (action !== undefined) produced.add(action);
    }
    expect(produced.size).toBe(8);
    for (const action of produced) {
      const landed = nextSwatchIndex(action, 12, sections);
      expect(landed, `${action} landed outside the grid`).toBeGreaterThanOrEqual(0);
      expect(landed).toBeLessThan(gridFixture.length);
    }
  });
});

// ── the swatch's states ──────────────────────────────────────────────────────

describe('a swatch’s states', () => {
  test('a live cell’s three facts resolve to exactly one state', () => {
    expect(swatchStateOf({ selected: false, active: false, unavailable: false })).toBe('rest');
    expect(swatchStateOf({ selected: false, active: true, unavailable: false })).toBe('active');
    expect(swatchStateOf({ selected: true, active: false, unavailable: false })).toBe('selected');
    expect(swatchStateOf({ selected: true, active: true, unavailable: false })).toBe('selectedActive');
    // Unavailable wins over both, which is what makes a refused choice that is also chosen read as
    // refused rather than as chosen.
    expect(swatchStateOf({ selected: true, active: true, unavailable: true })).toBe('unavailable');
  });

  test('the cascade is exactly the states, once each, with unavailable last', () => {
    expect([...swatchStateCascade].sort()).toEqual([...swatchStateNames].sort());
    expect(swatchStateCascade.at(-1)).toBe('unavailable');
  });

  test('every state describes itself and templates its own selector', () => {
    for (const state of swatchStateNames) {
      const spec = swatchStates[state];
      expect(spec.description.length).toBeGreaterThan(20);
      expect(spec.matches.length).toBeGreaterThan(0);
      for (const match of spec.matches) expect(match).toContain('%s');
    }
  });

  test('a state whose indicator cannot be decided here declares a non-colour cue', () => {
    // The two states carrying the *measured* ring, plus the refused one. Their contrast is not
    // decidable at build time, so each says what tells it apart without colour — and the browser
    // gate asserts each declared cue is actually drawn.
    for (const state of swatchStateNames) {
      const spec = swatchStates[state];
      if (spec.insetRing || state === 'unavailable') {
        expect(spec.nonColourCue, `${state} declares no cue`).toBeDefined();
      }
    }
  });

  test('the two rings are two rings: no state draws the measured one outside the square', () => {
    for (const state of swatchStateNames) {
      const spec = swatchStates[state];
      if (spec.cellRing === 'transparent') continue;
      // A cell ring is a token, so it is nameable in the generated table. The measured ring never
      // is, which is why it is a custom property and not a member.
      expect(Object.keys(tokens.theme.light)).toContain(spec.cellRing);
    }
  });
});

// ── the stylesheets ──────────────────────────────────────────────────────────

describe('the picker stylesheets', () => {
  const stateCss = swatchStatesCss('.swatch');

  /**
   * Every rule block in a stylesheet string, comments removed.
   *
   * ⚠ The comment stripping is not tidiness: without it a `split(';')` walks straight into the
   * prose explaining a declaration, and `box-shadow` disappears from a block that plainly has one.
   * A parser whose failure mode is *"found no declarations"* makes every assertion about a block
   * pass — U07's `tabStops` defect in a stylesheet's costume, and it cost two red runs here before
   * it was the parser rather than the CSS that was wrong.
   */
  function blocks(css: string): { selector: string; properties: string[] }[] {
    const bare = css.replaceAll(/\/\*[\s\S]*?\*\//g, ' ');
    const found: { selector: string; properties: string[] }[] = [];
    for (const match of bare.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const properties = (match[2] ?? '')
        .split(';')
        .map((declaration) => declaration.split(':')[0]?.trim() ?? '')
        .filter((property) => property !== '');
      found.push({ selector: (match[1] ?? '').trim(), properties });
    }
    return found;
  }

  /** The properties one selector's own block writes. */
  function propertiesOf(css: string, selector: string): string[] {
    return blocks(css)
      .filter((block) => block.selector === selector)
      .flatMap((block) => block.properties);
  }

  /**
   * Every property the state table writes **on the cell itself**.
   *
   * Derived, so a state that grows a new property is covered the day it is added — and restricted
   * to blocks that target the cell rather than something inside it, because the mark's own
   * `display` is not a property the base `.swatch` block is forbidden to set.
   */
  function stateProperties(): string[] {
    const found = new Set<string>();
    for (const block of blocks(stateCss)) {
      const trailing = block.selector.replace(/^:where\([\s\S]*\)/, '').trim();
      if (trailing !== '') continue;
      for (const property of block.properties) found.add(property);
    }
    return [...found];
  }

  test('the base rule for a swatch writes none of the properties its state table writes', () => {
    // ⚠ MJXOFF-183's specificity accident, prevented rather than described: a `border-color:` in
    // the base `.swatch` block would score (0,1,0) and out-specify every `:where()` rule in the
    // table, so the cursor ring would simply never appear and the emission-order test would stay
    // green — emission order only arbitrates between rules of *equal* specificity.
    const written = stateProperties();
    expect([...written].sort()).toEqual(['border-color', 'border-style', 'box-shadow']);
    const base = propertiesOf(colorPickerCss, '.swatch');
    expect(base.length).toBeGreaterThan(0);
    expect(base.filter((property) => written.includes(property))).toEqual([]);
  });

  test('the elevation and the field chip are deliberately not in that claim', () => {
    // The popup's own shadow and the field's preview chip *do* carry border and shadow
    // declarations, and must: they are not states of anything, and no `:where()` rule targets
    // them. Saying so here is what stops the assertion above being quietly widened until it fails
    // for a reason that is not a defect — the way a rule that cries wolf gets disabled.
    expect(propertiesOf(colorPickerCss, '.preview')).toContain('border-color');
    expect(propertiesOf(colorPickerCss, '.palette')).toContain('box-shadow');
    expect(propertiesOf(colorPickerCss, '.swatch-paint')).toContain('box-shadow');
  });

  test('every state rule is wrapped in :where(), so source order is not the cascade', () => {
    const selectors = stateCss.split('\n').filter((line) => line.trim().endsWith('{'));
    expect(selectors.length).toBeGreaterThan(0);
    for (const selector of selectors) expect(selector.trim()).toMatch(/^:where\(/);
  });

  test('the rules come out in cascade order', () => {
    const positions = swatchStateCascade.map((state) => {
      const first = swatchStates[state].matches[0]?.replaceAll('%s', '.swatch') ?? '';
      return stateCss.indexOf(first);
    });
    for (const position of positions) expect(position).toBeGreaterThanOrEqual(0);
    expect([...positions].sort((a, b) => a - b)).toEqual(positions);
  });

  test('the measured ring is a custom property everywhere it appears, never a token', () => {
    const ringLines = stateCss.split('\n').filter((line) => line.includes('inset 0 0 0'));
    expect(ringLines.length).toBeGreaterThan(0);
    for (const line of ringLines) expect(line).toContain('var(--mjx-swatch-indicator)');
  });
});

// ── the font picker ──────────────────────────────────────────────────────────

const installed: FontDescriptor = { family: 'Georgia', availability: 'installed' };
const compatible: FontDescriptor = {
  family: 'Cambria',
  availability: 'substituted',
  substitutedBy: 'Caladea',
  metricCompatible: true,
};
const incompatible: FontDescriptor = {
  family: 'Constantia',
  availability: 'substituted',
  substitutedBy: 'Gelasio',
  metricCompatible: false,
};
const anonymous: FontDescriptor = { family: 'Bodoni MT', availability: 'substituted' };
const missing: FontDescriptor = { family: 'Wingdings', availability: 'missing' };

describe('the substitution warning', () => {
  test('a warning appears for a substituted face and does not for a present one', () => {
    // The headline assertion of this control, in one line each direction.
    expect(substitutionNote(installed)).toBeUndefined();
    expect(substitutionWord(installed)).toBeUndefined();
    for (const font of [compatible, incompatible, anonymous, missing]) {
      expect(substitutionNote(font), `${font.family} carries no note`).toBeDefined();
      expect(substitutionWord(font), `${font.family} carries no word`).toBeDefined();
    }
  });

  test('the sentence leads with the half that decides whether the page repaginates', () => {
    expect(substitutionNote(compatible)).toContain('Caladea');
    expect(substitutionNote(compatible)).toContain('metrics match');
    expect(substitutionNote(incompatible)).toContain('metrics differ');
    expect(substitutionNote(incompatible)).toContain('rewrap');
    // Two different facts must not produce one sentence: a metric-compatible substitution and one
    // that reflows the document are the same word and opposite consequences.
    expect(substitutionNote(compatible)).not.toBe(substitutionNote(incompatible));
  });

  test('a stand-in nobody named still gets a sentence', () => {
    expect(substitutionNote(anonymous)).toContain('does not have the face');
  });

  test('missing is a stronger statement than substituted, and says so', () => {
    expect(substitutionWord(missing)).toBe('Missing');
    expect(substitutionWord(compatible)).toBe('Substituted');
    expect(substitutionNote(missing)).toContain('fallback');
  });

  test('the families needing a warning are exactly the ones that are not installed', () => {
    const fonts = [installed, compatible, incompatible, anonymous, missing];
    expect(fontsNeedingWarning(fonts).map((font) => font.family)).toEqual(
      fonts.filter((font) => isSubstituted(font)).map((font) => font.family),
    );
    expect(fontsNeedingWarning(fonts)).toHaveLength(4);
  });

  test('a substituted family is previewed in what it will actually be drawn in', () => {
    // ⚠ The one place a preview could lie. A browser asked for a face it has not got falls back
    // silently, so previewing the *family* would draw the fallback and make the row look installed.
    expect(fontPreviewStack(installed)).toContain(quoteFamily('Georgia'));
    expect(fontPreviewStack({ ...compatible })).toContain(quoteFamily('Caladea'));
    expect(fontPreviewStack({ ...compatible })).not.toContain(quoteFamily('Cambria'));
    // A declared stack wins, because a host that has said which faces to draw with has said it.
    expect(fontPreviewStack({ ...compatible, stack: 'serif' })).toBe('serif');
  });

  test('a family name is quoted, because a bare Segoe UI is two idents and a parse error', () => {
    expect(quoteFamily('Segoe UI')).toBe('"Segoe UI"');
    expect(quoteFamily('He said "hi"')).toBe('"He said \\"hi\\""');
  });

  test('a substituted family stays choosable, which is the decision the control is built on', () => {
    expect(substitutedFontsRemainChoosable).toBe(true);
    // And nothing in the model marks one unavailable: refusing would silently rewrite a document
    // that legitimately asks for a face this machine has not got.
    for (const font of [compatible, incompatible, anonymous, missing]) {
      expect(Object.hasOwn(font, 'unavailable')).toBe(false);
    }
  });
});

// ── the component ships no colours ───────────────────────────────────────────

describe('the pickers ship no colours of their own', () => {
  function linter(): ESLint {
    return new ESLint({
      overrideConfigFile: true,
      overrideConfig: [
        {
          files: ['**/*.ts'],
          languageOptions: { parser: tseslint.parser, ecmaVersion: 2023, sourceType: 'module' },
          plugins: { mjx: designValues },
          rules: { 'mjx/no-literal-design-values': 'error' },
        },
      ],
    });
  }

  test('every source under src/pickers/ is clean, measured with the real rule', async () => {
    const directory = resolve(import.meta.dirname, '../src/pickers');
    const files = readdirSync(directory).filter((name) => name.endsWith('.ts'));
    // A directory that had become empty would make this whole suite vacuous.
    expect(files.length).toBeGreaterThanOrEqual(4);
    for (const name of files) {
      const source = readFileSync(resolve(directory, name), 'utf8');
      const [result] = await linter().lintText(source, { filePath: name });
      const messages = (result?.messages ?? []).map((message) => `${name}: ${message.message}`);
      expect(messages).toEqual([]);
    }
  });

  test('the rule would have fired on a picker that shipped a palette', async () => {
    // The failability half. This is what a colour picker looks like when it decides what the
    // standard row is instead of being told — and it is the exact mistake the architecture of this
    // child was arranged to make impossible.
    const source = "export const standardColors = ['#c00000', '#ff0000'];";
    const [result] = await linter().lintText(source, { filePath: 'color-picker.ts' });
    expect((result?.messages ?? []).map((message) => message.messageId)).toEqual([
      'literalColor',
      'literalColor',
    ]);
  });
});
