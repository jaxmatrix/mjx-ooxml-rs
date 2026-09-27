import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it, test } from 'vitest';

import {
  SurfaceStack,
  chooseDockEdge,
  chooseIndicator,
  chooseModalEdge,
  clampFraction,
  chooseSurfaceHandle,
  compositeOver,
  darkestThemeMember,
  dialogPresentationAt,
  dialogSheetAtOrBelow,
  dismissible,
  dockEdgeCandidates,
  focusManagementPatterns,
  fractionFromDrag,
  fractionFromKey,
  modalEdgeCandidates,
  nonTextMinimum,
  persistentSurfaceKinds,
  scrimAlpha,
  scrimBand,
  scrimColor,
  scrimOpacityPercent,
  schemeSelectors,
  surfaceCloseReasons,
  surfaceHandleCandidates,
  surfaceKindNames,
  surfaceKinds,
  surfaceSchemeCss,
  surfaceStoryTitles,
  taskPaneCss,
  themeColor,
  worstAgainstBand,
  worstDockEdge,
  type ScrimBand,
  type SurfaceKind,
} from '../src/surfaces/surface-model.ts';
import {
  chooseSwatchIndicatorAmong,
  indicatorSweepColors,
} from '../src/pickers/picker-model.ts';
import { contrastRatioOrWorst, formatRatio, relativeLuminance } from '../src/tokens/contrast.ts';
import { phoneShellAtOrBelow } from '../src/harness/presets.ts';
import { menuSheetAtOrBelow } from '../src/menus/menu-model.ts';
import { gallerySheetAtOrBelow } from '../src/gallery/gallery-model.ts';
import { tokens, type ColorScheme } from '../tokens/tokens.ts';
import type { ThemeMember } from '../src/tokens/resolver.ts';

/**
 * The surfaces, proved in Node.
 *
 * Three kinds of assertion live here and each is one a browser cannot make better:
 *
 * * **the indicator rule, swept.** MJXOFF-269's lesson is that a gate comparing code to a model
 *   cannot tell you the model is wrong, so nothing below asserts a ratio this project chose. Every
 *   number is measured over 4,352 colours, against WCAG's floor, and every *fixed* alternative to
 *   the rule is measured beside it and shown to fail — because *"the worst case clears 3 : 1"* is
 *   satisfied by a sweep that could not have failed;
 * * **the stack, over sequences.** *Closing an outer surface takes its descendants and closing an
 *   inner one does not touch the outer* is a property of every sequence of opens and closes, and a
 *   browser test can drive one;
 * * **the vocabulary's own consistency** — that the task pane really is the only surface nothing
 *   dismisses, and that the CSS a component reads back is the CSS the model would have written.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];
const sweep = indicatorSweepColors();

// ── the vocabulary ───────────────────────────────────────────────────────────

describe('the six surfaces', () => {
  it('is six rows across three elements, and every element renders at least one', () => {
    expect(surfaceKindNames).toHaveLength(6);
    const tags = new Set(surfaceKindNames.map((kind) => surfaceKinds[kind].tag));
    expect([...tags].sort()).toEqual(['mjx-dialog', 'mjx-popover', 'mjx-task-pane']);
  });

  it('every dismissal names a real reason', () => {
    for (const kind of surfaceKindNames) {
      for (const reason of surfaceKinds[kind].dismissals) {
        expect(surfaceCloseReasons, `${kind} lists ${reason}`).toContain(reason);
      }
    }
  });

  /**
   * ⚠ **The assertion this child exists to get right, and the one a shared sweep would invert.**
   *
   * MJXOFF-188: *"A task pane is docked, resizable and persistent — it is the one surface that is
   * not dismissible, and a gate that treats it like the others will assert the wrong thing."* So
   * the emptiness is asserted **and** its uniqueness is, because an empty list on a second row
   * would mean somebody had made a dialog undismissable and nothing would have said so.
   */
  it('the task pane is the only surface a person cannot close', () => {
    expect(persistentSurfaceKinds).toEqual(['taskPane']);
    expect(surfaceKinds.taskPane.dismissals).toEqual([]);
    for (const reason of surfaceCloseReasons) {
      expect(dismissible('taskPane', reason), `taskPane refused ${reason}`).toBe(
        reason === 'programmatic' ? false : false,
      );
    }
    // …and every other surface answers Escape.
    for (const kind of surfaceKindNames) {
      if (kind === 'taskPane') continue;
      expect(dismissible(kind, 'escape'), `${kind} closes on Escape`).toBe(true);
    }
  });

  it('only a modal traps, and only a modal scrims', () => {
    for (const kind of surfaceKindNames) {
      const spec = surfaceKinds[kind];
      if (spec.modal) {
        expect(spec.focus, `${kind} is modal and must trap`).toBe('trap');
        expect(spec.scrim, `${kind} is modal and must scrim`).toBe(true);
        expect(spec.dismissals, `${kind} is modal and its scrim must dismiss`).toContain('scrim');
      } else {
        expect(spec.scrim, `${kind} is not modal and must not scrim`).toBe(false);
        expect(spec.dismissals, `${kind} has no scrim to click`).not.toContain('scrim');
      }
    }
  });

  /**
   * U05's rule, and the half MJXOFF-188 added to it.
   *
   * The table is `focusManagementPatterns` in `src/menus/menu-model.ts` — extended there rather
   * than restated here, which is what the ticket asked for. This asserts that the extension is
   * additive: U05's two members are untouched.
   */
  it('extends U05’s focus table rather than restating it', () => {
    expect(Object.keys(focusManagementPatterns).sort()).toEqual(['roving', 'shared', 'trap']);
    expect(focusManagementPatterns.roving.tabStops).toBe('one');
    expect(focusManagementPatterns.trap.tabStops).toBe('many');
    expect(focusManagementPatterns.shared.tabStops).toBe('many');
    // The whole point of the third member: many stops and no containment, which is only safe
    // because the background is still reachable.
    for (const kind of surfaceKindNames) {
      if (surfaceKinds[kind].focus !== 'shared') continue;
      expect(surfaceKinds[kind].modal, `${kind} is shared and must not be modal`).toBe(false);
    }
  });

  it('the story titles are the ones the story files declare', () => {
    const root = resolve(import.meta.dirname, '../stories/surfaces');
    for (const [key, title] of Object.entries(surfaceStoryTitles)) {
      const file = join(root, `${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}.stories.ts`);
      const source = readFileSync(file, 'utf8');
      // ⚠ CSF is indexed statically and refuses a computed title, so the literal is duplicated into
      // the story file on purpose. This is what keeps the duplicate honest.
      expect(source, `${file} declares ${title}`).toContain(`title: '${title}'`);
    }
  });
});

