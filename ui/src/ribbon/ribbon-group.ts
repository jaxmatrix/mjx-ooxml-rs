/**
 * `<mjx-ribbon-group>` — the labelled container 1,193 of Office's published groups are, and **the
 * hardest responsive problem in the catalogue**.
 *
 * ```html
 * <mjx-ribbon-group label="Font" priority="primary">
 *   <mjx-button label="Increase Font Size" icon="font-increase" size="icon"></mjx-button>
 *   <mjx-toggle-button slot="essential" label="Bold" icon="text-bold" size="icon"></mjx-toggle-button>
 *   <mjx-toggle-button label="Strikethrough" icon="text-strikethrough" size="icon"></mjx-toggle-button>
 *   <mjx-dialog-launcher slot="dialog-launcher" label="Font settings"></mjx-dialog-launcher>
 * </mjx-ribbon-group>
 * ```
 *
 * **Order the children the way Office draws them, and mark the survivors where they stand.** The
 * group above draws Increase Font Size, Bold, Strikethrough in that order at every width with room
 * for all three; collapsed, Bold stays beside the trigger and the other two go behind it.
 * `slot="essential"` is a *declaration* — *this command survives a collapse* — and says nothing
 * about position.
 *
 * ## Two slots, one order — which is why nothing can be lost and nothing moves
 *
 * The obvious way to build a collapsing group is to render the commands twice: once in the strip
 * and once in a menu. That is also the way to lose one, and MJXOFF-183 says exactly what is at
 * stake: *"a control that disappears at narrow width has not degraded, it has been lost. That
 * reachability assertion is the one that matters."*
 *
 * So this element renders every command **once**. Its shadow root uses **manual slot assignment**
 * and has two command slots: the ordered one inside `.panel`, and the survivor row beside the
 * trigger. In `full` and `reduced` every command is assigned to the panel, in declared order, and
 * the survivor row is empty — so an essential command draws, and takes focus, exactly where its
 * author put it. In `collapsed` the essential commands are assigned to the survivor row and the rest
 * stay in the panel, which is now an overlay anchored to the trigger. The commands are the same DOM
 * nodes at every width — a command is *moved between slots*, never rebuilt — so *reachable in all
 * three* is structural rather than something this component has to remember, and
 * `tests/browser/ribbon.spec.ts` still counts them at all three, because a structural guarantee
 * nobody has watched hold is a guarantee about a file.
 *
 * `survivorPlacement` in `ribbon-model.ts` is the decision this is built on, with the four designs
 * it rejected and why; `placeGroupCommands` is the rule, and this file calls it rather than restating
 * it.
 *
 * ## The layout is CSS's decision; this file only reads it back
 *
 * No width is measured here. `src/ribbon/ribbon-model.ts` generates eight `@container` blocks from
 * the priority ladder, each of which writes `--mjx-group-presentation`, and `presentation` below is a
 * single `getComputedStyle` read of that custom property. That is what lets the *behaviour* (a popup
 * traps focus; a panel does not; the survivors have a row) follow the *layout* without the two
 * being computed twice.
 *
 * What CSS cannot do is say *when* its answer changed, and slot assignment has to follow it. So
 * there is **one** `ResizeObserver`, shared by every group on the page, and it observes a probe
 * rather than anything with content: a zero-height box inside each group's shadow root whose width
 * is `100cqi` — the width of the nearest inline-size query container, which inside a ribbon is
 * `<mjx-ribbon>`, and nothing else. It therefore fires exactly when a container condition can have
 * changed, and when a group that was inside a hidden tab becomes rendered; its callback ignores every
 * size it is handed, reads every group's presentation, and only then re-slots, so a page of forty
 * groups costs one style read rather than forty interleaved with forty writes. The probe, not the
 * group, is observed because re-slotting changes the group's size inside the callback, which would
 * be a ResizeObserver loop error.
 *
 * Two other paths re-slot, and neither measures anything either: a `MutationObserver` on the light
 * DOM, because a manually assigned slot is never told about a new child or a changed `slot`, and a
 * change to the group's own `priority` or `simplified` attribute, which changes CSS's answer.
 *
 * `<mjx-gallery>` answers a different question and observes differently: it watches **itself, its
 * parent group and its clipping ancestor**, with no probe, because it has a column count and a
 * flyout position to recompute from real sizes rather than a presentation to read back.
 *
 * ## The popup, and why it is trapped
 *
 * A collapsed group is a popup, and popups are where keyboard access dies. This one opens with
 * Enter, Space or Arrow Down, moves focus to its first command, refuses to let focus leave while it
 * is open, and returns focus to the trigger on Escape or on close. The trap is written on
 * `focusout` with the last Tab direction remembered, rather than by enumerating focus stops:
 * enumeration would have to reach through four components' shadow roots to find their inner
 * buttons, and it would be wrong the moment a later child adds a control with two of them.
 *
 * ⚠ **A popup that stops being a popup closes.** If the container widens while the popup is open,
 * the group is no longer collapsed: its commands are back in the strip, its trigger is not drawn,
 * and there is nothing to close. Left `open`, it would keep the dismissal listener and — worse —
 * the trap, so Tab out of Font at a desktop width would be pulled back into Font. So the moment the
 * group re-slots for a presentation whose panel is not a popup, it closes itself **without moving
 * focus**: a person who was inside the popup is now on the same command, drawn in the strip, and
 * nothing should take the keyboard off it. `tests/browser/ribbon.spec.ts` widens an open Font group
 * and asserts both halves.
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
  groupPresentations,
  isGroupPriority,
  placeGroupCommands,
  ribbonEvents,
  ribbonGroupCss,
  type GroupPresentation,
  type GroupPriority,
} from './ribbon-model.ts';

/** The id the trigger's `aria-controls` names. Resolved inside this shadow root, so it works. */
const panelElementId = 'panel';

