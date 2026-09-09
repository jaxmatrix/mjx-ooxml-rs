/**
 * `<mjx-resizable-container>` — the responsive harness.
 *
 * `BUILD_PLAN_LOOP_1.md` §3:
 *
 * > **Responsive is proved by container, not viewport.** The chrome is container-query driven, so
 * > every component story renders in a resizable frame with desktop / tablet / phone presets. A
 * > component that only looks right at a viewport width has not been validated.
 *
 * ## Why this is not the viewport addon
 *
 * A viewport addon resizes the *iframe*. That answers "does this look right on a phone-sized
 * screen", which is the wrong question for chrome that will be laid out inside a task pane, a
 * split editor or a docked inspector — none of which have anything to do with the window's width.
 * The mechanism this platform actually ships is `@container`, so the harness has to be a
 * **container**: an element with `container-type: inline-size` whose width the auditor changes
 * while the window stays exactly where it is.
 *
 * `tests/browser/container.spec.ts` asserts that difference directly — it drives this element
 * across all three presets and asserts `window.innerWidth` never moves. A harness that quietly
 * resized the viewport would pass a screenshot review and fail that test.
 *
 * ## Accessibility of the harness itself
 *
 * The auditor uses this on every story, so it is chrome and not scaffolding: the presets are real
 * `<button>`s, the width is a labelled `<input type="range">`, and the pointer-drag affordance is
 * the platform's own `resize: inline` rather than a mouse-only handle written here. The a11y sweep
 * runs over every story, so this element is audited on every one of them.
 */

import {
  containerName,
  containerPresetOrder,
  containerPresets,
  containerWidthBounds,
  type ContainerPreset,
} from './presets.ts';

export {
  containerName,
  containerPresetOrder,
  containerPresets,
  containerWidthBounds,
  type ContainerPreset,
};

const template = `
  <div class="harness" part="harness">
    <div class="chrome" part="chrome">
      <div class="presets" role="group" aria-label="Container width presets">
        ${containerPresetOrder
          .map(
            (preset) =>
              `<button type="button" data-preset="${preset}">${preset} · ${String(containerPresets[preset])}</button>`,
          )
          .join('')}
      </div>
      <label class="width">
        <span>Width</span>
        <input type="range" min="${String(containerWidthBounds.min)}" max="${String(containerWidthBounds.max)}" step="1" />
        <output aria-live="off"></output>
      </label>
    </div>
    <div class="stage" part="stage">
      <div class="frame" part="frame">
        <slot></slot>
      </div>
    </div>
  </div>
`;

const styles = `
  :host {
    display: block;
    font-family: var(--font-sans);
    color: var(--theme-text-primary);
  }
  .harness {
    display: flex;
    flex-direction: column;
    gap: calc(var(--spacing) * 2);
    align-items: flex-start;
  }
  .chrome {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--spacing) * 3);
    padding: calc(var(--spacing) * 2) calc(var(--spacing) * 3);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-control);
    background: var(--theme-surface);
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
  }
  .presets { display: flex; gap: var(--spacing); }
  button {
    font: inherit;
    color: var(--theme-text-primary);
    background: var(--theme-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-chip);
    padding: var(--spacing) calc(var(--spacing) * 2);
    cursor: pointer;
    transition: background var(--duration-transition) var(--ease-ink);
  }
  button:hover { background: var(--theme-accent-surface); }
  button[aria-pressed='true'] {
    background: var(--theme-accent-surface);
    border-color: var(--theme-accent-border);
    color: var(--theme-accent-pressed);
  }
  button:focus-visible, input:focus-visible {
    outline: 2px solid var(--theme-accent-pressed);
    outline-offset: 2px;
  }
  .width { display: flex; align-items: center; gap: var(--spacing); }
  output { font-variant-numeric: tabular-nums; min-width: 5ch; }
  .frame {
    /* The whole point of the element. Everything slotted in queries THIS.
     *
     * ⚠ It has NO padding and NO border, and that is load-bearing rather than a style choice.
     * A container query resolves against the container's CONTENT box, so 16px of padding and a
     * 1px border would make a frame set to 700px report 666px to a container query — every
     * component in the catalogue would then hit its breakpoints 34px later than it was authored
     * for, and nothing would say so. The first version of this element had exactly that defect
     * and tests/browser/container.spec.ts caught it on the breakpoint boundary.
     *
     * The visible edge is an outline, which does not participate in layout at all, and the
     * breathing room is on .stage outside the frame. container.spec.ts asserts that the frame's
     * measured content width equals the declared width, so a padding added here later fails a
     * test rather than quietly shifting fifteen components' breakpoints. */
    container-type: inline-size;
    container-name: ${containerName};
    box-sizing: border-box;
    overflow: auto;
    resize: horizontal;
    outline: 1px solid var(--theme-border-subtle);
    border-radius: var(--radius-card);
    background: var(--theme-background);
  }
  /* ⚠ The frame is NOT clamped to the viewport, and that is the second half of the same decision.
   * A max-inline-size of 100% here would mean that opening the desktop preset in a 1280px window
   * silently gave the component 1280px — the preset would be a label rather than a width, and the
   * container test would pass for the wrong reason because 1280 and 1440 happen to fall in the
   * same band. So the stage scrolls instead, and the frame is exactly the width it says it is. */
  .stage {
    padding: calc(var(--spacing) * 2);
    max-inline-size: 100%;
    overflow-x: auto;
  }
`;

