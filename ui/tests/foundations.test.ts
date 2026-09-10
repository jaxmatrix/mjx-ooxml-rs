import { describe, expect, it } from 'vitest';

import { tokens, customProperties } from '../tokens/tokens.ts';
import { contrastRatio } from '../dev/contrast.ts';
import {
  typeRoleClass,
  typeRoleNames,
  typeRoles,
  typographyCss,
} from '../src/foundations/typography.ts';
import {
  radiusSteps,
  surfaceBackgroundMember,
  surfaceLevelNames,
  surfaceLevels,
  surfacesCss,
} from '../src/foundations/surfaces.ts';
import {
  focusIndicatorMinimumContrast,
  focusRingDefaults,
  focusCss,
} from '../src/foundations/focus.ts';
import {
  motionRoleNames,
  motionRoles,
  overshootingEasingToken,
  motionCss,
} from '../src/foundations/motion.ts';
import {
  accessibleHitTargetMinimum,
  densityModeNames,
  densityModes,
  densityCss,
} from '../src/foundations/density.ts';
import { foundationsCss } from '../src/foundations/stylesheet.ts';
import type { ThemeColors } from '../tokens/tokens.ts';

/**
 * The foundations, checked where no browser is needed.
 *
 * The browser tier proves that the *cascade* delivers these values; this tier proves that the
 * values are the right ones, and it is where three rules that have only ever been prose become
 * failures:
 *
 * | Rule | Source |
 * |---|---|
 * | A dense surface is `--text-xs` on `--leading-tight` | `DESIGN_TOKENS.md` §4 |
 * | Young Serif is display-only | `DESIGN_TOKENS.md` §4 |
 * | Nothing attached to a document object may overshoot | `DESIGN_TOKENS.md` §4 |
 *
 * …and where the one number MJXOFF-181 asks to be *measured* is measured: the focus ring's
 * contrast against **every rung of the elevation ladder in both schemes**, because *"a ring that
 * passes on white can vanish on the dark backdrop"* and *"a green ring on a green tint is the
 * predictable failure."*
 *
 * ⚠ **No token name and no colour is written in this file as a literal.** Every value is read from
 * the generated table, because the palette is about to be re-seeded from another product and a
 * gate that named a hex would break on the day it is most needed.
 */

/**
 * `surfaceRaised` → `surface-raised`, so a scheme member can be turned back into its alias.
 */
