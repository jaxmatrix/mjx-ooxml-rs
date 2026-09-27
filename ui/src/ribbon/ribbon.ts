/**
 * `<mjx-ribbon>` — the tab strip, the selected tab's panel, the ribbon's three states, and **the
 * container every group's `@container` query addresses**.
 *
 * ```html
 * <mjx-ribbon label="Word" selected="home" state="expanded">
 *   <mjx-ribbon-tab tab-id="home" label="Home">…</mjx-ribbon-tab>
 *   <mjx-ribbon-tab tab-id="insert" label="Insert">…</mjx-ribbon-tab>
 *   <mjx-contextual-tab-set label="Table Tools">
 *     <mjx-ribbon-tab tab-id="table-design" label="Design">…</mjx-ribbon-tab>
 *   </mjx-contextual-tab-set>
 * </mjx-ribbon>
 * ```
 *
 * ## The host is the query container, and it has no padding
 *
 * `container-type: inline-size` sits on `:host`, so every group inside queries the ribbon's own
 * width rather than the window's or the harness's. The host therefore carries **no padding and no
 * border**, for the reason `<mjx-resizable-container>` learned the hard way: a container query
 * resolves against the *content* box, so 16px of padding here would make every group in the
 * catalogue hit its presentation boundaries 32px later than the ladder says. The breathing room is
 * on `.strip` and `.body` inside the container, where it changes nothing.
 * `tests/browser/ribbon.spec.ts` asserts the container's content width equals the harness frame's.
 *
 * ## The tabs are buttons this element owns
 *
 * The ARIA tabs pattern needs one `role="tablist"` holding sibling `role="tab"` buttons with a
 * single roving tabindex between them, and `<mjx-ribbon-tab>` is a *panel*. So the buttons are
 * built here, from the declarations found in the light DOM, and **reconciled rather than rebuilt**:
 * a tab set appearing inserts new nodes after the existing ones and touches nothing else, which is
 * what makes *"a tab appearing does not steal focus"* true by construction instead of by a
 * `document.activeElement` restore that runs after the damage.
 *
 * Activation is **automatic** — arrowing to a tab selects it — which is what Office's ribbon does
 * and what a person expects from a tab strip whose panels are already in the document.
 *
 * ## Two popups, two behaviours, and the difference is deliberate
 *
 * A collapsed group traps focus (see `ribbon-group.ts`): it is a command surface with many stops
 * and a person who Tabs out of it has lost the group. The **tab picker** does not: it holds one
 * roving tab stop, it is a disclosure, and the standard behaviour for a disclosure is to close when
 * focus leaves it. Both return focus to the button that opened them on Escape.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { splitMenuIcon } from '../controls/control-states.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { MjxContextualTabSet } from './contextual-tab-set.ts';
import { MjxRibbonTab } from './ribbon-tab.ts';
import { MjxRibbonGroup } from './ribbon-group.ts';
import {
  defaultRibbonState,
  isRibbonState,
  ribbonCss,
  ribbonEvents,
  tabIdAttribute,
  tabStripPresentationProperty,
  tabToneAttribute,
  type RibbonState,
  type TabStripPresentation,
} from './ribbon-model.ts';

/** One tab declared in the light DOM, and the contextual set it belongs to. */
interface TabDeclaration {
  readonly panel: MjxRibbonTab;
  readonly set: MjxContextualTabSet | undefined;
}

/**
 * Put `desired` in `parent`, in order, **moving nothing that is already in the right place**.
 *
 * This is the whole reason *"a tab appearing does not steal focus"* holds: a rebuild would detach
 * and re-append the focused tab button, and reparenting a focused element blurs it. A reconcile
 * inserts the new nodes and leaves every existing one exactly where it is.
 */