/** The slot a dialog launcher declares itself into. */
const launcherSlotName = 'dialog-launcher';

/** The attributes that change which presentation CSS chooses, and therefore where commands sit. */
const placementAttributes: readonly string[] = ['priority', 'simplified'];

/** Whether a child declared itself one of the group's commands — the panel's or a survivor. */
function isCommand(child: Element): child is HTMLElement {
  if (!(child instanceof HTMLElement)) return false;
  const slot = child.getAttribute('slot');
  // `slot=""` counts as the panel — lit writes an empty attribute where a template has no value.
  return slot === null || slot === '' || slot === essentialSlotName;
}

function isEssential(command: HTMLElement): boolean {
  return command.getAttribute('slot') === essentialSlotName;
}

/** Assign `nodes` to `slot` unless it already holds exactly those, in that order. */
function assignIfChanged(slot: HTMLSlotElement, nodes: readonly HTMLElement[]): void {
  const current = slot.assignedElements();
  if (current.length === nodes.length && current.every((node, index) => node === nodes[index])) {
    return;
  }
  slot.assign(...nodes);
}

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
  #survivorSlot: HTMLSlotElement | undefined;
  #panelSlot: HTMLSlotElement | undefined;
  #launcherSlot: HTMLSlotElement | undefined;
  #probe: HTMLElement | undefined;
  #labelElement: HTMLElement | undefined;
  #panelLabelElement: HTMLElement | undefined;
  #triggerLabelElement: HTMLElement | undefined;
  /** Watches the light DOM, because a manually assigned slot is never told about a new child. */
  #childWatcher: MutationObserver | undefined;
  /** The presentation the slots were last filled for. `full` until the probe first reports. */
  #placedFor: GroupPresentation = 'full';
  /** Which way the last Tab went, so the trap wraps to the right end. */
  #backwards = false;
  /** Set on an outside pointer press, so the trap does not fight a deliberate click away. */
  #dismissing = false;
  static readonly #reported = new Set<string>();
  /** The one observer every group's probe shares. Created on first connection. */
  static #probeObserver: ResizeObserver | undefined;
  static readonly #groupsByProbe = new WeakMap<Element, MjxRibbonGroup>();

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.#childWatcher?.observe(this, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['slot'],
    });
    // No style is read here: forty groups connecting in one render would otherwise interleave forty
    // forced layouts with forty re-slots. The last known placement is applied, and the shared
    // observer's first report — which arrives before the first paint — settles every group at once.
    this.#fill(this.#placedFor);
    this.render();
    MjxRibbonGroup.#watchProbe(this);
  }

  disconnectedCallback(): void {
    this.#stopWatchingForDismissal();
    this.#childWatcher?.disconnect();
    MjxRibbonGroup.#unwatchProbe(this);
  }

  attributeChangedCallback(name: string): void {
    if (this.#root === undefined) return;
    this.render();
    if (this.isConnected && placementAttributes.includes(name)) this.#settle(this.presentation);
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
   * `full` before the element has rendered. Anything the browser reports that is not one of the
   * three is treated as `full`, because a group whose custom property has been overwritten by a host
   * is a group whose behaviour should be the least surprising one rather than a popup.
   */
  get presentation(): GroupPresentation {
    const element = this.#groupElement;
    if (element === undefined) return 'full';
    const value = getComputedStyle(element).getPropertyValue(groupPresentationProperty).trim();
    if (value === 'reduced' || value === 'collapsed') return value;
    return 'full';
  }

  /** Every command the group holds, essential or not, in the order its author declared them. */
  get commands(): readonly HTMLElement[] {
    return [...this.children].filter(isCommand);
  }

  /**
   * The commands that go behind the trigger when the group collapses, in declared order. The
   * essential ones are not among them.
   *
   * This is what the popup holds, and so what the focus trap cycles through. `slot=""` counts —
   * lit writes an empty attribute where a template has no value, and a command that landed in the
   * popup but not in this list would be a command the trap could not reach.
   */
  get panelCommands(): readonly HTMLElement[] {
    return this.commands.filter((command) => !isEssential(command));
  }

  /** The commands that survive a collapse, in declared order. */
  get essentialCommands(): readonly HTMLElement[] {
    return this.commands.filter(isEssential);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open', slotAssignment: 'manual' });
    this.#root = root;
    installControlStyles(root, ribbonGroupCss);

    const group = document.createElement('div');
    group.className = 'group';
    group.setAttribute('part', 'group');

    const probe = document.createElement('div');
    probe.className = 'presentation-probe';
    const probeWidth = document.createElement('div');
    probeWidth.className = 'presentation-probe-width';
    probe.append(probeWidth);

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

    // The survivor row. Filled only while the group is collapsed; see `#fill`.
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
    const panelSlot = document.createElement('slot');
    commands.append(panelSlot);

    const panelLabel = document.createElement('p');
    panelLabel.className = `panel-label ${typeRoleClass('dense')}`;
    panelLabel.setAttribute('part', 'panel-label');
    panel.append(commands, panelLabel);

    // ⚠ **This order is the focus order in the collapsed presentation, and it is only safe because
    // the survivor row is empty in the other two.** A keyboard reaches the trigger, then the
    // survivors, then the popup — which is also what a collapsed group looks like. In full and
    // reduced every command is in the panel's slot, in declared order, so the row's order has no
    // say at all. `d01cf93` tried to fix a misplaced survivor by moving this row; the fix was the
    // slot assignment, not the row.
    row.append(trigger, essential, panel);

    const footer = document.createElement('div');
    footer.className = 'footer';
    footer.setAttribute('part', 'footer');
    const label = document.createElement('span');
    label.className = `label ${typeRoleClass('dense')}`;
    label.setAttribute('part', 'label');
    const launcherSlot = document.createElement('slot');
    launcherSlot.name = launcherSlotName;
    footer.append(label, launcherSlot);

    group.append(probe, row, footer);
    root.append(group);

    this.#groupElement = group;
    this.#trigger = trigger;
    this.#triggerLabelElement = triggerLabel;
    this.#panel = panel;
    this.#survivorSlot = essentialSlot;
    this.#panelSlot = panelSlot;
    this.#launcherSlot = launcherSlot;
    this.#probe = probeWidth;
    this.#labelElement = label;
    this.#panelLabelElement = panelLabel;
    this.#childWatcher = new MutationObserver(this.#onChildrenChanged);

    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('focusout', this.#onFocusOut);
    essentialSlot.addEventListener('slotchange', this.#onSlotChange);
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

  // ── where each command sits ────────────────────────────────────────────────

  /**
   * Assign every child to its slot for one presentation.
   *
   * `placeGroupCommands` is the rule; this only hands it the declared commands and writes the answer
   * into the two slots. A slot that already holds exactly the right nodes is left alone, so a report
   * that changed nothing costs no layout.
   */
  #fill(presentation: GroupPresentation): void {
    const survivorSlot = this.#survivorSlot;
    const panelSlot = this.#panelSlot;
    const launcherSlot = this.#launcherSlot;
    if (survivorSlot === undefined || panelSlot === undefined || launcherSlot === undefined) return;

    const placement = placeGroupCommands(presentation, this.commands, isEssential);
    assignIfChanged(survivorSlot, placement.survivors);
    assignIfChanged(panelSlot, placement.panel);
    assignIfChanged(
      launcherSlot,
      [...this.children].filter(
        (child): child is HTMLElement =>
          child instanceof HTMLElement && child.getAttribute('slot') === launcherSlotName,
      ),
    );
    this.#placedFor = presentation;
  }

  /**
   * Fill the slots for a presentation CSS has **actually reported**, and close a popup that no
   * longer is one — see the module note.
   *
   * Separate from `#fill` because `connectedCallback` fills from the last known placement before
   * CSS has answered, and closing there would shut a group declared `open` in markup before its
   * container had been measured once. Every caller of this one has just read `presentation`.
   */
  #settle(presentation: GroupPresentation): void {
    this.#fill(presentation);
    if (this.open && !groupPresentations[presentation].panelIsPopup) {
      this.closePanel({ focus: false });
    }
  }

  /**
   * A child arrived, left, or changed its `slot`.
   *
   * The watcher observes the subtree so that a command's own `slot` attribute is seen, which also
   * delivers every change *inside* a command — an option added to a dropdown, a gallery's items. Only
   * a record about this element's own children can move anything, so the rest are ignored before
   * any style is read.
   */
  #onChildrenChanged = (records: readonly MutationRecord[]): void => {
    const relevant = records.some((record) =>
      record.type === 'childList' ? record.target === this : record.target.parentNode === this,
    );
    if (relevant && this.isConnected) this.#settle(this.presentation);
  };

  static #watchProbe(group: MjxRibbonGroup): void {
    const probe = group.#probe;
    if (probe === undefined) return;
    if (typeof ResizeObserver === 'undefined') {
      // No observer means no report will ever arrive, so ask once now. Every engine this catalogue
      // targets has one; this is the difference between a degraded group and a wrong one.
      group.#settle(group.presentation);
      return;
    }
    MjxRibbonGroup.#probeObserver ??= new ResizeObserver(MjxRibbonGroup.#onProbeReports);
    MjxRibbonGroup.#groupsByProbe.set(probe, group);
    MjxRibbonGroup.#probeObserver.observe(probe);
  }

  static #unwatchProbe(group: MjxRibbonGroup): void {
    const probe = group.#probe;
    if (probe === undefined) return;
    MjxRibbonGroup.#probeObserver?.unobserve(probe);
    MjxRibbonGroup.#groupsByProbe.delete(probe);
  }

  /**
   * The container's width changed, or a group became rendered. **Every read, then every write.**
   *
   * The sizes in the entries are deliberately ignored: which presentation a width means is CSS's
   * decision, and asking CSS is the whole of the answer.
   */
  static readonly #onProbeReports = (entries: readonly ResizeObserverEntry[]): void => {
    const readings: { readonly group: MjxRibbonGroup; readonly presentation: GroupPresentation }[] =
      [];
    for (const entry of entries) {
      const group = MjxRibbonGroup.#groupsByProbe.get(entry.target);
      if (group === undefined || !group.isConnected) continue;
      readings.push({ group, presentation: group.presentation });
    }
    for (const reading of readings) reading.group.#settle(reading.presentation);
  };

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
   * tells the author who wrote the fourth one, at the moment they wrote it. Counted over the
   * *declaration* rather than the survivor slot, so it speaks at every width rather than only once
   * the group has collapsed.
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
