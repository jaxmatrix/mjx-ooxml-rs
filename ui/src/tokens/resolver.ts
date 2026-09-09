/**
 * The TypeScript half of the token resolver.
 *
 * `DESIGN_TOKENS.md` §3 states the resolution order and the reason for it:
 *
 * > At runtime the shell resolves in this order — **explicit host configuration → CSS custom
 * > properties read off the host element → built-in defaults** — and pushes the resolved snapshot
 * > across the bridge to the renderer, so a theme change repaints canvas and chrome in the same
 * > frame. The shell watches for host changes with a `MutationObserver` and `prefers-color-scheme`.
 * >
 * > That resolution order is what satisfies the original brief: *if tokens are set, adopt them.*
 *
 * R01 (MJXOFF-156) defined that contract and built the Rust end. This is the other end.
 *
 * ## Why the three legs are three legs and not two
 *
 * The middle leg is the one that does the work in a browser: a host that already declares
 * `--color-green` re-themes the platform with no code change, because `getComputedStyle` on the
 * host resolves the cascade for us. The third leg is not a duplicate of `tokens.css` — it is what
 * answers when the stylesheet is **not loaded at all**, which is the ordinary case for a platform
 * embedded in someone else's application, and for the Rust canvas, which cannot read a custom
 * property in any circumstance. The first leg is programmatic configuration that has not been
 * written to the DOM — the shape a future `ShellBridge` hands us.
 *
 * `origin()` reports which leg answered, so a test can assert the *order* rather than only the
 * result. A resolver that returns the right value for the wrong reason passes an equality
 * assertion and fails a host.
 *
 * ## What is deliberately not here
 *
 * No token is defined in this file. Every default comes from `tokens.ts`, which is generated. The
 * only names written by hand below are the names of the *legs*.
 */

import {
  customProperties,
  tokens,
  type ColorScheme,
  type DocumentColors,
  type ThemeColors,
} from '../../tokens/tokens.ts';

/** Every token's dotted path, as the generated `customProperties` table spells it. */
export type TokenPath = keyof typeof customProperties & string;

/** Programmatic configuration: the first leg of the resolution order. */
export type TokenOverrides = Partial<Record<TokenPath, string>>;

/** Which leg of the resolution order answered a lookup. */
export type TokenOrigin = 'explicit' | 'host' | 'default';

/** A scheme-relative member of the semantic application palette — `surface`, `textPrimary`, … */
export type ThemeMember = keyof ThemeColors & string;

/** A scheme-relative member of the document-surface palette — `backdrop`, `page`, … */
export type DocumentMember = keyof DocumentColors & string;

/** How a host may ask for a colour scheme: a fixed one, or whatever the system prefers. */
export type SchemePreference = ColorScheme | 'system';

export interface TokenResolverOptions {
  /** The first leg: configuration handed to us rather than written to the DOM. */
  readonly explicit?: TokenOverrides;
  /** Called after any change the resolver watches actually moves a value. */
  readonly onChange?: (resolver: TokenResolver) => void;
  /** The window to read `matchMedia` from. Injected so a test can drive the media query. */
  readonly window?: Window;
}

/** Every token path the generator emitted, frozen once. */
export const tokenPaths: readonly TokenPath[] = Object.freeze(
  Object.keys(customProperties) as TokenPath[],
);