function aliasCase(member: string): string {
  return member.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`);
}

/**
 * Every custom property a component may legally write.
 *
 * Two families, and conflating them is the mistake this helper exists to prevent. A **token
 * property** is `--theme-light-surface`, emitted once per scheme by the generator. A **scheme
 * alias** is `--theme-surface`, which `tokens.css` points at one of them through its three
 * `:root` rules — and it is the alias, not the token property, that a component must write, because
 * writing the token property directly would give the chrome one theme for ever.
 *
 * So a gate that checked component CSS against `customProperties` alone would reject exactly the
 * spelling the architecture requires. Both are computed here from the generated table; neither is
 * written down.
 */
const legalCustomProperties = new Set<string>([
  ...Object.values(customProperties),
  ...Object.keys(tokens.theme.light).map((member) => `--theme-${aliasCase(member)}`),
  ...Object.keys(tokens.document.light).map((member) => `--document-${aliasCase(member)}`),
]);

/** The `var(--…)` a spec writes → the token path the generated table keys on. */
function tokenPathFor(variable: string): string | undefined {
  const property = /^var\((--[a-z0-9-]+)\)$/.exec(variable.trim())?.[1];
  if (property === undefined) return undefined;
  return Object.keys(customProperties).find((path) => customProperties[path] === property);
}

describe('the type scale', () => {
  it('names a real token in every var() it writes', () => {
    for (const role of typeRoleNames) {
      const spec = typeRoles[role];
      for (const [field, value] of Object.entries(spec)) {
        if (field === 'use') continue;
        for (const match of value.matchAll(/var\((--[a-z0-9-]+)\)/g)) {
          expect(
            legalCustomProperties,
            `${role}.${field} names ${String(match[1])}, which no token declares`,
          ).toContain(match[1]);
        }
      }
    }
  });

  it('sets dense to text.xs on leading.tight, which is what §4 corrects', () => {
    // The correction the whole child is built around: Nunito Sans reads wide, so a formula bar,
    // a cell grid and an inspector row use the small size on the tight measure — never the source
    // site's relaxed 1.7. Written as a test because it is the rule a later child will forget.
    expect(tokenPathFor(typeRoles.dense.size)).toBe('text.xs');
    expect(tokenPathFor(typeRoles.dense.leading)).toBe('leading.tight');
    expect(tokenPathFor(typeRoles.body.leading)).toBe('leading.body');
    expect(tokens.leading.body).toBeGreaterThan(tokens.leading.tight);
  });

  it('gives the serif to display and to nothing else', () => {
    const serif = customProperties['font.serif'];
    expect(serif).toBeDefined();
    for (const role of typeRoleNames) {
      const usesSerif = typeRoles[role].family.includes(String(serif));
      expect(usesSerif, `${role} names the serif; §4 says display-only`).toBe(role === 'display');
    }
  });

  it('is a scale: no two roles are the same size, weight and measure at once', () => {
    const fingerprints = typeRoleNames.map((role) => {
      const spec = typeRoles[role];
      return `${spec.size}|${spec.leading}|${spec.weight}|${spec.family}|${spec.tracking}`;
    });
    // Two roles may share a size — `dense` and `label` do — but not everything. A scale in which
    // two roles are indistinguishable is a scale with a name too many.
    expect(new Set(fingerprints).size).toBe(fingerprints.length);
  });

  it('emits a rule for every role, under the class the helper computes', () => {
    for (const role of typeRoleNames) {
      expect(typographyCss).toContain(`.${typeRoleClass(role)}`);
    }
    expect(typeRoleClass('paneTitle')).toBe('mjx-type-pane-title');
  });
});

describe('the elevation ladder', () => {
  it('names a real token in every var() it writes', () => {
    for (const level of surfaceLevelNames) {
      const spec = surfaceLevels[level];
      for (const value of [spec.background, spec.border, spec.shadow]) {
        for (const match of value.matchAll(/var\((--[a-z0-9-]+)\)/g)) {
          expect(
            legalCustomProperties,
            `${level} writes ${String(match[1])}, which is neither a generated token property nor ` +
              'a scheme alias',
          ).toContain(match[1]);
        }
      }
      expect(radiusSteps).toContain(spec.radius);
    }
  });

  it('carries an ink-tinted shadow and never a neutral one', () => {
    // MJXOFF-181: "a shadow that renders as flat black is wrong in a way a snapshot will happily
    // lock in." The property that a neutral black cannot have, whatever the palette becomes, is
    // that its three channels differ — so that is what is asserted, on the generated token itself.
    const lift = tokens.shadow.lift;
    const colour = /#([0-9a-fA-F]{6})([0-9a-fA-F]{2})?/.exec(lift)?.[1];
    expect(colour, `shadow.lift is '${lift}', which carries no hex colour`).toBeDefined();
    const value = Number.parseInt(colour ?? '000000', 16);
    const channels = [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
    expect(new Set(channels).size, `shadow.lift is achromatic: ${lift}`).toBeGreaterThan(1);
  });

  it('composes its top rung from the one shadow token rather than inventing a second', () => {
    expect(surfaceLevels.overlay.shadow).toBe(
      `${surfaceLevels.floating.shadow}, ${surfaceLevels.floating.shadow}`,
    );
  });

  it('says which theme member is behind every rung, so the focus gate can measure it', () => {
    for (const level of surfaceLevelNames) {
      const member = surfaceBackgroundMember[level];
      expect(Object.keys(tokens.theme.light), `${level} names no theme member`).toContain(member);
      // …and the ladder's own declaration must agree with it, or the gate would be measuring
      // against a colour the surface does not have.
      expect(surfaceLevels[level].background).toContain(
        `--theme-${member.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`)}`,
      );
    }
  });

  it('emits a rule for every rung and every radius step', () => {
    for (const level of surfaceLevelNames) expect(surfacesCss).toContain(`.mjx-surface-${level}`);
    for (const step of radiusSteps) expect(surfacesCss).toContain(`.mjx-radius-${step}`);
  });
});

