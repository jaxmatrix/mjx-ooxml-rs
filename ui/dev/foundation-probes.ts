/**
 * Throwaway probes for the foundations. **Not components; nothing may depend on them.**
 *
 * U01 established the pattern and the reason, and MJXOFF-181 needs it more than U01 did:
 *
 * > A gate is only known to work when it has been watched failing. Proving that on a *real*
 * > component means keeping a real component broken, which nobody does for long; proving it on a
 * > throwaway means the broken thing has no other job and cannot be quietly fixed.
 *
 * MJXOFF-181's own version of the trap is sharper, because three of the four things this child
 * ships are invisible in a static story:
 *
 * > **Focus is invisible in a static story.** A focus ring nobody focuses is untested — drive it
 * > with real keyboard interaction, and assert it is visible against **both** themes.
 *
 * | Probe | Proves |
 * |---|---|
 * | `mjx-focus-probe` | one focus ring, keyboard-only, visible on **every rung** of the elevation ladder |
 * | `mjx-density-probe` | comfortable and compact change spacing while the hit target stays above the floor |
 * | `mjx-motion-probe` | a document-attached role resolves to the ink easing and not the overshooting one |
 *
 * ## Light DOM, as U01's probes are
 *
 * The tests read computed styles and drive focus with real key presses; `document.activeElement`
 * reports a shadow *host* rather than the button inside it, and `innerText` does not cross a shadow
 * boundary. Keeping the probes plain is what keeps the assertions obviously right — which matters
 * more here than in a component, because these tests are the evidence for everything else.
 *
 * The probes use the real foundation classes, from the real stylesheet, installed on the real
 * document. What is throwaway is the *arrangement*, not the machinery under it.
 */

import { installFoundations } from '../src/foundations/stylesheet.ts';
import { surfaceLevelNames, type SurfaceLevel } from '../src/foundations/surfaces.ts';
import { densityModeNames, type DensityMode } from '../src/foundations/density.ts';
import {
  motionRoleClass,
  motionRoleNames,
  motionRoles,
  type MotionRole,
} from '../src/foundations/motion.ts';
import { typeRoleClass, typeRoleNames, type TypeRole } from '../src/foundations/typography.ts';

export {
  densityModeNames,
  motionRoleClass,
  motionRoleNames,
  surfaceLevelNames,
  typeRoleClass,
  typeRoleNames,
  type DensityMode,
  type MotionRole,
  type SurfaceLevel,
  type TypeRole,
};

// ── the focus probe ──────────────────────────────────────────────────────────

/**
 * One real `<button>` on every rung of the elevation ladder.
 *
 * Why buttons and not styled `<div>`s: `:focus-visible` is a *user-agent* judgement about whether
 * the focus was reached by keyboard, and the heuristic differs between a native control and a
 * `tabindex`. A probe that used the easy one would prove the ring on the case that was never in
 * doubt.
 *
 * The accessible name of each button says which rung it is, so the a11y sweep over this story is
 * a real sweep rather than a page of unnamed buttons.
 */
export class MjxFocusProbe extends HTMLElement {
  connectedCallback(): void {
    installFoundations(this.ownerDocument);
    if (this.childElementCount > 0) return;
    this.style.display = 'flex';
    this.style.flexWrap = 'wrap';
    this.style.gap = 'var(--mjx-density-step)';
    for (const level of surfaceLevelNames) {
      const surface = this.ownerDocument.createElement('div');
      surface.className = `mjx-surface-${level}`;
      surface.dataset['level'] = level;
      surface.style.padding = 'var(--mjx-density-gutter)';
      const button = this.ownerDocument.createElement('button');
      button.type = 'button';
      button.dataset['focusTarget'] = level;
      button.className = `${typeRoleClass('control')} mjx-hit-target`;
      button.textContent = level;
      button.style.background = 'transparent';
      button.style.color = 'inherit';
      button.style.border = '1px solid var(--theme-border)';
      button.style.borderRadius = 'var(--radius-control)';
      button.style.paddingInline = 'var(--mjx-density-gutter)';
      surface.append(button);
      this.append(surface);
    }
  }
}

// ── the density probe ────────────────────────────────────────────────────────

/**
 * The same three rows in both density modes, so an auditor sees the difference and a test measures
 * it.
 *
 * The gate MJXOFF-181 asks for is *"density modes change spacing without changing hit-target size
 * below the accessible minimum, asserted on computed values"* — two assertions with opposite
 * signs, which is what makes it non-vacuous: a compact mode that changed nothing passes the second
 * and fails the first.
 */
export class MjxDensityProbe extends HTMLElement {
  static readonly observedAttributes = ['density'];

  connectedCallback(): void {
    installFoundations(this.ownerDocument);
    this.#render();
  }

  attributeChangedCallback(): void {
    if (this.isConnected) this.#render();
  }

  /** Which mode, or `undefined` to inherit. */
  get density(): DensityMode | undefined {
    const declared = this.getAttribute('density');
    return (densityModeNames as readonly string[]).includes(String(declared))
      ? (declared as DensityMode)
      : undefined;
  }