function reconcile(parent: Element, desired: readonly Node[]): void {
  desired.forEach((node, index) => {
    const current: ChildNode | undefined = parent.childNodes[index];
    if (current !== node) parent.insertBefore(node, current ?? null);
  });
  while (parent.childNodes.length > desired.length) {
    parent.lastChild?.remove();
  }
}

/** The deepest focused element, across shadow boundaries. */
function deepActiveElement(): Element | null {
  let element: Element | null = document.activeElement;
  while (element?.shadowRoot?.activeElement != null) {
    element = element.shadowRoot.activeElement;
  }
  return element;
}

export class MjxRibbon extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'selected',
    'state',
    'simplified',
  ];

  #root: ShadowRoot | undefined;
  #strip: HTMLElement | undefined;
  #tabList: HTMLElement | undefined;
  #picker: HTMLButtonElement | undefined;
  #pickerLabel: HTMLElement | undefined;
  #ribbonToggle: HTMLButtonElement | undefined;
  #restore: HTMLButtonElement | undefined;
  #announcer: HTMLElement | undefined;
  #observer: MutationObserver | undefined;

  /** One button per tab id, so a reconcile can reuse rather than rebuild. */
  readonly #buttons = new Map<string, HTMLButtonElement>();
  /** One wrapper per contextual set label, for the same reason. */
  readonly #setWrappers = new Map<string, { wrapper: HTMLElement; row: HTMLElement }>();
  /** The tab sets that were on screen last time, so an appearance can be announced. */
  #knownSets: readonly string[] = [];
  /** The last **core** tab that was selected — where selection goes when a contextual set closes. */
  #lastCoreSelection: string | undefined;
  #hasFocusInside = false;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.#observer ??= new MutationObserver(() => {
      this.#onStructureChanged();
    });
    this.#observer.observe(this, { childList: true, subtree: true, attributeFilter: ['label'] });
    this.render();
  }

  disconnectedCallback(): void {
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The tablist's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? 'Ribbon';
  }

  /** The selected tab's id. Falls back to the first declared tab. */
  get selected(): string {
    const declared = this.getAttribute('selected');
    const declarations = this.tabs;
    if (declared !== null && declarations.some((entry) => entry.panel.tabId === declared)) {
      return declared;
    }
    return declarations[0]?.panel.tabId ?? '';
  }

  set selected(value: string) {
    this.setAttribute('selected', value);
  }

  /** Expanded, collapsed to its tabs, or out of the way. */
  get state(): RibbonState {
    const declared = this.getAttribute('state');
    return isRibbonState(declared) ? declared : defaultRibbonState;
  }

  set state(value: RibbonState) {
    this.setAttribute('state', value);
  }

  /** Office's single-row ribbon. Every group is at most `reduced`, at every width. */
  get simplified(): boolean {
    return this.hasAttribute('simplified');
  }

  /** Every tab declared in the light DOM, in order, with the set each belongs to. */
  get tabs(): readonly TabDeclaration[] {
    const found: TabDeclaration[] = [];
    for (const child of this.children) {
      if (child instanceof MjxRibbonTab) found.push({ panel: child, set: undefined });
      else if (child instanceof MjxContextualTabSet) {
        for (const grandchild of child.children) {
          if (grandchild instanceof MjxRibbonTab) found.push({ panel: grandchild, set: child });
        }
      }
    }
    return found;
  }

  /** **Which presentation CSS put the tab strip in** — read back, never measured. */
  get tabStripPresentation(): TabStripPresentation {
    const strip = this.#strip;
    if (strip === undefined) return 'strip';
    const value = getComputedStyle(strip).getPropertyValue(tabStripPresentationProperty).trim();
    return value === 'picker' ? 'picker' : 'strip';
  }

  /** Whether the narrow-width tab picker is showing its list. */
  get pickerOpen(): boolean {
    return this.hasAttribute('picker-open');
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, ribbonCss);

    const ribbon = document.createElement('div');
    ribbon.className = 'ribbon';
    ribbon.setAttribute('part', 'ribbon');

    const strip = document.createElement('div');
    strip.className = 'strip';
    strip.setAttribute('part', 'strip');

    const picker = this.#chromeButton('picker');
    picker.setAttribute('aria-haspopup', 'true');
    picker.setAttribute('aria-expanded', 'false');
    const pickerLabel = document.createElement('span');
    const pickerIcon = document.createElement('mjx-icon');
    pickerIcon.setAttribute('name', splitMenuIcon.name);
    pickerIcon.setAttribute('size', String(splitMenuIcon.size));
    picker.append(pickerLabel, pickerIcon);
    picker.addEventListener('click', this.#onPickerClick);

    const tabList = document.createElement('div');
    tabList.className = 'tabs';
    tabList.setAttribute('part', 'tabs');
    tabList.setAttribute('role', 'tablist');
    tabList.addEventListener('keydown', this.#onTabListKeyDown);

    const ribbonToggle = this.#chromeButton('ribbon-toggle');
    ribbonToggle.append(document.createTextNode('Ribbon commands'));
    ribbonToggle.addEventListener('click', this.#onRibbonToggleClick);

    strip.append(picker, tabList, ribbonToggle);

    const body = document.createElement('div');
    body.className = 'body';
    body.setAttribute('part', 'body');
    body.append(document.createElement('slot'));

    const restore = this.#chromeButton('restore');
    restore.append(document.createTextNode('Show the ribbon'));
    restore.addEventListener('click', () => {
      this.#setState('expanded');
    });

    const announcer = document.createElement('div');
    announcer.className = 'announcer visually-hidden';
    announcer.setAttribute('part', 'announcer');
    announcer.setAttribute('role', 'status');
    announcer.setAttribute('aria-live', 'polite');

    ribbon.append(strip, body, restore, announcer);
    root.append(ribbon);

    this.#strip = strip;
    this.#tabList = tabList;
    this.#picker = picker;
    this.#pickerLabel = pickerLabel;
    this.#ribbonToggle = ribbonToggle;
    this.#restore = restore;
    this.#announcer = announcer;

    this.addEventListener('focusin', this.#onFocusIn);
    this.addEventListener('focusout', this.#onFocusOut);
    this.addEventListener('keydown', this.#onKeyDown);
  }

  #chromeButton(className: string): HTMLButtonElement {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `${className} chrome-button ${typeRoleClass('control')} mjx-hit-target mjx-motion-surface-settle`;
    button.setAttribute('part', className);
    return button;
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the attributes and from what is in the light DOM. */
  render(): void {
    const tabList = this.#tabList;
    if (tabList === undefined) return;

    if (!isRibbonState(this.getAttribute('state'))) this.setAttribute('state', defaultRibbonState);

    const declarations = this.tabs;
    const selected = this.selected;

    tabList.setAttribute('aria-label', this.label);
    tabList.setAttribute(
      'aria-orientation',
      this.tabStripPresentation === 'picker' ? 'vertical' : 'horizontal',
    );

    // The strip, reconciled: a contextual set that appeared inserts nodes after the core tabs and
    // moves nothing that already exists.
    const desired: Node[] = [];
    const rowContents = new Map<HTMLElement, Node[]>();
    let currentSet: { set: MjxContextualTabSet; row: HTMLElement } | undefined;
    for (const declaration of declarations) {
      const button = this.#buttonFor(declaration, selected);
      if (declaration.set === undefined) {
        currentSet = undefined;
        desired.push(button);
        continue;
      }
      if (currentSet?.set !== declaration.set) {
        const wrapper = this.#setWrapperFor(declaration.set);
        currentSet = { set: declaration.set, row: wrapper.row };
        desired.push(wrapper.wrapper);
      }
      const row = currentSet.row;
      const contents = rowContents.get(row) ?? [];
      contents.push(button);
      rowContents.set(row, contents);
    }
    for (const { row } of this.#setWrappers.values()) {
      reconcile(row, rowContents.get(row) ?? []);
    }
    reconcile(tabList, desired);
    this.#pruneUnusedNodes(declarations);

    // The panels.
    for (const declaration of declarations) {
      const isSelected = declaration.panel.tabId === selected;
      if (isSelected) declaration.panel.setAttribute('selected', '');
      else declaration.panel.removeAttribute('selected');
      if (declaration.set === undefined && isSelected) this.#lastCoreSelection = declaration.panel.tabId;
    }

    // Simplified is Office's single-row form, and it is the ribbon's decision rather than each
    // group's. A group cannot see the ribbon's attributes from inside its own shadow root, so the
    // ribbon writes the attribute onto the groups it owns.
    for (const group of this.querySelectorAll('mjx-ribbon-group')) {
      if (!(group instanceof MjxRibbonGroup)) continue;
      if (this.simplified) group.setAttribute('simplified', '');
      else group.removeAttribute('simplified');
    }

    const selectedLabel =
      declarations.find((entry) => entry.panel.tabId === selected)?.panel.label ?? '';
    if (this.#pickerLabel !== undefined) this.#pickerLabel.textContent = selectedLabel;
    if (this.#picker !== undefined) {
      this.#picker.setAttribute('aria-expanded', this.pickerOpen ? 'true' : 'false');
      this.#picker.setAttribute('aria-label', `Ribbon tabs: ${selectedLabel}`);
    }
    if (this.#ribbonToggle !== undefined) {
      this.#ribbonToggle.setAttribute('aria-expanded', this.state === 'expanded' ? 'true' : 'false');
    }
    if (this.#restore !== undefined) {
      this.#restore.tabIndex = this.state === 'hidden' ? 0 : -1;
    }
  }

  #buttonFor(declaration: TabDeclaration, selected: string): HTMLButtonElement {
    const id = declaration.panel.tabId;
    let button = this.#buttons.get(id);
    if (button === undefined) {
      button = document.createElement('button');
      button.type = 'button';
      button.className = `tab ${typeRoleClass('control')} mjx-hit-target mjx-motion-surface-settle`;
      button.setAttribute('part', 'tab');
      button.setAttribute('role', 'tab');
      button.setAttribute(tabIdAttribute, id);
      const label = document.createElement('span');
      label.className = 'tab-label';
      const suffix = document.createElement('span');
      suffix.className = 'visually-hidden';
      button.append(label, suffix);
      button.addEventListener('click', () => {
        this.selectTab(id, { focus: false });
      });
      this.#buttons.set(id, button);
    }
    const isSelected = id === selected;
    const label = button.querySelector('.tab-label');
    if (label !== null) label.textContent = declaration.panel.label;
    const suffix = button.querySelector('.visually-hidden');
    // The set's name is part of the tab's accessible name, because the coloured band that says so
    // visually is a picture. "Design, Table Tools" is what a screen reader needs to tell this
    // Design from the Picture Tools one.
    if (suffix !== null) {
      suffix.textContent = declaration.set === undefined ? '' : `, ${declaration.set.label}`;
    }
    button.setAttribute(tabToneAttribute, declaration.set === undefined ? 'core' : 'contextual');
    button.setAttribute('aria-selected', isSelected ? 'true' : 'false');
    button.dataset['pressed'] = isSelected ? 'true' : 'false';
    // The roving tabindex: exactly one stop in the whole tablist, and it is the selected tab.
    button.tabIndex = isSelected ? 0 : -1;
    return button;
  }

  #setWrapperFor(set: MjxContextualTabSet): { wrapper: HTMLElement; row: HTMLElement } {
    const existing = this.#setWrappers.get(set.label);
    if (existing !== undefined) {
      const title = existing.wrapper.querySelector('.tab-set-title');
      if (title !== null) title.textContent = set.label;
      return existing;
    }
    const wrapper = document.createElement('div');
    wrapper.className = 'tab-set';
    wrapper.setAttribute('part', 'tab-set');
    // role="none" keeps the tablist's owned children the tabs themselves: an intermediate element
    // with a presentational role is transparent to `aria-required-children`.
    wrapper.setAttribute('role', 'none');
    const title = document.createElement('p');
    title.className = `tab-set-title ${typeRoleClass('dense')}`;
    title.setAttribute('part', 'tab-set-title');
    // The band is a picture of what the tabs' accessible names already say. Announcing it twice is
    // worse than not drawing it.
    title.setAttribute('aria-hidden', 'true');
    title.textContent = set.label;
    const row = document.createElement('div');
    row.className = 'tab-set-row';
    row.setAttribute('role', 'none');
    wrapper.append(title, row);
    const entry = { wrapper, row };
    this.#setWrappers.set(set.label, entry);
    return entry;
  }

  #pruneUnusedNodes(declarations: readonly TabDeclaration[]): void {
    const liveTabs = new Set(declarations.map((entry) => entry.panel.tabId));
    for (const [id, button] of this.#buttons) {
      if (liveTabs.has(id)) continue;
      button.remove();
      this.#buttons.delete(id);
    }
    const liveSets = new Set(
      declarations
        .map((entry) => entry.set?.label)
        .filter((label): label is string => label !== undefined),
    );
    for (const [label, entry] of this.#setWrappers) {
      if (liveSets.has(label)) continue;
      entry.wrapper.remove();
      this.#setWrappers.delete(label);
    }
  }

  // ── selection ──────────────────────────────────────────────────────────────

  /** Select a tab. `focus` moves the keyboard to it, which arrowing does and clicking does not. */
  selectTab(id: string, options: { readonly focus?: boolean } = {}): void {
    if (this.selected !== id) {
      this.setAttribute('selected', id);
      this.dispatchEvent(
        new CustomEvent(ribbonEvents.tabChange, {
          bubbles: true,
          composed: true,
          detail: { tabId: id },
        }),
      );
    } else {
      this.render();
    }
    if (this.pickerOpen) this.#closePicker({ focus: options.focus !== false });
    else if (options.focus !== false) this.#buttons.get(id)?.focus();
  }

  // ── structure changes ──────────────────────────────────────────────────────

  /**
   * A tab set appeared or disappeared.
   *
   * Three things have to be true here and each is a separate failure:
   *
   * * **An appearance never steals focus.** The reconcile does not touch existing nodes, and
   *   nothing here calls `focus()` on the way in.
   * * **A disappearance moves selection somewhere sane.** Back to the last core tab that was
   *   selected — Word puts you back on Home when the table you had selected goes away — and to the
   *   first tab if there has never been one.
   * * **A disappearance that took the focused element with it puts focus back.** Removing a focused
   *   node leaves the document with nothing focused at all, which is far worse than a moved focus:
   *   the next Tab starts from the top of the page.
   */
  #onStructureChanged(): void {
    const declarations = this.tabs;
    const ids = declarations.map((entry) => entry.panel.tabId);
    const declared = this.getAttribute('selected');
    const lost = declared !== null && !ids.includes(declared);

    if (lost) {
      const fallback =
        this.#lastCoreSelection !== undefined && ids.includes(this.#lastCoreSelection)
          ? this.#lastCoreSelection
          : ids[0];
      if (fallback !== undefined) this.setAttribute('selected', fallback);
    }

    this.render();

    const sets = [
      ...new Set(
        declarations
          .map((entry) => entry.set?.label)
          .filter((label): label is string => label !== undefined),
      ),
    ];
    for (const label of sets) {
      if (!this.#knownSets.includes(label)) this.#announce(`${label} tab set available`);
    }
    for (const label of this.#knownSets) {
      if (!sets.includes(label)) this.#announce(`${label} tab set closed`);
    }
    this.#knownSets = sets;

    // Focus was inside and is now nowhere: whatever held it has been removed.
    if (this.#hasFocusInside) {
      const active = deepActiveElement();
      if (active === null || active === this.ownerDocument.body) {
        this.#buttons.get(this.selected)?.focus();
      }
    }
  }

  #announce(message: string): void {
    if (this.#announcer === undefined) return;
    // Cleared first: a live region whose text is replaced with the same string announces nothing,
    // and two tab sets appearing in one turn would otherwise announce once.
    this.#announcer.textContent = '';
    this.#announcer.textContent = message;
  }

  // ── keyboard ───────────────────────────────────────────────────────────────

  #onTabListKeyDown = (event: KeyboardEvent): void => {
    const vertical = this.tabStripPresentation === 'picker';
    const ids = this.tabs.map((entry) => entry.panel.tabId);
    if (ids.length === 0) return;
    const index = Math.max(0, ids.indexOf(this.selected));

    let next: number | undefined;
    if (event.key === 'ArrowRight' || (vertical && event.key === 'ArrowDown')) {
      next = (index + 1) % ids.length;
    } else if (event.key === 'ArrowLeft' || (vertical && event.key === 'ArrowUp')) {
      next = (index - 1 + ids.length) % ids.length;
    } else if (event.key === 'Home') {
      next = 0;
    } else if (event.key === 'End') {
      next = ids.length - 1;
    }
    if (next === undefined) return;

    event.preventDefault();
    const id = ids[next];
    if (id === undefined) return;
    // Automatic activation: arrowing selects. The panels are already in the document, so there is
    // nothing expensive about it, and it is what a ribbon does.
    if (this.selected !== id) {
      this.setAttribute('selected', id);
      this.dispatchEvent(
        new CustomEvent(ribbonEvents.tabChange, {
          bubbles: true,
          composed: true,
          detail: { tabId: id },
        }),
      );
    } else {
      this.render();
    }
    this.#buttons.get(id)?.focus();
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== 'Escape' || !this.pickerOpen) return;
    event.preventDefault();
    event.stopPropagation();
    this.#closePicker({ focus: true });
  };

  #onPickerClick = (): void => {
    if (this.pickerOpen) this.#closePicker({ focus: true });
    else this.#openPicker();
  };

  #openPicker(): void {
    this.setAttribute('picker-open', '');
    this.render();
    this.#buttons.get(this.selected)?.focus();
  }

  #closePicker(options: { readonly focus?: boolean } = {}): void {
    if (!this.pickerOpen) return;
    this.removeAttribute('picker-open');
    this.render();
    if (options.focus === true) this.#picker?.focus();
  }

  #onRibbonToggleClick = (): void => {
    this.#setState(this.state === 'expanded' ? 'tabs' : 'expanded');
  };

  #setState(state: RibbonState): void {
    this.setAttribute('state', state);
    this.dispatchEvent(
      new CustomEvent(ribbonEvents.stateChange, {
        bubbles: true,
        composed: true,
        detail: { state },
      }),
    );
    if (state === 'expanded') this.#ribbonToggle?.focus();
  }

  #onFocusIn = (): void => {
    this.#hasFocusInside = true;
  };

  #onFocusOut = (event: FocusEvent): void => {
    const next = event.relatedTarget;
    if (next instanceof Node && next !== this && !this.contains(next)) {
      this.#hasFocusInside = false;
      // A disclosure closes when focus leaves it. The collapsed group traps instead, and
      // `ribbon-group.ts` says why the two differ.
      if (this.pickerOpen) this.#closePicker();
    }
  };
}

/** Register the element. Idempotent. */
export function defineRibbon(): void {
  defineIcon();
  if (customElements.get('mjx-ribbon') === undefined) customElements.define('mjx-ribbon', MjxRibbon);
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-ribbon': MjxRibbon;
  }
}