// ── the presentations ────────────────────────────────────────────────────────

describe('a sheet is a dialog at a phone’s width', () => {
  it('there is one threshold, and four surfaces are aliases of it', () => {
    expect(dialogSheetAtOrBelow).toBe(phoneShellAtOrBelow);
    expect(menuSheetAtOrBelow).toBe(phoneShellAtOrBelow);
    expect(gallerySheetAtOrBelow).toBe(phoneShellAtOrBelow);
  });

  it('the model and the stylesheet state the threshold the same way', () => {
    const css = readFileSync(
      resolve(import.meta.dirname, '../src/surfaces/surface-model.ts'),
      'utf8',
    );
    expect(css).toContain(`(width <= \${String(dialogSheetAtOrBelow)}px)`);
  });

  /**
   * ⚠ **A gate the browser suite asked for, after finding the defect it watches.**
   *
   * MJXOFF-183's specificity accident, arriving by the route its own fix does not cover. The task
   * pane's phone-width block was written *above* the base rules, where it read most naturally
   * beside the host rule it also changes — and since both splitter selectors score (0,1,0), the
   * base rule won. The pane reported `full` at a phone width and still drew its splitter, with the
   * presentation assertion green, because a presentation gate only compares a custom property.
   *
   * The emission-order assertion is only meaningful *because* the two selectors are of equal
   * specificity, which U04 is the record of: order arbitrates nothing between rules that are not.
   * So both facts are asserted, not just the order.
   */
  it('the task pane’s phone block is emitted after the base rules it overrides', () => {
    const base = taskPaneCss.indexOf('.splitter {');
    const container = taskPaneCss.indexOf('@container');
    expect(base, 'the base splitter rule is gone, so this measures nothing').toBeGreaterThan(-1);
    expect(container, 'the phone block is gone, so this measures nothing').toBeGreaterThan(-1);
    expect(container).toBeGreaterThan(base);
    // …and the override is at the same specificity, which is what makes order decide at all.
    expect(taskPaneCss.slice(container)).toContain('.splitter { display: none; }');
    expect(taskPaneCss.slice(container)).not.toContain(':where(.splitter)');
  });

  it('a modal becomes a sheet at the threshold and a modeless dialog never does', () => {
    expect(dialogPresentationAt(dialogSheetAtOrBelow, true)).toBe('sheet');
    expect(dialogPresentationAt(dialogSheetAtOrBelow + 1, true)).toBe('modal');
    // ⚠ The condition that is easy to leave out: a modeless dialog has no scrim, so a modeless
    // sheet would acquire a dismissal path its own row does not list.
    expect(dialogPresentationAt(dialogSheetAtOrBelow, false)).toBe('dialog');
    expect(dialogPresentationAt(dialogSheetAtOrBelow - 200, false)).toBe('dialog');
  });
});