  #render(): void {
    this.innerHTML = '';
    const density = this.density;
    if (density === undefined) this.removeAttribute('data-density');
    else this.dataset['density'] = density;
    this.style.display = 'flex';
    this.style.flexDirection = 'column';
    this.style.gap = 'var(--mjx-density-step)';
    this.className = 'mjx-surface-raised';
    this.style.padding = 'var(--mjx-density-gutter)';

    for (const label of ['Font', 'Size', 'Colour']) {
      const row = this.ownerDocument.createElement('div');
      row.dataset['row'] = label.toLowerCase();
      row.className = `${typeRoleClass('dense')} mjx-hit-target`;
      row.style.display = 'flex';
      row.style.alignItems = 'center';
      row.style.gap = 'var(--mjx-density-step)';
      row.style.paddingInline = 'var(--mjx-density-gutter)';
      row.style.background = 'var(--theme-background)';
      row.style.borderRadius = 'var(--radius-chip)';
      row.textContent = label;
      this.append(row);
    }
  }
}

// ── the motion probe ─────────────────────────────────────────────────────────

/**
 * One box per motion role, each carrying the role's class and therefore the role's easing.
 *
 * The assertion is on `transition-timing-function`, read back out of the browser and compared with
 * the generated `ease.*` token — so a role that names the right token in `motion.ts` and resolves
 * to the wrong curve in the cascade is caught, and the §4 rule (*nothing attached to a document
 * object may overshoot*) is checked where it is actually applied rather than where it is written.
 *
 * Each box also *runs*, on a looping animation whose timing function is the same role's easing, so
 * the auditor sees the difference between an overshooting curve and an ink one — which no amount
 * of reading a cubic-bezier conveys. The loop honours `prefers-reduced-motion`, because
 * `motionCss` states that once for everything that carries a motion class.
 */
const motionProbeStyleId = 'mjx-motion-probe-styles';

const motionProbeStyles = `
  @keyframes mjx-motion-probe-run {
    from { transform: translateX(0); }
    to   { transform: translateX(calc(100% * 4)); }
  }
  mjx-motion-probe [data-mover] {
    animation-name: mjx-motion-probe-run;
    animation-iteration-count: infinite;
    animation-direction: alternate;
  }
`;

export class MjxMotionProbe extends HTMLElement {
  connectedCallback(): void {
    installFoundations(this.ownerDocument);
    if (this.ownerDocument.getElementById(motionProbeStyleId) === null) {
      const sheet = this.ownerDocument.createElement('style');
      sheet.id = motionProbeStyleId;
      sheet.textContent = motionProbeStyles;
      this.ownerDocument.head.append(sheet);
    }
    if (this.childElementCount > 0) return;
    this.style.display = 'flex';
    this.style.flexDirection = 'column';
    this.style.gap = 'var(--mjx-density-step)';
    for (const role of motionRoleNames) {
      const track = this.ownerDocument.createElement('div');
      track.dataset['role'] = role;
      track.style.display = 'flex';
      track.style.alignItems = 'center';
      track.style.gap = 'var(--mjx-density-step)';

      const name = this.ownerDocument.createElement('span');
      name.className = typeRoleClass('dense');
      name.textContent = role;
      name.style.minInlineSize = '12ch';

      const rail = this.ownerDocument.createElement('div');
      rail.className = 'mjx-surface-sunken';
      rail.style.flex = '1';
      rail.style.padding = 'var(--mjx-density-step)';

      const mover = this.ownerDocument.createElement('div');
      mover.dataset['mover'] = role;
      mover.className = motionRoleClass(role);
      mover.style.inlineSize = 'var(--mjx-hit-target)';
      mover.style.blockSize = 'var(--mjx-hit-target)';
      mover.style.background = 'var(--theme-accent)';
      mover.style.borderRadius = 'var(--radius-chip)';
      mover.style.transitionProperty = 'transform';
      // The demonstration runs on `animation`, whose timing function is a different property from
      // the `transition-timing-function` the role's class sets — so both are written from the one
      // `motionRoles` entry rather than one of them being retyped here.
      mover.style.animationTimingFunction = motionRoles[role].easing;
      mover.style.animationDuration = `calc(var(--duration-transition) * 8)`;

      rail.append(mover);
      track.append(name, rail);
      this.append(track);
    }
  }
}

// ── registration ─────────────────────────────────────────────────────────────

const registry: readonly [string, CustomElementConstructor][] = [
  ['mjx-focus-probe', MjxFocusProbe],
  ['mjx-density-probe', MjxDensityProbe],
  ['mjx-motion-probe', MjxMotionProbe],
];

/** Register every foundation probe. Idempotent. */
export function defineFoundationProbes(): void {
  for (const [name, constructor] of registry) {
    if (customElements.get(name) === undefined) customElements.define(name, constructor);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-focus-probe': MjxFocusProbe;
    'mjx-density-probe': MjxDensityProbe;
    'mjx-motion-probe': MjxMotionProbe;
  }
}
