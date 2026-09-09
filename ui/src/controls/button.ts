/**
 * `<mjx-button>` — the archetype 8,687 of Office's 15,346 published controls use.
 *
 * ```html
 * <mjx-button label="Paste" icon="folder-open" size="large"></mjx-button>
 * <mjx-button label="Bold" icon="text-bold" size="icon"></mjx-button>
 * <mjx-button label="Delete" icon="delete" unavailable
 *             explanation="Select something first."></mjx-button>
 * ```
 *
 * ## A native `<button>`, inside the shadow root
 *
 * Enter and Space, the `button` role, the disabled semantics and the suppression of activation
 * while disabled are all things the platform already does correctly, and a `<div role="button">`
 * with hand-written key handling is a re-implementation of four behaviours in order to avoid one
 * element. So the shadow root holds a real `<button>`; the host has no role of its own and adds
 * none.
 *
 * Sequential focus navigation descends into a shadow tree, so `Tab` reaches the inner button
 * without `delegatesFocus`, and the focus ring the foundations install lands on the thing that is
 * actually focused. `tests/browser/controls.spec.ts` presses both keys through the browser's own
 * input path rather than dispatching synthetic events, because a synthetic `keydown` proves that a
 * handler exists and proves nothing about whether the platform would ever call it.
 *
 * ## The accessible name is the visible label, and it is never dropped
 *
 * The label is a `<span>` inside the button, so the name is computed from content and no
 * `aria-label` overrides anything. At `size="icon"` that span becomes visually hidden rather than
 * absent: an icon-only command is still announced by its name, which is the whole difference
 * between a toolbar a screen-reader user can use and one they cannot.
 *
 * A button with no `label` renders a nameless button and says so on the console. It does **not**
 * invent a name from the icon: an icon's Fluent name is a drawing's name, not a command's, and a
 * button announced as *"text bold 20 regular"* is worse than one axe can catch.
 * `Gates/Control Naming` is the story that keeps that catchable.
 *
 * ## Two kinds of unavailable
 *
 * `disabled` is the platform's. `unavailable` is Office's — greyed, still reachable, and carrying
 * the reason. `control-element.ts` holds the distinction; this component only forwards it.
 */

import { defineIcon } from '../icons/icon.ts';
import type { IconSize, IconVariant } from '../icons/manifest.ts';
import {
  applyAvailability,
  controlClassName,
  controlEvents,
  createExplanationElement,
  emitControlEvent,
  explanationElementId,
  forcedState,
  installControlStyles,
  refusesActivation,
} from './control-element.ts';
import {
  controlBaseCss,
  controlSizeCss,
  controlSizes,
  controlStatesCss,
  defaultControlSize,
  isControlSize,
  type ControlSize,
} from './control-states.ts';

/** The whole sheet, composed once. Shared by every instance — see `installControlStyles`. */
export const buttonCss = [controlBaseCss, controlSizeCss, controlStatesCss('.control')].join('\n');

/** The attributes every button-shaped archetype observes. */
export const buttonAttributes = [
  'label',
  'icon',
  'size',
  'disabled',
  'unavailable',
  'explanation',
  'force-state',
] as const;

export class MjxButton extends HTMLElement {
  static readonly observedAttributes: readonly string[] = buttonAttributes;