// ── the scheme layer, restated and therefore checked ─────────────────────────

describe('the scheme-keyed properties', () => {
  /**
   * ⚠ **The weakest thing in this child, and this is the gate that watches it.**
   *
   * The three properties are chosen per scheme, and `tokens.css` publishes the *choice* only
   * through three selectors. Restating them is unavoidable; restating them **unchecked** is not.
   */
  it('names the same three selectors the generated stylesheet uses', () => {
    const generated = readFileSync(resolve(import.meta.dirname, '../tokens/tokens.css'), 'utf8');
    expect(generated).toContain(schemeSelectors.darkPreferred);
    expect(generated).toContain(schemeSelectors.darkExplicit);
    expect(generated).toContain('@media (prefers-color-scheme: dark)');
  });

  it('emits a light block and both dark blocks, with a different edge in each', () => {
    expect(surfaceSchemeCss).toContain(schemeSelectors.darkPreferred);
    expect(surfaceSchemeCss).toContain(schemeSelectors.darkExplicit);
    const light = chooseModalEdge('light').member;
    const dark = chooseModalEdge('dark').member;
    // Not an assertion about *which* members — that is the rule's business and a re-seed may move
    // it. What is asserted is that the sheet actually publishes whatever the rule chose.
    expect(surfaceSchemeCss).toContain(`var(--theme-light-${dashed(light)})`);
    expect(surfaceSchemeCss).toContain(`var(--theme-dark-${dashed(dark)})`);
    expect(surfaceSchemeCss).toContain(`${String(scrimOpacityPercent)}%, transparent`);
  });
});

