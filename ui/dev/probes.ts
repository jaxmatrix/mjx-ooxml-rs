/**
 * Throwaway probes. **These are not components and nothing may depend on them.**
 *
 * ## Why they exist
 *
 * MJXOFF-180's trap, in its own words:
 *
 * > *"Storybook builds and stories render"* is satisfied by a Storybook with no theming, no
 * > responsiveness and a disabled a11y rule — and every one of those would then silently stay off
 * > for fifteen further children. **So the gates below are about the harness's own machinery
 * > failing correctly**, proved on a deliberately bad throwaway component rather than on a real
 * > one.
 *
 * A gate is only known to work when it has been watched failing. Proving that on a *real*
 * component means keeping a real component broken, which nobody does for long; proving it on a
 * throwaway means the broken thing has no other job and cannot be quietly fixed. So each probe is
 * the smallest element that can make one piece of harness machinery produce a verdict:
 *
 * | Probe | Proves |
 * |---|---|
 * | `mjx-contrast-probe` | the a11y contrast rule is enforced, and can reject |
 * | `mjx-container-probe` | the resizable container, and not the viewport, is the mechanism |
 * | `mjx-token-probe` | explicit → host property → generated default, in that order |
 * | `mjx-document-surface-probe` | the page stays true white while the backdrop changes |
 *
 * ## Why they are light-DOM and inline-styled
 *
 * A shipped component is a custom element with Shadow DOM. A probe deliberately is not: the tests
 * read its rendered text and its computed styles, and `innerText` does not cross a shadow boundary.
 * Making the probes plain is what keeps the *assertions* simple enough to be obviously right,
 * which matters more here than in a component, because these tests are the evidence for everything
 * else.
 */

import { customProperties } from '../tokens/tokens.ts';
import { TokenResolver, type TokenPath } from '../src/tokens/resolver.ts';
import { containerName } from '../src/harness/presets.ts';
import { containerBands, containerBreakpoints, type ContainerBand } from './bands.ts';
import { firstColorToken } from './token-choice.ts';

export { containerBands, containerBreakpoints, firstColorToken, type ContainerBand };

// ── the contrast probe ───────────────────────────────────────────────────────

/**
 * A paragraph of body text in whatever colour it is told, on whatever background it is told.
 *
 * That is the whole element. `DESIGN_TOKENS.md` §2.2's rule is about *body text*, so the probe has
 * to be body text: axe's `color-contrast` rule measures rendered glyphs against their rendered
 * background and has no opinion about a swatch.
 */
export class MjxContrastProbe extends HTMLElement {
  static readonly observedAttributes = ['color', 'background', 'label'];

  connectedCallback(): void {
    this.#render();
  }

  attributeChangedCallback(): void {
    if (this.isConnected) this.#render();
  }

  #render(): void {
    const color = this.getAttribute('color') ?? 'currentColor';
    const background = this.getAttribute('background') ?? '#ffffff';
    const label = this.getAttribute('label') ?? 'Body text at the declared colour.';
    this.style.display = 'block';
    this.style.background = background;
    this.style.padding = '16px';
    this.style.borderRadius = '10px';
    this.innerHTML = '';
    const paragraph = this.ownerDocument.createElement('p');
    paragraph.style.margin = '0';
    paragraph.style.color = color;
    paragraph.style.font = '400 16px/1.7 system-ui, sans-serif';
    paragraph.textContent = label;
    this.append(paragraph);
  }
}

// ── the container probe ──────────────────────────────────────────────────────

const containerStyleId = 'mjx-container-probe-styles';

/**
 * These are `@container` queries and nothing else. There is deliberately no `@media` rule in this
 * file: if one crept in, the container test would pass for the wrong reason at exactly the
 * viewport widths the harness is usually opened at.
 */