/**
 * The harness element. `width` is the source of truth; `preset` is a convenience that writes it.
 */
export class MjxResizableContainer extends HTMLElement {
  static readonly observedAttributes = ['width', 'preset'];

  #frame: HTMLElement | undefined;
  #range: HTMLInputElement | undefined;
  #output: HTMLOutputElement | undefined;
  #observer: ResizeObserver | undefined;

  connectedCallback(): void {
    if (this.shadowRoot === null) this.#render();
    this.#sync();
  }

  disconnectedCallback(): void {
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  attributeChangedCallback(name: string): void {
    // ⚠ No early return on an un-rendered shadow root. Attributes are set on a custom element
    // *before* it is connected — lit builds the element, writes `preset`, and only then puts it in
    // the document — so a callback that ignored a pre-connection change dropped the preset
    // entirely and every story silently opened at the default width. `#sync()` is a no-op until
    // `#render()` has run, and `connectedCallback` calls it again afterwards, so the ordering
    // takes care of itself without a guard that loses information.
    //
    // `preset` and `width` are two spellings of one value and the last one written wins, which is
    // why setting either clears the other.
    if (name === 'preset' && this.hasAttribute('width')) this.removeAttribute('width');
    this.#sync();
  }

  /**
   * The frame's width in CSS pixels — what a container query sees.
   *
   * An explicit `width` wins; otherwise a named `preset`; otherwise the widest preset. The frame is
   * never clamped to the viewport, so this number is what the component actually gets.
   */
  get width(): number {
    const declared = Number(this.getAttribute('width'));
    if (Number.isFinite(declared) && declared > 0) return declared;
    const preset = this.getAttribute('preset');
    if (preset !== null && preset in containerPresets) {
      return containerPresets[preset as ContainerPreset];
    }
    return containerPresets.desktop;
  }

  set width(value: number) {
    this.removeAttribute('preset');
    this.setAttribute('width', String(Math.round(value)));
  }

  /** Which preset, if the current width is exactly one of them. */
  get preset(): ContainerPreset | undefined {
    return containerPresetOrder.find((name) => containerPresets[name] === this.width);
  }

  #render(): void {
    const root = this.attachShadow({ mode: 'open' });
    const sheet = document.createElement('style');
    sheet.textContent = styles;
    root.append(sheet);
    const holder = document.createElement('div');
    holder.innerHTML = template;
    root.append(...holder.childNodes);

    this.#frame = root.querySelector('.frame') ?? undefined;
    this.#range = root.querySelector('input[type="range"]') ?? undefined;
    this.#output = root.querySelector('output') ?? undefined;

    for (const button of root.querySelectorAll<HTMLButtonElement>('button[data-preset]')) {
      button.addEventListener('click', () => {
        const preset = button.dataset['preset'];
        if (preset !== undefined && preset in containerPresets) {
          this.width = containerPresets[preset as ContainerPreset];
        }
      });
    }
    this.#range?.addEventListener('input', () => {
      const value = Number(this.#range?.value);
      if (Number.isFinite(value)) this.width = value;
    });

    // A pointer drag on `resize: horizontal` changes the layout width without changing the
    // attribute, so the readout is driven by what the box actually is rather than by what was
    // asked for. Those two agree until someone drags, and the readout has to follow the drag.
    if (this.#frame !== undefined && typeof ResizeObserver !== 'undefined') {
      this.#observer = new ResizeObserver((entries) => {
        const measured = entries[0]?.contentBoxSize?.[0]?.inlineSize;
        if (measured !== undefined) this.#report(Math.round(measured));
      });
      this.#observer.observe(this.#frame);
    }
  }

  #sync(): void {
    if (this.shadowRoot === null) return;
    const width = this.width;
    if (this.#frame !== undefined) this.#frame.style.inlineSize = `${String(width)}px`;
    if (this.#range !== undefined) this.#range.value = String(width);
    this.#report(width);
    const active = this.preset;
    for (const button of this.shadowRoot?.querySelectorAll<HTMLButtonElement>(
      'button[data-preset]',
    ) ?? []) {
      button.setAttribute('aria-pressed', String(button.dataset['preset'] === active));
    }
  }

  #report(width: number): void {
    if (this.#output !== undefined) this.#output.textContent = `${String(width)}px`;
  }
}

/** Register the element. Idempotent, because a story file and a test may both ask. */
export function defineResizableContainer(): void {
  if (customElements.get('mjx-resizable-container') === undefined) {
    customElements.define('mjx-resizable-container', MjxResizableContainer);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-resizable-container': MjxResizableContainer;
  }
}
