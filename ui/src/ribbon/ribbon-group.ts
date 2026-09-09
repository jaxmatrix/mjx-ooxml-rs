/**
 * `<mjx-ribbon-group>` — the labelled container 1,193 of Office's published groups are, and **the
 * hardest responsive problem in the catalogue**.
 *
 * ```html
 * <mjx-ribbon-group label="Font" priority="primary">
 *   <mjx-toggle-button slot="essential" label="Bold" icon="text-bold" size="icon"></mjx-toggle-button>
 *   <mjx-button label="Underline" icon="text-underline"></mjx-button>
 *   <mjx-dialog-launcher slot="dialog-launcher" label="Font settings"></mjx-dialog-launcher>
 * </mjx-ribbon-group>
 * ```
 *
 * ## One slot, three presentations — which is why nothing can be lost
 *
 * The obvious way to build a collapsing group is to render the commands twice: once in the strip
 * and once in a menu. That is also the way to lose one, and MJXOFF-183 says exactly what is at
 * stake: *"a control that disappears at narrow width has not degraded, it has been lost. That
 * reachability assertion is the one that matters."*
 *
 * So this element renders its commands **once**, into one `<slot>`, inside one `.panel` element.
 * What the three presentations change is what that element *is*: part of the strip in `full` and
 * `reduced`, and an overlay anchored to a single button in `collapsed`. The commands are the same
 * DOM nodes at every width, so *reachable in all three* is structural rather than something this
 * component has to remember — and `tests/browser/ribbon.spec.ts` still counts them at all three,
 * because a structural guarantee nobody has watched hold is a guarantee about a file.
 *
 * ## The layout is CSS's decision; this file only reads it back
 *
 * There is no `ResizeObserver` here and no measurement of any kind. `src/ribbon/ribbon-model.ts`
 * generates eight `@container` blocks from the priority ladder, each of which writes
 * `--mjx-group-presentation`, and `presentation` below is a single `getComputedStyle` read of that
 * custom property. That is what lets the *behaviour* (a popup traps focus; a panel does not) follow
 * the *layout* without the two being computed twice.
 *
 * ## The popup, and why it is trapped
 *
 * A collapsed group is a popup, and popups are where keyboard access dies. This one opens with
 * Enter, Space or Arrow Down, moves focus to its first command, refuses to let focus leave while it
 * is open, and returns focus to the trigger on Escape or on close. The trap is written on
 * `focusout` with the last Tab direction remembered, rather than by enumerating focus stops:
 * enumeration would have to reach through four components' shadow roots to find their inner
 * buttons, and it would be wrong the moment a later child adds a control with two of them.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { splitMenuIcon } from '../controls/control-states.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import {
  defaultGroupPriority,
  essentialCommandLimit,
  essentialSlotName,
  groupPresentationProperty,
  isGroupPriority,
  ribbonEvents,
  ribbonGroupCss,
  type GroupPresentation,
  type GroupPriority,
} from './ribbon-model.ts';

/** The id the trigger's `aria-controls` names. Resolved inside this shadow root, so it works. */
const panelElementId = 'panel';

