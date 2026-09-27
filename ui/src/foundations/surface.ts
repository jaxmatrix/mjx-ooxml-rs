/**
 * `<mjx-surface>` — a rung of the elevation ladder, with a radius and a density.
 *
 * ```html
 * <mjx-surface level="floating" radius="panel" density="compact">…</mjx-surface>
 * ```
 *
 * It is the one shipped element in this child that carries no behaviour at all, and that is the
 * point: fourteen component children need *the same* card, panel, menu and selected-row treatment,
 * and a system where each one writes `background: var(--theme-surface); border: 1px solid …;
 * border-radius: var(--radius-card)` into its own shadow root has four slightly different cards
 * within a month.
 *
 * ## Light content, shadow chrome
 *
 * The content is slotted rather than adopted, so the *document's* stylesheet styles it. That is
 * what lets a caller put its own component inside a surface and have it keep its own styling — and
 * it is why `installFoundations` is called on the document as well as on this element's shadow
 * root by `.storybook/preview.ts`: the typography classes an author writes on slotted content are
 * document rules, not shadow rules.
 *
 * ## `density` is an attribute *and* a cascading property
 *
 * Setting `density` on this element writes `data-density` on the host, which the foundations
 * stylesheet's `[data-density]` rules pick up — so the custom properties cascade into the slotted
 * content, and a compact inspector inside a comfortable shell is one attribute. This is the
 * difference from the colour scheme, which `tokens.css` keys off `:root` and which therefore
 * genuinely cannot vary per subtree; the distinction is worth knowing before assuming both work
 * the same way.
 */

import { installFoundations } from './stylesheet.ts';
import { densityModeNames, type DensityMode } from './density.ts';
import {
  radiusSteps,
  surfaceLevelNames,
  surfaceLevels,
  type RadiusStep,
  type SurfaceLevel,
} from './surfaces.ts';

/** The rung a surface sits on when it does not say. */
export const defaultSurfaceLevel: SurfaceLevel = 'raised';

const styles = `
  :host {
    display: block;
    box-sizing: border-box;
    padding: var(--mjx-density-gutter);
    color: var(--theme-text-primary);
    font-family: var(--font-sans);
  }
  :host([hidden]) { display: none; }
  .surface { display: block; }
`;

/** Whether a value names a rung. */
export function isSurfaceLevel(value: unknown): value is SurfaceLevel {
  return (surfaceLevelNames as readonly string[]).includes(String(value));
}

/** Whether a value names a radius step. */
export function isRadiusStep(value: unknown): value is RadiusStep {
  return (radiusSteps as readonly string[]).includes(String(value));
}

/** Whether a value names a density mode. */
export function isDensityMode(value: unknown): value is DensityMode {
  return (densityModeNames as readonly string[]).includes(String(value));
}

export class MjxSurface extends HTMLElement {
  static readonly observedAttributes = ['level', 'radius', 'density'];

  #root: ShadowRoot | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.#render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.#render();
  }

  /** The rung. */
  get level(): SurfaceLevel {
    const declared = this.getAttribute('level');
    return isSurfaceLevel(declared) ? (declared as SurfaceLevel) : defaultSurfaceLevel;
  }

  set level(value: SurfaceLevel) {
    this.setAttribute('level', value);
  }

  /** The corner radius. Absent means the rung's own default. */
  get radius(): RadiusStep {
    const declared = this.getAttribute('radius');
    return isRadiusStep(declared) ? (declared as RadiusStep) : surfaceLevels[this.level].radius;
  }

  set radius(value: RadiusStep) {
    this.setAttribute('radius', value);
  }

  /** The density mode, or `undefined` to inherit whatever an ancestor set. */
  get density(): DensityMode | undefined {
    const declared = this.getAttribute('density');
    return isDensityMode(declared) ? (declared as DensityMode) : undefined;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installFoundations(root);
    // …and on the document too, because the level and radius classes land on the HOST, which is a
    // light-DOM element and is styled by the document's sheet rather than by this one. An element
    // that needed its consumer to remember an installation step would be an element that renders
    // as an unstyled box in the one place nobody tested.
    installFoundations(this.ownerDocument);
    const sheet = document.createElement('style');
    sheet.textContent = styles;
    const holder = document.createElement('div');
    holder.className = 'surface';
    holder.setAttribute('part', 'surface');
    holder.append(document.createElement('slot'));
    root.append(sheet, holder);
  }

  #render(): void {
    if (this.#root === undefined) return;

    // The level and radius classes go on the HOST, not on the inner div, so the shadow root's
    // `:host` box is the one that carries the background, the border and the shadow. A shadow on
    // an inner element would be clipped by nothing and drawn inside the host's padding, which is
    // the sort of half-right elevation a screenshot happily locks in.
    for (const level of surfaceLevelNames) this.classList.remove(`mjx-surface-${level}`);
    for (const step of radiusSteps) this.classList.remove(`mjx-radius-${step}`);
    this.classList.add(`mjx-surface-${this.level}`, `mjx-radius-${this.radius}`);

    const density = this.density;
    if (density === undefined) this.removeAttribute('data-density');
    else this.dataset['density'] = density;
  }
}

/** Register the element. Idempotent. */
export function defineSurface(): void {
  if (customElements.get('mjx-surface') === undefined) {
    customElements.define('mjx-surface', MjxSurface);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-surface': MjxSurface;
  }
}