const containerStyles = `
  mjx-container-probe { display: block; font: 500 14px/1.4 system-ui, sans-serif; }
  mjx-container-probe .band { display: none; }
  mjx-container-probe .band[data-band='narrow'] { display: block; }
  @container ${containerName} (min-width: ${String(containerBreakpoints.medium)}px) {
    mjx-container-probe .band[data-band='narrow'] { display: none; }
    mjx-container-probe .band[data-band='medium'] { display: block; }
  }
  @container ${containerName} (min-width: ${String(containerBreakpoints.wide)}px) {
    mjx-container-probe .band[data-band='medium'] { display: none; }
    mjx-container-probe .band[data-band='wide'] { display: block; }
  }
`;

export class MjxContainerProbe extends HTMLElement {
  connectedCallback(): void {
    const document_ = this.ownerDocument;
    if (document_.getElementById(containerStyleId) === null) {
      const sheet = document_.createElement('style');
      sheet.id = containerStyleId;
      sheet.textContent = containerStyles;
      document_.head.append(sheet);
    }
    if (this.childElementCount > 0) return;
    for (const band of containerBands) {
      const element = document_.createElement('div');
      element.className = 'band';
      element.dataset['band'] = band;
      element.textContent = `container band: ${band}`;
      this.append(element);
    }
  }

  /** The band currently displayed — what a test reads, and what the auditor sees. */
  get band(): ContainerBand | undefined {
    const view = this.ownerDocument.defaultView;
    if (view === null) return undefined;
    for (const band of containerBands) {
      const element = this.querySelector<HTMLElement>(`.band[data-band='${band}']`);
      if (element !== null && view.getComputedStyle(element).display !== 'none') return band;
    }
    return undefined;
  }
}

// ── the token-resolution probe ───────────────────────────────────────────────

/**
 * Resolves one token and reports both the value and *which leg answered*.
 *
 * The origin is the load-bearing half. A probe that only reported the value would pass its test
 * whenever the answer happened to be right, including when the host override was ignored and the
 * default coincidentally matched.
 */
export class MjxTokenProbe extends HTMLElement {
  static readonly observedAttributes = ['token', 'explicit'];

  #resolver: TokenResolver | undefined;
  #swatch: HTMLElement | undefined;
  #caption: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#swatch === undefined) this.#build();
    this.#build_resolver();
    this.refresh();
  }

  disconnectedCallback(): void {
    this.#resolver?.dispose();
    this.#resolver = undefined;
  }

  attributeChangedCallback(name: string): void {
    if (this.#resolver === undefined) return;
    if (name === 'explicit') this.#build_resolver();
    this.refresh();
  }

  /** The token this probe reads. */
  get token(): TokenPath {
    const declared = this.getAttribute('token');
    return declared !== null && declared in customProperties
      ? (declared as TokenPath)
      : firstColorToken();
  }

  /**
   * The first leg: programmatic configuration that has *not* been written to the DOM.
   *
   * A probe needs it because a browser cannot otherwise reach that leg — `explicit` is what a
   * future `ShellBridge` hands the resolver, and a gate that only exercised the two DOM legs would
   * leave the highest-priority one unproved.
   */
  get explicit(): string | undefined {
    return this.getAttribute('explicit') ?? undefined;
  }

  #build_resolver(): void {
    this.#resolver?.dispose();
    const explicit = this.explicit;
    this.#resolver = new TokenResolver(this, {
      onChange: () => this.refresh(),
      ...(explicit === undefined ? {} : { explicit: { [this.token]: explicit } }),
    });
  }

  /** Recompute now. `MutationObserver` calls this on its own; a test may call it directly. */
  refresh(): void {
    const resolver = this.#resolver;
    if (resolver === undefined) return;
    const token = this.token;
    const value = resolver.get(token);
    const origin = resolver.origin(token);
    this.dataset['resolved'] = value;
    this.dataset['origin'] = origin;
    if (this.#swatch !== undefined) this.#swatch.style.background = value;
    if (this.#caption !== undefined) {
      this.#caption.textContent = `${token} = ${value} (from the ${origin})`;
    }
  }

  #build(): void {
    this.style.display = 'flex';
    this.style.alignItems = 'center';
    this.style.gap = '12px';
    const swatch = this.ownerDocument.createElement('span');
    swatch.style.inlineSize = '40px';
    swatch.style.blockSize = '40px';
    swatch.style.borderRadius = '8px';
    swatch.style.border = '1px solid var(--theme-border)';
    const caption = this.ownerDocument.createElement('span');
    caption.style.font = '400 14px/1.4 ui-monospace, monospace';
    caption.style.color = 'var(--theme-text-primary)';
    this.append(swatch, caption);
    this.#swatch = swatch;
    this.#caption = caption;
  }
}

