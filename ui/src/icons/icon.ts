/**
 * `<mjx-icon>` — one drawing from the subset, tinted from a token.
 *
 * ```html
 * <mjx-icon name="text-bold" size="20" variant="filled" label="Bold"></mjx-icon>
 * <mjx-icon name="chevron-right" size="16"></mjx-icon>   <!-- decorative -->
 * ```
 *
 * ## Inlined SVG, not a font and not a sprite
 *
 * MJXOFF-181's constraint: *"Icons are inlined SVG symbols, not a font — an icon font breaks at
 * arbitrary zoom and cannot be tinted per-path."* A `<use href="#…">` sprite would have been the
 * other option and was rejected for the reason a shadow root makes obvious: a sprite lives in one
 * document and `<use>` does not reach across a shadow boundary, so every component would need its
 * own copy of the sprite or a document-level contract about where the sprite lives. An inlined
 * path has no such dependency, and the subset is 14 kB of path data — small enough that inlining
 * costs nothing worth a mechanism.
 *
 * The paths are built with `createElementNS`, never with `innerHTML`. `scripts/subset-icons.mjs`
 * refuses any vendor file that is not a plain sequence of `<path d="…"/>` elements, so what reaches
 * this file is a list of path-data strings and the injection question does not arise.
 *
 * ## An icon not in the subset cannot be referenced
 *
 * This is the property the ticket asks for, and it holds at three different moments:
 *
 * * **Statically**, because `iconGlyphs` is a generated table and a story that builds its
 *   attributes from `iconRequests` cannot name a row that is not there.
 * * **At the bundler**, because ESLint forbids importing `@fluentui/svg-icons` outside
 *   `scripts/`, so no other icon has a route into the build at all.
 * * **At runtime**, here: an unknown name renders **nothing**, sets `data-icon-state="unknown"`,
 *   and — because a blank space in a toolbar is a bug nobody notices — reports the miss on the
 *   console. It does not fall back to a default icon; a wrong icon is worse than a missing one,
 *   because a person acts on it.
 *
 * ## Tinting and sizing
 *
 * The paths are filled with `currentColor`, so the icon takes the colour of whatever text it sits
 * beside and a component tints it by setting `color` — which is what makes an icon inside a
 * pressed button turn the pressed colour with no icon-specific rule anywhere. `::part(svg)` is
 * exposed for the cases where that is not enough.
 *
 * The box is `size` CSS pixels, from the same attribute that chose the drawing: Fluent draws each
 * size separately, with its own stroke weights and its own optical corrections, so rendering the
 * 20px drawing at 24px would throw away the reason for using Fluent at all. A `size` the subset
 * does not carry is an unknown icon, exactly like an unknown name.
 */

import { installFoundations } from '../foundations/stylesheet.ts';
import { iconGlyphs, type IconGlyph } from './generated.ts';
import {
  iconId,
  iconSizes,
  iconVariants,
  type IconSize,
  type IconVariant,
} from './manifest.ts';

const svgNamespace = 'http://www.w3.org/2000/svg';

/** The default size: Fluent's own chrome size, and the one most of the subset carries. */
export const defaultIconSize: IconSize = 20;

/** The default variant. `filled` is a state, so `regular` is what a resting command draws. */
export const defaultIconVariant: IconVariant = 'regular';

const styles = `
  :host {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
    flex: none;
    line-height: 0;
    vertical-align: middle;
  }
  :host([hidden]) { display: none; }
  svg { display: block; fill: currentColor; }
`;

/** Whether a value is one of the five sizes the subset is drawn at. */
export function isIconSize(value: unknown): value is IconSize {
  return (iconSizes as readonly number[]).includes(Number(value));
}

/** Whether a value is one of the two variants. */
export function isIconVariant(value: unknown): value is IconVariant {
  return (iconVariants as readonly string[]).includes(String(value));
}

/** The glyph for a name/size/variant, or `undefined` when the subset does not carry it. */
export function lookupGlyph(
  name: string,
  size: IconSize,
  variant: IconVariant,
): IconGlyph | undefined {
  return iconGlyphs[iconId(name, size, variant)];
}

/** How a component's state is reported to a test and to the DOM. */
export type IconState = 'resolved' | 'unknown';

export class MjxIcon extends HTMLElement {
  static readonly observedAttributes = ['name', 'size', 'variant', 'label'];