describe('the focus ring', () => {
  it('is visible on every rung of the ladder, in both schemes', () => {
    // ⚠ This is the assertion MJXOFF-181 asks for by name, and it is arithmetic rather than a
    // screenshot: WCAG 2.2's non-text contrast minimum, measured between the ring and the surface
    // behind it, for every rung and both schemes — the accent rungs included, because "a green
    // ring on a green tint" is the failure the ticket predicts.
    //
    // If a palette re-seed ever breaks a pair, this fails and names the pair. That is correct: an
    // invisible focus ring is a defect whether or not anybody chose it.
    const failures: string[] = [];
    for (const scheme of ['light', 'dark'] as const) {
      const palette: ThemeColors = tokens.theme[scheme];
      const ring = palette[focusRingDefaults.colorTokenMember as keyof ThemeColors];
      for (const level of surfaceLevelNames) {
        const behind = palette[surfaceBackgroundMember[level] as keyof ThemeColors];
        const ratio = contrastRatio(String(ring), String(behind));
        expect(ratio, `${scheme}/${level}: one of the two colours is not an opaque hex`).toBeDefined();
        if ((ratio ?? 0) < focusIndicatorMinimumContrast) {
          failures.push(
            `${scheme} · ${level}: ring ${String(ring)} on ${String(behind)} measures ` +
              `${(ratio ?? 0).toFixed(2)} : 1, under ${String(focusIndicatorMinimumContrast)} : 1`,
          );
        }
      }
    }
    expect(failures, failures.join('\n')).toEqual([]);
  });

  it('would notice an invisible ring, rather than passing whatever it is given', () => {
    // The other direction, which is what tells a working measurement apart from a switched-off
    // one: a ring painted in the surface's own colour must fail. No token is named — the colour is
    // taken from the palette and measured against itself.
    for (const scheme of ['light', 'dark'] as const) {
      const surface = tokens.theme[scheme].surface;
      expect(contrastRatio(surface, surface)).toBe(1);
      expect(contrastRatio(surface, surface) ?? 0).toBeLessThan(focusIndicatorMinimumContrast);
    }
  });

  it('is keyboard-only, and says so in the stylesheet rather than in a comment', () => {
    expect(focusCss).toContain(':focus:not(:focus-visible)');
    expect(focusCss).toContain(':focus-visible');
  });

  it('derives its geometry from --spacing rather than from a number', () => {
    expect(focusRingDefaults.width).toContain('var(--spacing)');
    expect(focusRingDefaults.offset).toContain('var(--spacing)');
    expect(tokenPathFor(focusRingDefaults.color)).toBeUndefined();
    // …because the colour is the *scheme-relative alias* `--theme-accent-pressed`, which is not a
    // token path but a `tokens.css` alias onto one. That is deliberate: naming the scheme here
    // would give the chrome one theme.
    expect(focusRingDefaults.color).toBe('var(--theme-accent-pressed)');
  });
});

describe('the motion vocabulary', () => {
  it('never gives an overshooting easing to something attached to a document object', () => {
    // DESIGN_TOKENS.md §4, made a test. This is the rule a later child animating a selection
    // handle will otherwise break, and the sentence in a design document would not have stopped
    // them.
    for (const role of motionRoleNames) {
      const spec = motionRoles[role];
      if (!spec.attachedToDocumentObject) continue;
      expect(
        spec.easingToken,
        `${role} is attached to a document object and uses ${spec.easingToken}; §4: overshoot ` +
          'there reads as imprecision.',
      ).not.toBe(overshootingEasingToken);
    }
  });

  it('uses the overshooting easing somewhere, or the rule above proves nothing', () => {
    // A vocabulary in which no role overshoots would satisfy the assertion above trivially. This
    // is the second direction: the token exists, it is used, and it is used only on chrome.
    const overshooting = motionRoleNames.filter(
      (role) => motionRoles[role].easingToken === overshootingEasingToken,
    );
    expect(overshooting.length).toBeGreaterThan(0);
    for (const role of overshooting) {
      expect(motionRoles[role].attachedToDocumentObject).toBe(false);
    }
  });

  it('names a real easing token in every role', () => {
    for (const role of motionRoleNames) {
      const spec = motionRoles[role];
      expect(Object.keys(customProperties)).toContain(spec.easingToken);
      expect(spec.easing).toBe(`var(${String(customProperties[spec.easingToken])})`);
      expect(tokenPathFor(spec.duration)).toBe('duration.transition');
    }
  });

  it('honours a reduced-motion preference once, for everything that carries a motion class', () => {
    expect(motionCss).toContain('prefers-reduced-motion');
  });
});