  #root: ShadowRoot | undefined;
  #button: HTMLButtonElement | undefined;
  #labelElement: HTMLElement | undefined;
  #iconElement: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;
  /** Names already reported, so a ribbon of nameless buttons prints one line per shape. */
  static readonly #reported = new Set<string>();

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  attributeChangedCallback(): void {
    // No `isConnected` guard, for the reason `<mjx-icon>` states: lit writes every attribute
    // before it inserts the element.
    if (this.#root !== undefined) this.render();
  }

  /** The command's name. It is the visible label *and* the accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /**
   * The Fluent icon's name, or `undefined` for a text-only button.
   *
   * A subclass whose glyph is part of what it *is* — the dialog launcher's corner arrow — supplies
   * it through `fixedIcon()` rather than by overriding this accessor, so the attribute path and
   * the fixed path stay one implementation.
   */
  get icon(): string | undefined {
    const declared = this.getAttribute('icon');
    if (declared !== null && declared !== '') return declared;
    return this.fixedIcon();
  }

  /** `large`, `small` or `icon`. A subclass with only one shape returns it from `fixedSize()`. */
  get size(): ControlSize {
    const fixed = this.fixedSize();
    if (fixed !== undefined) return fixed;
    const declared = this.getAttribute('size');
    return isControlSize(declared) ? (declared as ControlSize) : defaultControlSize;
  }

  set size(value: ControlSize) {
    this.setAttribute('size', value);
  }

  /** The glyph a subclass always draws, or `undefined` to take it from the attribute. */
  protected fixedIcon(): string | undefined {
    return undefined;
  }

  /** The one shape a subclass has, or `undefined` to take it from the attribute. */
  protected fixedSize(): ControlSize | undefined {
    return undefined;
  }

  /** The inner button, for a test and for a subclass. `undefined` before the first connection. */
  protected get control(): HTMLButtonElement | undefined {
    return this.#button;
  }

  /** The shadow root, for a subclass. */
  protected get shadow(): ShadowRoot | undefined {
    return this.#root;
  }

  /** The stylesheet this archetype adopts. A subclass overrides it to add its own rules. */
  protected get styles(): string {
    return buttonCss;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    // The foundations first, then this component's own rules. Order matters and is explained in
    // `control-element.ts`; a component that installed them the other way round would lose every
    // tie to `.mjx-type-control`.
    installControlStyles(root, this.styles);

    const button = document.createElement('button');
    button.type = 'button';
    button.className = controlClassName;
    button.setAttribute('part', 'control');
    button.addEventListener('click', this.#onClick);

    const icon = document.createElement('mjx-icon');
    const label = document.createElement('span');
    const explanation = createExplanationElement(explanationElementId);

    button.append(icon, label);
    root.append(button, explanation);

    this.#button = button;
    this.#iconElement = icon;
    this.#labelElement = label;
    this.#explanationElement = explanation;
  }

  #onClick = (event: MouseEvent): void => {
    if (refusesActivation(this)) {
      // An `aria-disabled` button is still a button as far as the platform is concerned, so the
      // refusal has to be written down. Stopping propagation as well as preventing the default is
      // what keeps a ribbon's own click listener from seeing an activation the control refused.
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    this.activated(event);
  };

  /** What one activation does. `<mjx-button>` reports it; a toggle moves first. */
  protected activated(_event: MouseEvent): void {
    emitControlEvent(this, controlEvents.activate);
  }

  /** Re-render from the attributes. Safe to call at any time; a no-op before the first build. */
  render(): void {
    const button = this.#button;
    const labelElement = this.#labelElement;
    const iconElement = this.#iconElement;
    const explanationElement = this.#explanationElement;
    if (
      button === undefined ||
      labelElement === undefined ||
      iconElement === undefined ||
      explanationElement === undefined
    ) {
      return;
    }

    const size = this.size;
    const spec = controlSizes[size];
    button.dataset['size'] = size;

    const icon = this.icon;
    if (icon === undefined) {
      iconElement.remove();
    } else {
      iconElement.setAttribute('name', icon);
      iconElement.setAttribute('size', String(this.iconRenderSize(spec.iconSize)));
      iconElement.setAttribute(
        'variant',
        this.iconVariant(icon, this.iconRenderSize(spec.iconSize)),
      );
      if (iconElement.parentNode === null) button.prepend(iconElement);
    }

    labelElement.textContent = this.label;
    labelElement.className = spec.labelVisible ? 'label' : 'visually-hidden';

    const forced = forcedState(this);
    if (forced === undefined) delete button.dataset['state'];
    else button.dataset['state'] = forced;

    applyAvailability(this, button, explanationElement);
    this.rendered(button);
    this.#warnIfNameless();
  }

  /**
   * Which of Fluent's drawings this control renders at.
   *
   * The size ladder belongs to the size variant, so this returns it unchanged — except for the
   * dialog launcher, whose glyph is a mark in a corner rather than a command's icon and is drawn
   * one rung down. Fluent draws each size separately, so this chooses a *drawing*, never a scale
   * factor.
   */
  protected iconRenderSize(size: IconSize): IconSize {
    return size;
  }

  /** Which drawing this control wants. A toggle overrides it: Fluent draws `filled` for selected. */
  protected iconVariant(_name: string, _size: IconSize): IconVariant {
    return 'regular';
  }

  /** A hook for a subclass, called at the end of every render. */
  protected rendered(_button: HTMLButtonElement): void {
    // Nothing.
  }

  #warnIfNameless(): void {
    if (this.label.trim() !== '') return;
    const key = `${this.localName}:${this.icon ?? '(no icon)'}`;
    if (MjxButton.#reported.has(key)) return;
    MjxButton.#reported.add(key);
    console.error(
      `<${this.localName}> has no 'label', so it renders a button with no accessible name. A ` +
        "screen reader announces it as 'button' and nothing else. The icon's name is a drawing's " +
        "name, not a command's, so nothing is invented from it — give the control a label.",
    );
  }
}

/** Register the element. Idempotent, because a story file and a test may both ask. */
export function defineButton(): void {
  defineIcon();
  if (customElements.get('mjx-button') === undefined) customElements.define('mjx-button', MjxButton);
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-button': MjxButton;
  }
}