export class MjxRibbonGroup extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'priority',
    'open',
    'simplified',
  ];

  #root: ShadowRoot | undefined;
  #groupElement: HTMLElement | undefined;
  #trigger: HTMLButtonElement | undefined;
  #panel: HTMLElement | undefined;
  #labelElement: HTMLElement | undefined;
  #panelLabelElement: HTMLElement | undefined;
  #triggerLabelElement: HTMLElement | undefined;
  /** Which way the last Tab went, so the trap wraps to the right end. */
  #backwards = false;
  /** Set on an outside pointer press, so the trap does not fight a deliberate click away. */
  #dismissing = false;
  static readonly #reported = new Set<string>();

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  disconnectedCallback(): void {
    this.#stopWatchingForDismissal();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The group's name — drawn under its commands, and the popup's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /** When this group gives way relative to its neighbours. */
  get priority(): GroupPriority {
    const declared = this.getAttribute('priority');
    return isGroupPriority(declared) ? declared : defaultGroupPriority;
  }

  set priority(value: GroupPriority) {
    this.setAttribute('priority', value);
  }

  /** Whether the collapsed popup is showing. Meaningless in the other two presentations. */
  get open(): boolean {
    return this.hasAttribute('open');
  }

  set open(value: boolean) {
    if (value) this.setAttribute('open', '');
    else this.removeAttribute('open');
  }

  /**
   * **Which presentation CSS put this group in** — read back, never measured.
   *
   * `undefined` before the element has rendered. Anything the browser reports that is not one of
   * the three is treated as `full`, because a group whose custom property has been overwritten by
   * a host is a group whose behaviour should be the least surprising one rather than a popup.
   */
  get presentation(): GroupPresentation {
    const element = this.#groupElement;
    if (element === undefined) return 'full';
    const value = getComputedStyle(element).getPropertyValue(groupPresentationProperty).trim();
    if (value === 'reduced' || value === 'collapsed') return value;
    return 'full';
  }

  /**
   * Every command in the panel, in document order. The essential ones are not among them.
   *
   * `slot=""` counts as the default slot — lit writes an empty attribute where a template has no
   * value, and a command that landed in the panel but not in this list would be a command the
   * focus trap could not reach.
   */
  get panelCommands(): readonly HTMLElement[] {
    return [...this.children].filter((child): child is HTMLElement => {
      if (!(child instanceof HTMLElement)) return false;
      const slot = child.getAttribute('slot');
      return slot === null || slot === '';
    });
  }

  /** The commands that survive a collapse. */
  get essentialCommands(): readonly HTMLElement[] {
    return [...this.children].filter(
      (child): child is HTMLElement =>
        child instanceof HTMLElement && child.getAttribute('slot') === essentialSlotName,
    );
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, ribbonGroupCss);

    const group = document.createElement('div');
    group.className = 'group';
    group.setAttribute('part', 'group');

    const row = document.createElement('div');
    row.className = 'row';
    row.setAttribute('part', 'row');

    const trigger = document.createElement('button');
    trigger.type = 'button';
    trigger.className = `trigger ${typeRoleClass('control')} mjx-hit-target mjx-motion-surface-settle`;
    trigger.setAttribute('part', 'trigger');
    trigger.setAttribute('aria-haspopup', 'true');
    trigger.setAttribute('aria-expanded', 'false');
    trigger.setAttribute('aria-controls', panelElementId);
    trigger.addEventListener('click', this.#onTriggerClick);
    trigger.addEventListener('keydown', this.#onTriggerKeyDown);
    const triggerLabel = document.createElement('span');
    const triggerIcon = document.createElement('mjx-icon');
    triggerIcon.setAttribute('name', splitMenuIcon.name);
    triggerIcon.setAttribute('size', String(splitMenuIcon.size));
    trigger.append(triggerLabel, triggerIcon);

    const essential = document.createElement('div');
    essential.className = 'essential';
    essential.setAttribute('part', 'essential');
    const essentialSlot = document.createElement('slot');
    essentialSlot.name = essentialSlotName;
    essential.append(essentialSlot);

    const panel = document.createElement('div');
    panel.className = 'panel';
    panel.id = panelElementId;
    panel.setAttribute('part', 'panel');
    // The popup announces itself as the group it is. In full and reduced this is simply the group,
    // which is what it is there too.
    panel.setAttribute('role', 'group');

    const commands = document.createElement('div');
    commands.className = 'commands';
    commands.setAttribute('part', 'commands');
    const defaultSlot = document.createElement('slot');
    commands.append(defaultSlot);

    const panelLabel = document.createElement('p');
    panelLabel.className = `panel-label ${typeRoleClass('dense')}`;
    panelLabel.setAttribute('part', 'panel-label');
    panel.append(commands, panelLabel);

    row.append(trigger, essential, panel);

    const footer = document.createElement('div');
    footer.className = 'footer';
    footer.setAttribute('part', 'footer');
    const label = document.createElement('span');
    label.className = `label ${typeRoleClass('dense')}`;
    label.setAttribute('part', 'label');
    const launcherSlot = document.createElement('slot');
    launcherSlot.name = 'dialog-launcher';
    footer.append(label, launcherSlot);

    group.append(row, footer);
    root.append(group);

    this.#groupElement = group;
    this.#trigger = trigger;
    this.#triggerLabelElement = triggerLabel;
    this.#panel = panel;
    this.#labelElement = label;
    this.#panelLabelElement = panelLabel;

    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('focusout', this.#onFocusOut);
    defaultSlot.addEventListener('slotchange', this.#onSlotChange);
  }

  /** Re-render from the attributes. Safe at any time; a no-op before the first build. */
  render(): void {
    const group = this.#groupElement;
    const trigger = this.#trigger;
    const panel = this.#panel;
    if (group === undefined || trigger === undefined || panel === undefined) return;

    group.dataset['priority'] = this.priority;
    // Mirrored onto the box rather than matched with `:host([simplified])`, so that every
    // presentation selector has the same shape and the same (0,0,0) specificity. See
    // `groupSelector` for the bug that made this necessary.
    if (this.hasAttribute('simplified')) group.dataset['simplified'] = '';
    else delete group.dataset['simplified'];

    const label = this.label;
    if (this.#labelElement !== undefined) this.#labelElement.textContent = label;
    if (this.#panelLabelElement !== undefined) this.#panelLabelElement.textContent = label;
    if (this.#triggerLabelElement !== undefined) this.#triggerLabelElement.textContent = label;
    panel.setAttribute('aria-label', label);

    trigger.setAttribute('aria-expanded', this.open ? 'true' : 'false');
    if (this.open) this.#watchForDismissal();
    else this.#stopWatchingForDismissal();

    this.#warnIfOverDemoted();
  }

  // ── the popup ──────────────────────────────────────────────────────────────

  /** Open the collapsed popup and put focus on its first command. */
  openPanel(options: { readonly focus?: boolean } = {}): void {
    if (this.open) return;
    this.open = true;
    this.dispatchEvent(
      new CustomEvent(ribbonEvents.groupToggle, {
        bubbles: true,
        composed: true,
        detail: { open: true },
      }),
    );
    if (options.focus !== false) this.#focusCommand('first');
  }

  /** Close it, and give focus back to the button that opened it. */
  closePanel(options: { readonly focus?: boolean } = {}): void {
    if (!this.open) return;
    this.open = false;
    this.dispatchEvent(
      new CustomEvent(ribbonEvents.groupToggle, {
        bubbles: true,
        composed: true,
        detail: { open: false },
      }),
    );
    if (options.focus !== false) this.#trigger?.focus();
  }

  #onTriggerClick = (): void => {
    if (this.open) this.closePanel();
    else this.openPanel({ focus: false });
  };

  #onTriggerKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== 'ArrowDown') return;
    event.preventDefault();
    this.openPanel();
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === 'Tab') {
      this.#backwards = event.shiftKey;
      return;
    }
    if (event.key !== 'Escape' || !this.open) return;
    // Only the collapsed presentation has anything to close, and only it should swallow Escape —
    // a group that ate Escape while expanded would take it from whatever is above it.
    event.preventDefault();
    event.stopPropagation();
    this.closePanel();
  };

  /**
   * The trap.
   *
   * `relatedTarget` is the element about to receive focus, retargeted to this element's tree — so
   * anything still inside the group reads as a descendant (or as this host, for the trigger in the
   * shadow root) and anything outside does not. Wrapping in a microtask rather than synchronously
   * is required: the browser is mid-way through moving focus and a `focus()` call inside the
   * `focusout` handler is undone by the move that is already in flight.
   */
  #onFocusOut = (event: FocusEvent): void => {
    if (!this.open || this.#dismissing) return;
    const next = event.relatedTarget;
    if (next instanceof Node && next !== this && this.contains(next)) return;
    const direction = this.#backwards ? 'last' : 'first';
    queueMicrotask(() => {
      if (this.open) this.#focusCommand(direction);
    });
  };

  #focusCommand(end: 'first' | 'last'): void {
    const commands = this.panelCommands.filter((command) => !command.hasAttribute('disabled'));
    const target = end === 'first' ? commands[0] : commands[commands.length - 1];
    if (target !== undefined) target.focus();
    else this.#trigger?.focus();
  }

  #onPointerDown = (event: Event): void => {
    const path = event.composedPath();
    if (path.includes(this)) return;
    this.#dismissing = true;
    this.closePanel({ focus: false });
    // The flag is cleared after the focus change the click is about to cause has settled.
    queueMicrotask(() => {
      this.#dismissing = false;
    });
  };

  #watchForDismissal(): void {
    this.ownerDocument.addEventListener('pointerdown', this.#onPointerDown, true);
  }

  #stopWatchingForDismissal(): void {
    this.ownerDocument.removeEventListener('pointerdown', this.#onPointerDown, true);
  }

  #onSlotChange = (): void => {
    this.#warnIfOverDemoted();
  };

  /**
   * The demotion ceiling, reported the way `<mjx-button>` reports a nameless button.
   *
   * `demotionRules` says why a longer survivor row gives back exactly the width the collapse was
   * bought with. The browser gate asserts the same thing over the worst-case story; this is what
   * tells the author who wrote the fifth one, at the moment they wrote it.
   */
  #warnIfOverDemoted(): void {
    const count = this.essentialCommands.length;
    if (count <= essentialCommandLimit) return;
    const key = `${this.label}:${String(count)}`;
    if (MjxRibbonGroup.#reported.has(key)) return;
    MjxRibbonGroup.#reported.add(key);
    console.error(
      `<mjx-ribbon-group label="${this.label}"> declares ${String(count)} essential commands and ` +
        `the ceiling is ${String(essentialCommandLimit)}. A survivor row longer than that is a ` +
        'ribbon again, and the collapse gave back the width it was bought with. See ' +
        'demotionRules in src/ribbon/ribbon-model.ts.',
    );
  }
}

/** Register the element. Idempotent, because a story file and a test may both ask. */
export function defineRibbonGroup(): void {
  defineIcon();
  if (customElements.get('mjx-ribbon-group') === undefined) {
    customElements.define('mjx-ribbon-group', MjxRibbonGroup);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-ribbon-group': MjxRibbonGroup;
  }
}