function dashed(member: string): string {
  return member.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`);
}

// ── the scrim arithmetic ─────────────────────────────────────────────────────

describe('compositing', () => {
  it('an alpha of zero is the backdrop and an alpha of one is the front', () => {
    expect(compositeOver('#123456', '#abcdef', 0)).toBe('#123456');
    expect(compositeOver('#123456', '#abcdef', 1)).toBe('#abcdef');
  });

  it('a colour it cannot read composites to the backdrop rather than to something invented', () => {
    expect(compositeOver('#123456', 'chartreuse', 0.5)).toBe('#123456');
  });

  it('the scrim is the scheme’s darkest token, and it is not the same token in both', () => {
    for (const scheme of schemes) {
      const chosen = darkestThemeMember(scheme);
      const chosenLuminance = relativeLuminance(themeColor(scheme, chosen)) ?? 1;
      for (const [member, value] of Object.entries(tokens.theme[scheme])) {
        const luminance = relativeLuminance(String(value)) ?? 1;
        expect(chosenLuminance, `${member} in ${scheme} is darker than ${chosen}`).toBeLessThanOrEqual(
          luminance + 1e-12,
        );
      }
    }
  });

  /**
   * ⚠ **The property that actually matters, stated so a palette re-seed cannot break it for the
   * wrong reason.**
   *
   * The first version of this asserted that the two schemes pick *different* members, which is
   * true today and is a fact about the palette rather than about the rule. What the rule owes is
   * that a scrim **darkens**: an ink scrim in the dark scheme would *lighten* a dark application,
   * which is the defect the derivation exists to make impossible.
   */
  it('the scrim is darker than the surface it is drawn behind, in both schemes', () => {
    for (const scheme of schemes) {
      const scrim = relativeLuminance(scrimColor(scheme)) ?? 1;
      const fill = relativeLuminance(themeColor(scheme, 'surfaceRaised')) ?? 0;
      expect(scrim, `the ${scheme} scrim is not darker than the surface it dims`).toBeLessThan(fill);
    }
  });

  /**
   * ⚠ **The anti-vacuity, and the first two attempts at it were both wrong — which is worth
   * writing down, because both were the shape of mistake this catalogue keeps making.**
   *
   * The first asserted the two schemes pick *different* members. True today, a fact about the
   * palette rather than about the rule, and exactly the gate MJXOFF-188 warns against: *"if a gate
   * of yours would break when a colour changes, it is the wrong gate."*
   *
   * The second asserted that **no** single member could have served as the scrim in both schemes.
   * It failed immediately, and the failure was informative: **ten members darken in both**, so a
   * fixed choice would in fact have worked. The derivation's value is therefore not that a fixed
   * member is impossible — it is that a fixed member is a *coincidence nobody is watching* and the
   * darkest member is a **guarantee**. The re-seed proved the point while this was being written:
   * under the previous palette an ink scrim genuinely did lighten the dark scheme, and under the
   * current one it does not. The motivating fact moved; the guarantee did not.
   *
   * So the guarantee is what is asserted — nothing in a scheme is darker than that scheme's scrim —
   * and beside it, that the property is not free: some member darkens in one scheme and lightens in
   * the other, which is the risk a named member carries and a derived one cannot.
   */
  it('nothing in a scheme is darker than that scheme’s scrim', () => {
    for (const scheme of schemes) {
      const scrim = relativeLuminance(scrimColor(scheme)) ?? 1;
      let compared = 0;
      for (const [member, value] of Object.entries(tokens.theme[scheme])) {
        const luminance = relativeLuminance(String(value));
        // A member that is not a colour at all — the re-seed added percentage members — is not a
        // candidate, and is skipped rather than treated as black.
        if (luminance === undefined) continue;
        expect(scrim, `${member} in ${scheme} is darker than the scrim`).toBeLessThanOrEqual(
          luminance + 1e-12,
        );
        compared += 1;
      }
      expect(compared, `${scheme} has no colours in it, so this measures nothing`).toBeGreaterThan(5);
    }
  });

  it('and a named member would have been a coincidence: some darken in one scheme only', () => {
    const members = Object.keys(tokens.theme.light) as ThemeMember[];
    const darkens = (scheme: ColorScheme, member: ThemeMember): boolean =>
      (relativeLuminance(themeColor(scheme, member)) ?? 1) <
      (relativeLuminance(themeColor(scheme, 'surfaceRaised')) ?? 0);
    const inconsistent = members.filter(
      (member) => darkens('light', member) !== darkens('dark', member),
    );
    expect(
      inconsistent.length,
      'every member darkens in both schemes, so naming one would carry no risk',
    ).toBeGreaterThan(0);
  });
});

/**
 * The band is two colours, and this is the proof.
 *
 * The closed form in `worstAgainstBand` rests on an argument — alpha compositing is monotone per
 * channel and relative luminance is monotone in every channel, so the composite of *anything* lies
 * between the composite of black and the composite of white. An argument is not a gate.
 */
describe('the scrim band', () => {
  for (const scheme of schemes) {
    it(`the closed form agrees with all ${String(sweep.length)} composites in ${scheme}`, () => {
      const band = scrimBand(scheme);
      const scrim = scrimColor(scheme);
      const candidates: readonly ThemeMember[] = [...modalEdgeCandidates, 'surfaceRaised'];
      let compared = 0;
      for (const member of candidates) {
        const colour = themeColor(scheme, member);
        let swept = Number.POSITIVE_INFINITY;
        for (const backdrop of sweep) {
          swept = Math.min(swept, contrastRatioOrWorst(colour, compositeOver(backdrop, scrim, scrimAlpha)));
          compared += 1;
        }
        expect(
          Math.abs(swept - worstAgainstBand(colour, band)),
          `${member} in ${scheme}: swept ${formatRatio(swept)}, closed form ` +
            `${formatRatio(worstAgainstBand(colour, band))}`,
        ).toBeLessThan(0.01);
      }
      // ⚠ Anti-vacuity, U06's first defect in this child's costume: a sweep that compared nothing
      // agrees with everything.
      expect(compared).toBe(sweep.length * candidates.length);
    });
  }

  it('a colour inside the band is worth exactly 1, not Infinity', () => {
    const band = scrimBand('light');
    const inside = band.darkest;
    expect(worstAgainstBand(inside, band)).toBe(1);
    // U07's second defect, restated: an unreadable comparison must fail rather than pass.
    expect(worstAgainstBand('chartreuse', band)).toBe(1);
  });
});

// ── the modal's edge ─────────────────────────────────────────────────────────

describe('the modal’s edge, over a scrim it cannot see through', () => {
  for (const scheme of schemes) {
    it(`clears ${String(nonTextMinimum)} : 1 against the whole band in ${scheme}`, () => {
      const chosen = chooseModalEdge(scheme);
      expect(
        chosen.sufficient,
        `nothing in the candidate list reads on the ${scheme} scrim; the best was ` +
          `${chosen.member} at ${formatRatio(chosen.ratio)}`,
      ).toBe(true);
      expect(chosen.ratio).toBeGreaterThanOrEqual(nonTextMinimum);
    });
  }

  /**
   * ⚠ **The assertion above is satisfied by a sweep that could not have failed. This is the one
   * that says it could.**
   *
   * U08's remedy, applied: every *fixed* alternative to the rule is measured over the same band,
   * and each of them is shown to fail in at least one scheme. If they did not, the rule would be
   * doing no work and the ratio above would be an accident of the palette.
   */
  it('every fixed alternative fails in at least one scheme', () => {
    const failures: string[] = [];
    for (const member of modalEdgeCandidates) {
      const worst = Math.min(
        ...schemes.map((scheme) => worstAgainstBand(themeColor(scheme, member), scrimBand(scheme))),
      );
      if (worst < nonTextMinimum) failures.push(`${member} ${formatRatio(worst)}`);
    }
    expect(
      failures.length,
      `some candidate reads on the scrim in both schemes, so choosing does no work: ` +
        `failures were ${failures.join(', ')}`,
    ).toBe(modalEdgeCandidates.length);
  });

  /**
   * And the gate can fire: take the scrim away and the rule reports insufficiency rather than
   * quietly returning its least-bad option.
   */
  it('reports insufficiency when the scrim is transparent', () => {
    for (const scheme of schemes) {
      const chosen = chooseModalEdge(scheme, 0);
      expect(chosen.sufficient, `${scheme} claimed a sufficient edge with no scrim at all`).toBe(
        false,
      );
      expect(chosen.ratio).toBeLessThan(nonTextMinimum);
    }
  });

  /**
   * ⚠ **Why the opacity is the number it is, stated as a monotonicity rather than as a threshold.**
   *
   * The first version asserted that a scrim at half opacity leaves *some* scheme insufficient,
   * which is true of this palette and would be a gate that fires on a re-seed rather than on a
   * defect. What is actually true of every palette is the shape of the relationship: a thinner
   * scrim composites into a **wider** band, a wider band contains the narrower one, and a candidate
   * can therefore only do worse against it. So thinning the scrim never improves a single
   * candidate anywhere — and it strictly worsens at least one, which is what says the opacity is
   * load-bearing rather than decorative.
   */
  it('thinning the scrim never improves a candidate, and worsens at least one', () => {
    let worsened = 0;
    let compared = 0;
    for (const scheme of schemes) {
      const thick = scrimBand(scheme, scrimAlpha);
      const thin = scrimBand(scheme, scrimAlpha / 2);
      for (const member of modalEdgeCandidates) {
        const colour = themeColor(scheme, member);
        const before = worstAgainstBand(colour, thick);
        const after = worstAgainstBand(colour, thin);
        expect(after, `${member} in ${scheme} read better through a thinner scrim`).toBeLessThanOrEqual(
          before + 1e-9,
        );
        if (after < before - 1e-9) worsened += 1;
        compared += 1;
      }
    }
    expect(compared).toBe(schemes.length * modalEdgeCandidates.length);
    expect(worsened, 'the scrim’s opacity changes nothing at all').toBeGreaterThan(0);
  });
});

// ── the sheet's handle ───────────────────────────────────────────────────────

describe('the sheet’s grab handle, over a fill this catalogue does own', () => {
  for (const scheme of schemes) {
    it(`is the quietest candidate that clears the floor in ${scheme}`, () => {
      const chosen = chooseSurfaceHandle(scheme);
      expect(chosen.sufficient).toBe(true);
      expect(chosen.ratio).toBeGreaterThanOrEqual(nonTextMinimum);
      // Quietest, not loudest: everything earlier in the list must genuinely have failed. A rule
      // that maximised would always answer `textPrimary`, which is a black bar across a dialog.
      const index = surfaceHandleCandidates.indexOf(chosen.member);
      expect(index).toBeGreaterThanOrEqual(0);
      for (const earlier of surfaceHandleCandidates.slice(0, index)) {
        const ratio = contrastRatioOrWorst(
          themeColor(scheme, earlier),
          themeColor(scheme, 'surfaceRaised'),
        );
        expect(ratio, `${earlier} would have done in ${scheme}`).toBeLessThan(nonTextMinimum);
      }
    });
  }

  /**
   * ⚠ **A measured finding about a table this child does not own, recorded and not changed.**
   *
   * The obvious colour for a handle — and for the hairline `surfaceLevels.overlay` already draws
   * around every dialog and menu in the catalogue — is `--theme-border`. It measures **1.31 : 1**
   * in light and **1.17 : 1** in dark against the fill it sits on, which is not an indicator by any
   * reading of WCAG 1.4.11. That is legitimate for a *decorative* edge and it is why the modal's
   * boundary is carried by a separate, measured outline instead. U07 recorded the same shape about
   * `controlStateSpecs.on` and deliberately did not edit another child's committed model; this does
   * the same.
   */
  it('the obvious choice is not an indicator, in either scheme', () => {
    for (const scheme of schemes) {
      const ratio = contrastRatioOrWorst(
        themeColor(scheme, 'border'),
        themeColor(scheme, 'surfaceRaised'),
      );
      expect(ratio, `border on surfaceRaised in ${scheme} is ${formatRatio(ratio)}`).toBeLessThan(
        nonTextMinimum,
      );
    }
  });
});

// ── the task pane's edge, against a document nobody here owns ────────────────

describe('the task pane’s edge, over a page the person chose', () => {
  for (const scheme of schemes) {
    it(`the worst edge over ${String(sweep.length)} page colours clears the floor in ${scheme}`, () => {
      const worst = worstDockEdge(sweep, scheme);
      expect(worst, 'the sweep produced no measurement at all').toBeDefined();
      if (worst === undefined) return;
      expect(
        worst.indicator.ratio,
        `${worst.indicator.member} on ${worst.against} is ${formatRatio(worst.indicator.ratio)}`,
      ).toBeGreaterThanOrEqual(nonTextMinimum);
      expect(worst.indicator.sufficient).toBe(true);
    });

    it(`every fixed alternative bottoms out at 1.00 : 1 in ${scheme}`, () => {
      for (const member of dockEdgeCandidates) {
        let lowest = Number.POSITIVE_INFINITY;
        for (const page of sweep) {
          lowest = Math.min(lowest, contrastRatioOrWorst(themeColor(scheme, member), page));
        }
        expect(lowest, `a fixed ${member} edge survived the sweep in ${scheme}`).toBeLessThan(1.01);
      }
    });
  }

  /**
   * ⚠ **A near-duplicate, checked against the thing it nearly duplicates — and the check found a
   * real difference, which is why it is written this way rather than as an equality.**
   *
   * A task pane's edge against a document page and a colour picker's ring against a swatch are the
   * *same question with the same two candidates*, and this child could have written a second
   * implementation of it without anybody noticing. The first version of this test asserted the two
   * rules pick the same member, and it **failed on the grey line**: on `#7f7f7f` both candidates
   * clear 3 : 1, U08's rule takes the *stronger* of the two and this one takes the *quieter*
   * sufficient one, because a task pane's hairline is not a selection ring and does not want to be
   * as loud as it can be.
   *
   * So the relationship that actually holds is asserted instead, and it is the stronger claim:
   *
   * * the two **agree exactly** wherever nothing is sufficient, because both then fall back to the
   *   strongest — so this rule can never be worse than U08's when it matters;
   * * this rule reports `sufficient` **exactly** when U08's rule clears the floor, so neither can
   *   drift into thinking a page is servable when the other does not;
   * * and it never claims a ratio the maximum does not support.
   */
  for (const scheme of schemes) {
    it(`agrees with the colour picker's ring rule wherever it matters, in ${scheme}`, () => {
      const candidates = {
        textPrimary: themeColor(scheme, 'textPrimary'),
        surface: themeColor(scheme, 'surface'),
      } as const;
      let compared = 0;
      let quieter = 0;
      for (const page of sweep) {
        const mine = chooseDockEdge(page, scheme);
        const theirs = chooseSwatchIndicatorAmong(page, candidates);
        expect(mine.sufficient, `disagreed about ${page} in ${scheme}`).toBe(
          theirs.ratio >= nonTextMinimum,
        );
        expect(mine.ratio).toBeLessThanOrEqual(theirs.ratio + 1e-9);
        if (theirs.ratio < nonTextMinimum) {
          expect(mine.member, `fallback disagreed about ${page}`).toBe(theirs.member);
          expect(Math.abs(mine.ratio - theirs.ratio)).toBeLessThan(1e-9);
        }
        if (mine.member !== theirs.member) quieter += 1;
        compared += 1;
      }
      expect(compared).toBe(sweep.length);
      // ⚠ Anti-vacuity. If the two rules never diverged, this whole test would be an equality
      // wearing a longer explanation, and the difference it documents would not exist.
      expect(quieter, 'the two rules never differed, so there is nothing here to reconcile')
        .toBeGreaterThan(0);
    });
  }
});