// ── the document-surface probe ───────────────────────────────────────────────

/**
 * A page on a backdrop, and nothing else.
 *
 * `DESIGN_TOKENS.md` §2.3: *the page stays true white in both themes* while the canvas backdrop
 * behind it changes. Both halves are read from the scheme-relative aliases, so the probe never
 * names a scheme and the test can switch themes underneath it.
 */
export class MjxDocumentSurfaceProbe extends HTMLElement {
  static readonly observedAttributes = ['scheme'];

  connectedCallback(): void {
    this.#render();
  }

  attributeChangedCallback(): void {
    if (this.isConnected) this.#render();
  }

  /**
   * Which set of custom properties to read.
   *
   * ⚠ **`tokens.css`'s scheme layer is `:root`-scoped.** Its three rules are `:root`,
   * `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` and
   * `:root[data-theme="dark"]`, so `--document-backdrop` resolves for the *document*, and putting
   * `data-theme="dark"` on a `<div>` changes nothing. That is correct for an application with one
   * theme at a time and it means a subtree cannot carry its own scheme — which is why this probe
   * can also name a scheme outright, reading `--document-light-*` / `--document-dark-*`, which the
   * generator emits unconditionally at `:root`.
   */
  get scheme(): 'light' | 'dark' | undefined {
    const declared = this.getAttribute('scheme');
    return declared === 'light' || declared === 'dark' ? declared : undefined;
  }

  #property(name: string): string {
    const scheme = this.scheme;
    return scheme === undefined ? `var(--document-${name})` : `var(--document-${scheme}-${name})`;
  }

  #render(): void {
    this.innerHTML = '';
    this.style.display = 'block';
    this.dataset['role'] = 'document-surface';
    const backdrop = this.ownerDocument.createElement('div');
    backdrop.dataset['part'] = 'backdrop';
    backdrop.style.background = this.#property('backdrop');
    backdrop.style.padding = '32px';
    backdrop.style.borderRadius = '16px';
    const page = this.ownerDocument.createElement('div');
    page.dataset['part'] = 'page';
    page.style.background = this.#property('page');
    page.style.border = `1px solid ${this.#property('page-border')}`;
    page.style.boxShadow = this.#property('page-shadow');
    page.style.blockSize = '160px';
    page.style.inlineSize = 'min(100%, 320px)';
    page.style.marginInline = 'auto';
    backdrop.append(page);
    this.append(backdrop);
  }
}

// ── registration ─────────────────────────────────────────────────────────────

const registry: readonly [string, CustomElementConstructor][] = [
  ['mjx-contrast-probe', MjxContrastProbe],
  ['mjx-container-probe', MjxContainerProbe],
  ['mjx-token-probe', MjxTokenProbe],
  ['mjx-document-surface-probe', MjxDocumentSurfaceProbe],
];

/** Register every probe. Idempotent. */
export function defineProbes(): void {
  for (const [name, constructor] of registry) {
    if (customElements.get(name) === undefined) customElements.define(name, constructor);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-contrast-probe': MjxContrastProbe;
    'mjx-container-probe': MjxContainerProbe;
    'mjx-token-probe': MjxTokenProbe;
    'mjx-document-surface-probe': MjxDocumentSurfaceProbe;
  }
}