describe('density', () => {
  it('changes the spacing between the modes', () => {
    // Half of the ticket's gate. A compact mode that changed nothing would pass the hit-target
    // assertion below and fail this one, which is what makes the pair non-vacuous.
    expect(densityModes.compact.stepUnits).toBeLessThan(densityModes.comfortable.stepUnits);
    expect(densityModes.compact.gutterUnits).toBeLessThan(densityModes.comfortable.gutterUnits);
  });

  it('keeps every mode above the accessible minimum, in the units the tokens are in', () => {
    // `--spacing` is a rem value; the floor is a pixel one. The conversion is the browser's, so
    // the *computed* assertion lives in tests/browser/foundations.spec.ts — what is checked here
    // is the arithmetic at the default root size, which is the case a re-seed would move.
    const spacingPixels = Number.parseFloat(tokens.spacing) * 16;
    expect(Number.isFinite(spacingPixels)).toBe(true);
    for (const mode of densityModeNames) {
      const target = densityModes[mode].hitTargetUnits * spacingPixels;
      expect(
        target,
        `${mode} asks for ${String(target)}px, under WCAG 2.2's ${String(accessibleHitTargetMinimum)}px minimum`,
      ).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }
  });

  it('applies the floor in the stylesheet rather than trusting the multipliers', () => {
    // Even if a future mode chose a multiplier under the floor, the `max()` below stops it. That
    // is the difference between a documented minimum and an enforced one.
    expect(densityCss).toContain(`max(var(${'--mjx-hit-target'})`);
    expect(densityCss).toContain(`${String(accessibleHitTargetMinimum)}px`);
  });

  it('writes every length as a multiple of --spacing', () => {
    for (const mode of densityModeNames) {
      for (const units of Object.values(densityModes[mode])) {
        if (typeof units !== 'number') continue;
        expect(Number.isInteger(units)).toBe(true);
      }
    }
  });
});

describe('the composed stylesheet', () => {
  it('contains all five parts, so a component that installs it gets all five', () => {
    for (const part of [typographyCss, surfacesCss, densityCss, focusCss, motionCss]) {
      expect(foundationsCss).toContain(part.trim());
    }
  });

  it('reads only properties that exist, so a typo cannot resolve to nothing', () => {
    // The failure this catches is silent by construction: `var(--theme-surace)` is valid CSS, it
    // resolves to the empty string, and the surface is then transparent — which looks like a
    // design choice in a screenshot and like nothing at all in a diff. Fourteen children are about
    // to write custom properties into shadow roots, so it is worth an assertion.
    const own = new Set(
      [...foundationsCss.matchAll(/(--mjx-[a-z-]+)\s*:/g)].map((match) => String(match[1])),
    );
    const unknown = new Set<string>();
    for (const match of foundationsCss.matchAll(/var\((--[a-z0-9-]+)/g)) {
      const property = String(match[1]);
      if (legalCustomProperties.has(property) || own.has(property)) continue;
      unknown.add(property);
    }
    expect([...unknown], `the foundations sheet reads properties nothing declares`).toEqual([]);
  });

  it('writes no literal colour, radius, duration or easing', () => {
    // The literal-value lint covers the *source*; this covers the *composed output*, which is what
    // a browser actually parses. They are not the same check: a value assembled from two harmless
    // fragments would pass the lint and land here.
    expect(foundationsCss).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
    expect(foundationsCss).not.toMatch(/cubic-bezier/);
    expect(foundationsCss).not.toMatch(/\brgba?\(/);
    // Exactly two px values are allowed in the whole sheet, and naming them is the point: the
    // accessible-minimum floor, which is a standard's number rather than a design token, and the
    // 1px hairline, for which no token exists. Anything else appearing here is a design value that
    // escaped the token pipeline — which is the failure this assertion is shaped around, and the
    // reason it is an equality rather than a "does not contain".
    const pixels = [...foundationsCss.matchAll(/\b\d+(?:\.\d+)?px\b/g)].map((match) => match[0]);
    expect(new Set(pixels)).toEqual(new Set(['1px', `${String(accessibleHitTargetMinimum)}px`]));
  });
});