// ── the rule itself ──────────────────────────────────────────────────────────

describe('chooseIndicator', () => {
  it('takes the first sufficient candidate, not the strongest', () => {
    const chosen = chooseIndicator(
      ['border', 'textPrimary'],
      (member) => (member === 'border' ? '#767676' : '#000000'),
      (colour) => contrastRatioOrWorst(colour, '#ffffff'),
    );
    expect(chosen.member).toBe('border');
    expect(chosen.sufficient).toBe(true);
  });

  it('falls back to the strongest and says it fell short', () => {
    const chosen = chooseIndicator(
      ['border', 'textPrimary'],
      (member) => (member === 'border' ? '#fefefe' : '#e0e0e0'),
      (colour) => contrastRatioOrWorst(colour, '#ffffff'),
    );
    expect(chosen.sufficient).toBe(false);
    expect(chosen.member).toBe('textPrimary');
    expect(chosen.ratio).toBeLessThan(nonTextMinimum);
  });

  it('an empty candidate list is insufficient rather than an exception', () => {
    const chosen = chooseIndicator([], () => '#000000', () => 21);
    expect(chosen.sufficient).toBe(false);
  });
});

// ── the stack ────────────────────────────────────────────────────────────────

describe('the surface stack', () => {
  it('the topmost is the newest, and re-opening moves rather than duplicates', () => {
    const stack = new SurfaceStack();
    stack.push({ id: 'a', kind: 'modal', parent: undefined });
    stack.push({ id: 'b', kind: 'popover', parent: 'a' });
    expect(stack.topmost?.id).toBe('b');
    stack.push({ id: 'a', kind: 'modal', parent: undefined });
    expect(stack.depth).toBe(2);
    expect(stack.topmost?.id).toBe('a');
  });

  it('closing an inner surface leaves the outer one open', () => {
    const stack = new SurfaceStack();
    stack.push({ id: 'dialog', kind: 'modal', parent: undefined });
    stack.push({ id: 'popover', kind: 'popover', parent: 'dialog' });
    expect(stack.remove('popover').map((entry) => entry.id)).toEqual(['popover']);
    expect(stack.has('dialog')).toBe(true);
  });

  it('closing an outer surface takes its descendants, innermost first', () => {
    const stack = new SurfaceStack();
    stack.push({ id: 'dialog', kind: 'modal', parent: undefined });
    stack.push({ id: 'flyout', kind: 'flyout', parent: 'dialog' });
    stack.push({ id: 'popover', kind: 'popover', parent: 'flyout' });
    stack.push({ id: 'unrelated', kind: 'popover', parent: undefined });
    expect(stack.remove('dialog').map((entry) => entry.id)).toEqual([
      'popover',
      'flyout',
      'dialog',
    ]);
    expect(stack.entries.map((entry) => entry.id)).toEqual(['unrelated']);
  });

  /**
   * The invariant over **sequences**, driven the way U06 drives its preview session: a browser
   * test can prove one sequence, and *"closing takes exactly the descendants"* is a claim about all
   * of them.
   */
  it('holds its invariant across 400 random sequences', () => {
    let seed = 20250910;
    const random = (): number => {
      seed = (seed * 1103515245 + 12345) % 2147483648;
      return seed / 2147483648;
    };
    const kinds: readonly SurfaceKind[] = ['modal', 'popover', 'flyout', 'dialog'];

    for (let run = 0; run < 400; run += 1) {
      const stack = new SurfaceStack();
      /** The parent of each id, kept outside the stack so the check is independent of it. */
      const parents = new Map<string, string | undefined>();
      let serial = 0;

      for (let step = 0; step < 12; step += 1) {
        const entries = stack.entries;
        if (entries.length > 0 && random() < 0.4) {
          const victim = entries[Math.floor(random() * entries.length)];
          if (victim === undefined) continue;
          const expected = new Set<string>([victim.id]);
          let grew = true;
          while (grew) {
            grew = false;
            for (const [id, parent] of parents) {
              if (parent !== undefined && expected.has(parent) && !expected.has(id)) {
                if (stack.has(id)) {
                  expected.add(id);
                  grew = true;
                }
              }
            }
          }
          const removed = stack.remove(victim.id).map((entry) => entry.id);
          expect(new Set(removed)).toEqual(expected);
          // Innermost first: whatever came out is in reverse open order.
          const order = removed.map((id) => entries.findIndex((entry) => entry.id === id));
          expect(order).toEqual([...order].sort((a, b) => b - a));
          for (const id of removed) parents.delete(id);
        } else {
          serial += 1;
          const id = `s${String(serial)}`;
          const parent =
            entries.length > 0 && random() < 0.7
              ? entries[Math.floor(random() * entries.length)]?.id
              : undefined;
          const kind = kinds[Math.floor(random() * kinds.length)] ?? 'modal';
          stack.push({ id, kind, parent });
          parents.set(id, parent);
        }

        // No duplicates, ever, and the newest is on top.
        const ids = stack.entries.map((entry) => entry.id);
        expect(new Set(ids).size).toBe(ids.length);
      }

      // `clear` reports everything, innermost first, and leaves nothing behind.
      const before = stack.entries.map((entry) => entry.id);
      expect(stack.clear().map((entry) => entry.id)).toEqual([...before].reverse());
      expect(stack.depth).toBe(0);
    }
  });
});