/** `surfaceRaised` → `surface-raised`, so a scheme member can name its custom property. */
export function customPropertyCase(member: string): string {
  return member.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`);
}

/**
 * The generated default for a token path — the third leg, read out of `tokens.ts` and never
 * written here.
 */
export function generatedDefault(path: TokenPath): string {
  let cursor: unknown = tokens;
  for (const segment of path.split('.')) {
    if (typeof cursor !== 'object' || cursor === null || !(segment in cursor)) {
      // Unreachable while `TokenPath` comes from the same generated file as `tokens`; the throw is
      // what makes that stay true if the two ever stop being emitted together.
      throw new Error(`the generated defaults have no token at '${path}'.`);
    }
    cursor = (cursor as Record<string, unknown>)[segment];
  }
  return String(cursor);
}

/**
 * Read a custom property off an element, or `undefined` when the cascade has nothing to say.
 *
 * A detached element and an element in a document that never loaded `tokens.css` both report the
 * empty string, which is exactly the condition the third leg exists for.
 */
function hostProperty(host: Element, property: string): string | undefined {
  const view = host.ownerDocument.defaultView;
  if (view === null) return undefined;
  const value = view.getComputedStyle(host).getPropertyValue(property).trim();
  return value === '' ? undefined : value;
}

/**
 * Write overrides onto an element as inline custom properties, so the *CSS* cascade adopts them
 * too.
 *
 * The resolver itself never writes to the DOM: `explicit` configuration and a host-declared custom
 * property are different legs and a resolver that quietly turned one into the other could not
 * report which answered. This function is the deliberate, named way to cross from one to the
 * other — what a shell calls when it wants a host's chrome to follow as well as its canvas.
 */
export function applyTokenOverrides(element: HTMLElement, overrides: TokenOverrides): void {
  for (const [path, value] of Object.entries(overrides)) {
    const property = customProperties[path];
    if (property === undefined) throw new Error(`no token is named '${path}'.`);
    if (value === undefined) element.style.removeProperty(property);
    else element.style.setProperty(property, value);
  }
}

/**
 * Set — or clear — the colour scheme on a root element.
 *
 * `tokens.css` keys its scheme layer off `:root[data-theme="…"]`, with `prefers-color-scheme`
 * between the two, so `'system'` is written by *removing* the attribute rather than by naming a
 * scheme. That is the whole mechanism: three CSS rules, and this function chooses between them.
 */
export function applyColorScheme(
  preference: SchemePreference,
  root: HTMLElement = document.documentElement,
): void {
  if (preference === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', preference);
}

/**
 * Resolves tokens for one host element, and keeps resolving them as the host changes.
 *
 * Construct one per host — the shell has one, and a component that wants to be embeddable
 * separately may have its own. `dispose()` releases both observers; a resolver that is dropped
 * without it keeps a `MutationObserver` alive for the life of the document.
 */
export class TokenResolver {
  readonly #host: Element;
  readonly #window: Window;
  readonly #onChange: ((resolver: TokenResolver) => void) | undefined;
  #explicit: TokenOverrides;
  #observer: MutationObserver | undefined;
  #media: MediaQueryList | undefined;
  #lastSnapshot: Record<string, string> | undefined;
  #flushScheduled = false;
  #disposed = false;

  constructor(host: Element, options: TokenResolverOptions = {}) {
    this.#host = host;
    const view = options.window ?? host.ownerDocument.defaultView;
    if (view === null || view === undefined) {
      throw new Error('a token resolver needs a window: the host element is not in a document.');
    }
    this.#window = view;
    this.#explicit = { ...options.explicit };
    this.#onChange = options.onChange;
    if (this.#onChange !== undefined) this.#watch();
  }

  /** The value in force for `path`, by the resolution order. */
  get(path: TokenPath): string {
    const explicit = this.#explicit[path];
    if (explicit !== undefined) return explicit;
    const property = customProperties[path];
    if (property === undefined) throw new Error(`no token is named '${path}'.`);
    return hostProperty(this.#host, property) ?? generatedDefault(path);
  }

  /** Which leg answered `get(path)`. */
  origin(path: TokenPath): TokenOrigin {
    if (this.#explicit[path] !== undefined) return 'explicit';
    const property = customProperties[path];
    if (property === undefined) throw new Error(`no token is named '${path}'.`);
    return hostProperty(this.#host, property) === undefined ? 'default' : 'host';
  }

  /**
   * The colour scheme in force: an explicit `data-theme` on the host or an ancestor wins, and the
   * system preference decides when nothing has asked.
   *
   * The order mirrors `tokens.css`'s three rules exactly, because a resolver that disagreed with
   * the stylesheet would paint the canvas in one scheme and the chrome in the other.
   */
  scheme(): ColorScheme {
    const asked = this.#host.closest('[data-theme]')?.getAttribute('data-theme');
    if (asked === 'dark' || asked === 'light') return asked;
    return this.#prefersDark() ? 'dark' : 'light';
  }

  /** A scheme-relative application colour — `--theme-surface` and its kin. */
  theme(member: ThemeMember): string {
    return this.#schemeRelative('theme', member, tokens.theme[this.scheme()][member]);
  }

  /** A scheme-relative document-surface colour — `--document-page` and its kin. */
  document(member: DocumentMember): string {
    return this.#schemeRelative('document', member, tokens.document[this.scheme()][member]);
  }

  /** Every token, resolved. This is the shape the bridge hands the renderer. */
  snapshot(): Readonly<Record<TokenPath, string>> {
    const out: Record<string, string> = {};
    for (const path of tokenPaths) out[path] = this.get(path);
    return out as Record<TokenPath, string>;
  }

  /** Replace the explicit configuration. Fires `onChange` if it moved anything. */
  setExplicit(explicit: TokenOverrides): void {
    this.#explicit = { ...explicit };
    this.#flush();
  }

  /** Release the observers. Safe to call twice. */
  dispose(): void {
    if (this.#disposed) return;
    this.#disposed = true;
    this.#observer?.disconnect();
    this.#observer = undefined;
    this.#media?.removeEventListener('change', this.#onMediaChange);
    this.#media = undefined;
  }

  // ── internals ──────────────────────────────────────────────────────────────

  #prefersDark(): boolean {
    return this.#window.matchMedia?.('(prefers-color-scheme: dark)').matches === true;
  }

  #schemeRelative(group: 'theme' | 'document', member: string, fallback: string): string {
    const property = `--${group}-${customPropertyCase(member)}`;
    return hostProperty(this.#host, property) ?? fallback;
  }

  #watch(): void {
    this.#lastSnapshot = this.snapshot();

    // The host's own attributes, because that is where a host writes an override; and the root's,
    // because `tokens.css`'s scheme layer is keyed off `:root[data-theme]` and a scheme change
    // therefore never touches the host at all. Attribute-filtered: a resolver that recomputed on
    // every DOM mutation in an editor would be a performance defect wearing a correctness costume.
    this.#observer = new MutationObserver(() => this.#schedule());
    const filter = { attributes: true, attributeFilter: ['style', 'class', 'data-theme'] };
    this.#observer.observe(this.#host, filter);
    const root = this.#host.ownerDocument.documentElement;
    if (root !== this.#host) this.#observer.observe(root, filter);

    this.#media = this.#window.matchMedia?.('(prefers-color-scheme: dark)');
    this.#media?.addEventListener('change', this.#onMediaChange);
  }

  readonly #onMediaChange = (): void => this.#schedule();

  /**
   * Coalesce to one recomputation per microtask.
   *
   * `snapshot()` calls `getComputedStyle` once per token, and a host that writes eight custom
   * properties in a loop would otherwise pay for all of it eight times. A mutation observer's
   * callback is already a microtask, so this collapses a burst into one pass without deferring the
   * notification past the frame.
   */
  #schedule(): void {
    if (this.#flushScheduled || this.#disposed) return;
    this.#flushScheduled = true;
    queueMicrotask(() => {
      this.#flushScheduled = false;
      this.#flush();
    });
  }

  #flush(): void {
    if (this.#onChange === undefined || this.#disposed) return;
    const next = this.snapshot();
    const previous = this.#lastSnapshot;
    if (previous !== undefined && !moved(previous, next)) return;
    this.#lastSnapshot = { ...next };
    this.#onChange(this);
  }
}

/** Whether two snapshots differ. Cheaper than a deep compare and exact for a flat string map. */
function moved(previous: Record<string, string>, next: Readonly<Record<string, string>>): boolean {
  for (const path of tokenPaths) {
    if (previous[path] !== next[path]) return true;
  }
  return false;
}