  #root: ShadowRoot | undefined;
  #svg: SVGSVGElement | undefined;
  /** Names already reported, so a list of two hundred rows does not print two hundred lines. */
  static readonly #reported = new Set<string>();

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.#render();
  }

  attributeChangedCallback(): void {
    // No `isConnected` guard: lit writes every attribute before it inserts the element, so a
    // callback that ignored a pre-connection change would drop the icon's name. `#render` is a
    // no-op until `#build` has run, and `connectedCallback` calls it again afterwards.
    if (this.#root !== undefined) this.#render();
  }

  /** The Fluent name, hyphenated. */
  get name(): string {
    return this.getAttribute('name') ?? '';
  }

  set name(value: string) {
    this.setAttribute('name', value);
  }

  /** The size, in CSS pixels, and the drawing that is used. */
  get size(): IconSize {
    const declared = this.getAttribute('size');
    return isIconSize(declared) ? (Number(declared) as IconSize) : defaultIconSize;
  }

  set size(value: IconSize) {
    this.setAttribute('size', String(value));
  }

  /** `regular` (resting) or `filled` (selected or active). */
  get variant(): IconVariant {
    const declared = this.getAttribute('variant');
    return isIconVariant(declared) ? (declared as IconVariant) : defaultIconVariant;
  }

  set variant(value: IconVariant) {
    this.setAttribute('variant', value);
  }

  /**
   * The accessible name.
   *
   * Absent means **decorative**, which is the right default: an icon in a labelled button is a
   * duplicate announcement, and the great majority of icons in a chrome sit inside something that
   * already has a name. An icon that carries meaning on its own — a status mark in a row, a lone
   * icon button — sets `label`, and gets `role="img"` and an `aria-label` for it.
   */
  get label(): string | undefined {
    return this.getAttribute('label') ?? undefined;
  }

  /** Whether the subset carries the drawing this element is asking for. */
  get state(): IconState {
    return lookupGlyph(this.name, this.size, this.variant) === undefined ? 'unknown' : 'resolved';
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installFoundations(root);
    const sheet = document.createElement('style');
    sheet.textContent = styles;
    root.append(sheet);
  }

  #render(): void {
    const root = this.#root;
    if (root === undefined) return;

    const name = this.name;
    const size = this.size;
    const variant = this.variant;
    const glyph = lookupGlyph(name, size, variant);

    this.#svg?.remove();
    this.#svg = undefined;

    if (glyph === undefined) {
      this.dataset['iconState'] = 'unknown';
      this.style.inlineSize = '';
      this.style.blockSize = '';
      this.removeAttribute('role');
      this.removeAttribute('aria-label');
      this.setAttribute('aria-hidden', 'true');
      const key = iconId(name, size, variant);
      if (!MjxIcon.#reported.has(key)) {
        MjxIcon.#reported.add(key);
        // Not an exception: a chrome that throws because one toolbar button asked for an icon
        // nobody added to the manifest is a chrome that does not open. The gate that turns this
        // into a build failure is tests/browser/icons.spec.ts, which is where it belongs.
        console.error(
          `<mjx-icon> was asked for '${key}', which the subset does not carry. Add a row to ` +
            'src/icons/manifest.ts and run `npm run icons:subset`. Nothing is rendered: a wrong ' +
            'icon is worse than a missing one, because a person acts on it.',
        );
      }
      return;
    }

    this.dataset['iconState'] = 'resolved';
    this.style.inlineSize = `${String(size)}px`;
    this.style.blockSize = `${String(size)}px`;

    const svg = document.createElementNS(svgNamespace, 'svg');
    svg.setAttribute('viewBox', glyph.viewBox);
    svg.setAttribute('width', String(size));
    svg.setAttribute('height', String(size));
    svg.setAttribute('part', 'svg');
    svg.setAttribute('focusable', 'false');
    for (const data of glyph.paths) {
      const path = document.createElementNS(svgNamespace, 'path');
      path.setAttribute('d', data);
      svg.append(path);
    }

    const label = this.label;
    if (label === undefined || label.trim() === '') {
      svg.setAttribute('aria-hidden', 'true');
      this.setAttribute('aria-hidden', 'true');
      this.removeAttribute('role');
      this.removeAttribute('aria-label');
    } else {
      this.removeAttribute('aria-hidden');
      this.setAttribute('role', 'img');
      this.setAttribute('aria-label', label);
      svg.setAttribute('aria-hidden', 'true');
    }

    root.append(svg);
    this.#svg = svg;
  }
}

/** Register the element. Idempotent, because a story file and a test may both ask. */
export function defineIcon(): void {
  if (customElements.get('mjx-icon') === undefined) customElements.define('mjx-icon', MjxIcon);
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-icon': MjxIcon;
  }
}