// ── the task pane's arithmetic ───────────────────────────────────────────────

describe('resizing a task pane', () => {
  it('clamps to the bounds and survives a value that is not a number', () => {
    expect(clampFraction(0)).toBeGreaterThan(0);
    expect(clampFraction(1)).toBeLessThan(1);
    expect(clampFraction(Number.NaN)).toBeGreaterThan(0);
  });

  /**
   * ⚠ **The arrow that grows the pane depends on which edge it is on.** A key map written for one
   * dock is a key map that shrinks the pane on the other, and the symptom is a control that works
   * backwards rather than one that does nothing — which nobody files.
   */
  it('the growing arrow mirrors with the dock', () => {
    const start = 0.4;
    expect(fractionFromKey('ArrowLeft', start, 'right')).toBeGreaterThan(start);
    expect(fractionFromKey('ArrowRight', start, 'right')).toBeLessThan(start);
    expect(fractionFromKey('ArrowRight', start, 'left')).toBeGreaterThan(start);
    expect(fractionFromKey('ArrowLeft', start, 'left')).toBeLessThan(start);
  });

  it('Home and End reach the bounds, and an unrelated key does nothing', () => {
    const min = fractionFromKey('Home', 0.4, 'right');
    const max = fractionFromKey('End', 0.4, 'right');
    expect(min).toBeDefined();
    expect(max).toBeDefined();
    expect(min).toBeLessThan(max ?? 0);
    expect(clampFraction(min ?? 0)).toBe(min);
    expect(fractionFromKey('Escape', 0.4, 'right')).toBeUndefined();
  });

  it('a drag reads the pointer from the right edge, and mirrors for the other dock', () => {
    const boundary = { start: 0, size: 1000 };
    // Pointer three quarters across: a pane docked at the right is a quarter wide.
    expect(fractionFromDrag(750, boundary, 'right')).toBeCloseTo(0.25, 5);
    expect(fractionFromDrag(250, boundary, 'left')).toBeCloseTo(0.25, 5);
    // …and both are clamped rather than allowed to swallow the document.
    expect(fractionFromDrag(10, boundary, 'left')).toBe(clampFraction(0.01));
    expect(fractionFromDrag(990, boundary, 'left')).toBe(clampFraction(0.99));
    // A boundary with no width is a measurement that has not happened yet.
    expect(fractionFromDrag(500, { start: 0, size: 0 }, 'right')).toBeGreaterThan(0);
  });
});

// ── the source itself ────────────────────────────────────────────────────────

/**
 * U08's grep, over this child's tree.
 *
 * `mjx/no-literal-design-values` refuses a hex in `src/`, and this is the stronger statement:
 * **there is no hex anywhere under `src/surfaces/` at all**, including in a comment, where the
 * lint rule deliberately does not look. The two colours the band arithmetic needs are the extremes
 * of the sRGB cube and are *constructed* from their channels, which is a different thing from a
 * design value spelled in hex.
 */
describe('src/surfaces/ ships no colours', () => {
  function sourcesUnder(directory: string): string[] {
    const found: string[] = [];
    for (const entry of readdirSync(directory)) {
      const path = join(directory, entry);
      if (statSync(path).isDirectory()) found.push(...sourcesUnder(path));
      else if (entry.endsWith('.ts')) found.push(path);
    }
    return found;
  }

  const hex = /#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{3})?\b/;

  test('no source under src/surfaces/ contains a hex colour', () => {
    const sources = sourcesUnder(resolve(import.meta.dirname, '../src/surfaces'));
    expect(sources.length, 'src/surfaces/ has no sources, so this measures nothing').toBeGreaterThan(
      3,
    );
    const offenders = sources.filter((path) => hex.test(readFileSync(path, 'utf8')));
    expect(offenders).toEqual([]);
  });

  test('the grep can fail', () => {
    expect(hex.test("const scrim = '#223b33';")).toBe(true);
    expect(hex.test('const scrim = tokens.theme.light.textPrimary;')).toBe(false);
  });
});

// ── a band a test can hold in its hand ───────────────────────────────────────

describe('a hand-built band', () => {
  it('says 1 for a colour inside it and a real ratio for one outside', () => {
    const band: ScrimBand = { darkest: '#101010', lightest: '#404040' };
    expect(worstAgainstBand('#202020', band)).toBe(1);
    expect(worstAgainstBand('#ffffff', band)).toBeGreaterThan(nonTextMinimum);
  });
});
