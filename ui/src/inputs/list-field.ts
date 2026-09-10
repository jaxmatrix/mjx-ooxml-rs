/**
 * What `<mjx-dropdown>` and `<mjx-combo-box>` are both made of.
 *
 * The two differ in **one** thing — whether the field is a text box a person types into — and that
 * one difference changes the ARIA (`aria-autocomplete`), the key map (Home and End are caret keys
 * in a text box), and what committing means. Everything else is shared, and it is shared as one
 * implementation rather than two that look alike, because the invariant this file exists to keep
 * cannot be kept twice:
 *
 * > **the value the field shows must be the value the control reports.**
 *
 * `#syncText()` is the only place the field's text is written and `displayTextFor()` in the model
 * is the only function that decides it. `tests/browser/inputs.spec.ts` asserts the two agree after
 * every path — type-and-commit, arrow-and-commit, Escape while open, Escape while closed, Tab,
 * blur, outside click, and an unmatched string with and without `allow-custom`.
 */

import { defineIcon } from '../icons/icon.ts';
import {
  applyAvailability,
  createExplanationElement,
  explanationElementId,
  installControlStyles,
  isHardDisabled,
  isUnavailable,
  refusesActivation,
} from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import { floatingCss, installFloatingProperties, type Direction } from '../overlay/floating.ts';
import {
  displayTextFor,
  fieldSheet,
  inputEvents,
  inputTags,
  inputTypeClass,
  isForcibleFieldState,
  listSheet,
  listboxKeyAction,
  nextOptionIndex,
  optionByValue,
  typeaheadIndex,
  typeaheadResetDelay,
  forcedFieldStateAttribute,
  type ListCloseReason,
  type ListboxAction,
  type OptionDescriptor,
} from './input-model.ts';
import { ListSurface, type ListSurfaceHost, type PopupSurface } from './list-surface.ts';
import { optionDescriptorsIn, optionsChangedEvent } from './descriptors.ts';

/** The sheet every list field adopts. */
export const listFieldCss = [fieldSheet, listSheet].join('\n');

/** The glyph a list field draws to say *there is a list under here*. */
export const listDisclosureIcon = { name: 'chevron-down', size: 16 } as const;

let sequence = 0;

export abstract class MjxListField extends HTMLElement implements ListSurfaceHost {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'placeholder',
    'disabled',
    'unavailable',
    'explanation',
    forcedFieldStateAttribute,
  ];

  #root: ShadowRoot | undefined;
  #field: HTMLElement | undefined;
  #entry: HTMLElement | undefined;
  #disclosure: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;
  #surface: PopupSurface | undefined;
  #declaredOptions: readonly OptionDescriptor[] | undefined;
  #lightOptions: readonly OptionDescriptor[] = [];
  #watching = false;
  #dismissing = false;
  #typeahead = '';
  #typeaheadTimer: ReturnType<typeof setTimeout> | undefined;
  readonly #id = `mjx-list-${String((sequence += 1))}`;

  // ── the two things a subclass decides ──────────────────────────────────────

  /** Whether the field is a text box. Decides the ARIA, the key map and what committing means. */
  protected abstract get editable(): boolean;

  /** Build the focusable element the field is made of. */
  protected abstract createEntry(): HTMLElement;

  /** What the entry currently shows. */
  protected abstract entryText(): string;

  /** Write what the entry shows. */
  protected abstract setEntryText(text: string): void;

  /**
   * Build the thing that pops up under the field.
   *
   * A third decision, added by MJXOFF-187, and it is the same *kind* of decision as the other two:
   * a colour picker is this field with a **grid** under it instead of a list, and everything else
   * — the ARIA combobox contract, the single tab stop, the dismissal model, the placement, the
   * commit ordering — is identical and had better stay identical. Overriding this is how a
   * subclass says what pops up; a subclass that reimplemented the field around its own popup would
   * be a second answer to all five of those.
   */
  protected createSurface(idPrefix: string): PopupSurface {
    return new ListSurface(this, idPrefix);
  }

  /**
   * A hook for a subclass that needs to decorate the rows its surface builds.
   *
   * A no-op here, and it is declared rather than left off so `ListSurfaceHost` is satisfied by
   * this class rather than by each subclass separately.
   */
  decorateOption(_row: HTMLElement, _option: OptionDescriptor, _index: number): void {
    // Nothing.
  }

  // ── lifecycle ──────────────────────────────────────────────────────────────

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.#readLightOptions();
    this.render();
  }

  disconnectedCallback(): void {
    this.#stopWatching();
    this.#surface?.dispose();
    if (this.#typeaheadTimer !== undefined) clearTimeout(this.#typeaheadTimer);
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  // ── the surface ────────────────────────────────────────────────────────────

  /** The accessible name. `<mjx-label for>` pushes this in when the author did not write one. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /** Shown when the control reports no value. Secondary text on the field's own fill. */
  get placeholder(): string {
    return this.getAttribute('placeholder') ?? '';
  }

  /** The committed value. Empty when nothing is chosen. */
  get value(): string {
    return this.getAttribute('value') ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** Whether a value the option list does not carry may be committed. */
  get allowCustom(): boolean {
    return this.hasAttribute('allow-custom');
  }

  /**
   * The options, as data.
   *
   * Set the property for a list that comes from somewhere — every installed font — and write
   * `<mjx-option>` children for one an author types out. The property wins when both are present,
   * because a host that has assigned a list has said what the list is.
   */
  get options(): readonly OptionDescriptor[] {
    return this.#declaredOptions ?? this.#lightOptions;
  }

  set options(next: readonly OptionDescriptor[]) {
    this.#declaredOptions = [...next];
    this.render();
  }

  /** The options the list is showing. A dropdown shows all of them; a combo box filters. */
  get visibleOptions(): readonly OptionDescriptor[] {
    return this.options;
  }

  /** Whether the list is showing. */
  get open(): boolean {
    return this.#surface?.open === true;
  }

  /**
   * The list surface — **public, because a gate has to read it.**
   *
   * `tests/browser/inputs.spec.ts` compares the number of rows the surface built against
   * `cellsInWindow` and re-runs `placeFloating` in Node over the anchor, boundary and natural size
   * it recorded. A gate that could only see the rendered result would be asserting that the list
   * is somewhere plausible; with the inputs it asserts that the component and the model agree,
   * which is the only comparison that can catch a placement wrong in both places at once.
   */
  get surface(): PopupSurface | undefined {
    return this.#surface;
  }

  /** The field box, for a gate that measures its paint. */
  get fieldElement(): HTMLElement | undefined {
    return this.#field;
  }

  /** The focusable element. */
  get entryElement(): HTMLElement | undefined {
    return this.#entry;
  }

  /** The writing direction this field resolves its placement against. */
  get direction(): Direction {
    return getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  /** Focus the control this element *is*. The host carries no tabindex. */
  override focus(options?: FocusOptions): void {
    if (this.#entry === undefined) super.focus(options);
    else this.#entry.focus(options);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, [floatingCss, listFieldCss].join('\n'));

    const field = document.createElement('div');
    field.className = 'field mjx-hit-target mjx-motion-surface-settle';
    field.setAttribute('part', 'field');

    const entry = this.createEntry();
    entry.classList.add('entry', inputTypeClass('fieldValue'));
    entry.setAttribute('part', 'entry');
    entry.id = `${this.#id}-entry`;
    entry.setAttribute('role', 'combobox');
    entry.setAttribute('aria-haspopup', 'listbox');
    entry.addEventListener('keydown', this.#onKeyDown);
    entry.addEventListener('focusout', this.#onFocusOut);

    const disclosure = document.createElement('mjx-icon');
    disclosure.setAttribute('name', listDisclosureIcon.name);
    disclosure.setAttribute('size', String(listDisclosureIcon.size));
    disclosure.setAttribute('aria-hidden', 'true');
    disclosure.classList.add('trailing');

    const surface = this.createSurface(this.#id);
    surface.element.id = `${this.#id}-list`;
    entry.setAttribute('aria-controls', surface.element.id);

    const explanation = createExplanationElement(explanationElementId);

    field.append(entry, disclosure);
    field.addEventListener('pointerdown', this.#onFieldPointerDown);
    root.append(field, surface.element, explanation);

    this.#field = field;
    this.#entry = entry;
    this.#disclosure = disclosure;
    this.#surface = surface;
    this.#explanationElement = explanation;

    // `<mjx-option>` children are descriptors, not content: they are read for their attributes and
    // never slotted. A slot is still attached so `slotchange` fires — which is how the list learns
    // that a child arrived after the host upgraded, U06's second defect in its own costume.
    const slot = document.createElement('slot');
    slot.hidden = true;
    slot.addEventListener('slotchange', this.#onSlotChange);
    root.append(slot);
    this.addEventListener(optionsChangedEvent, this.#onSlotChange);
  }

  #onSlotChange = (): void => {
    this.#readLightOptions();
    this.render();
  };

  #readLightOptions(): void {
    this.#lightOptions = optionDescriptorsIn(this, inputTags.option);
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the attributes. */
  render(): void {
    const field = this.#field;
    const entry = this.#entry;
    const surface = this.#surface;
    const explanation = this.#explanationElement;
    if (field === undefined || entry === undefined || surface === undefined || explanation === undefined) {
      return;
    }

    entry.setAttribute('aria-label', this.label);
    entry.setAttribute('aria-expanded', String(this.open));
    if (this.editable) entry.setAttribute('aria-autocomplete', 'list');

    const disabled = isHardDisabled(this);
    const unavailable = !disabled && isUnavailable(this);
    if (disabled) field.dataset['disabled'] = '';
    else delete field.dataset['disabled'];
    if (unavailable) field.dataset['unavailable'] = '';
    else delete field.dataset['unavailable'];
    if (this.open) field.dataset['editing'] = '';
    else delete field.dataset['editing'];

    const forced = this.getAttribute(forcedFieldStateAttribute);
    for (const state of ['hover', 'editing', 'invalid']) delete field.dataset[state === 'hover' ? 'state' : state];
    if (isForcibleFieldState(forced)) {
      if (forced === 'hover') field.dataset['state'] = 'hover';
      else field.dataset[forced] = '';
    }

    applyAvailability(this, entry, explanation);

    surface.setOptions(this.visibleOptions);
    surface.setSelected(this.value === '' ? undefined : this.value);
    const active = surface.activeDescendantId;
    if (this.open && active !== undefined) entry.setAttribute('aria-activedescendant', active);
    else entry.removeAttribute('aria-activedescendant');

    this.syncText();
    this.rendered(field, entry);
  }

  /** A hook for a subclass, at the end of every render. */
  protected rendered(_field: HTMLElement, _entry: HTMLElement): void {
    // Nothing.
  }

  /**
   * **The one place the field's text is written.**
   *
   * A subclass overrides it only to add a condition — the combo box does not overwrite what a
   * person is in the middle of typing — and never to compute the text a different way.
   */
  protected syncText(): void {
    this.setEntryText(this.displayText());
  }

  /** What the committed value is displayed as. One function, read by the component and the gate. */
  displayText(): string {
    return displayTextFor(this.options, this.value, this.allowCustom);
  }

  // ── opening and closing ────────────────────────────────────────────────────

  /** Show the list, with the keyboard on the chosen option or on the first one. */
  openList(): void {
    const surface = this.#surface;
    const field = this.#field;
    if (surface === undefined || field === undefined || refusesActivation(this)) return;
    if (surface.open) return;
    this.opening();
    surface.setOptions(this.visibleOptions);
    surface.setSelected(this.value === '' ? undefined : this.value);
    surface.show(field, this.direction);
    surface.setActive(this.initialActiveIndex());
    this.#startWatching();
    this.render();
    this.dispatchEvent(
      new CustomEvent(inputEvents.toggle, { bubbles: true, composed: true, detail: { open: true } }),
    );
  }

  /** A hook a subclass uses to record what it will have to restore. */
  protected opening(): void {
    // Nothing.
  }

  /** Which option the keyboard lands on when the list opens. The chosen one, or the first. */
  protected initialActiveIndex(): number {
    const options = this.visibleOptions;
    const chosen = options.findIndex((option) => option.value === this.value);
    return chosen >= 0 ? chosen : options.length > 0 ? 0 : -1;
  }

  /**
   * Hide the list.
   *
   * `focusRestoringListCloseReasons` says which reasons put focus back on the field. `blur` does
   * not, because focus has already gone somewhere a person chose.
   */
  closeList(reason: ListCloseReason, options: { restoreFocus?: boolean } = {}): void {
    const surface = this.#surface;
    if (surface === undefined || !surface.open) return;
    surface.hide();
    this.#stopWatching();
    this.#typeahead = '';
    this.closed(reason);
    this.render();
    this.dispatchEvent(
      new CustomEvent(inputEvents.toggle, {
        bubbles: true,
        composed: true,
        detail: { open: false, reason },
      }),
    );
    const restore = options.restoreFocus ?? (reason !== 'blur' && reason !== 'tab');
    if (restore) this.focus();
  }

  /** A hook a subclass uses to restore what it recorded. */
  protected closed(_reason: ListCloseReason): void {
    // Nothing.
  }

  // ── committing ─────────────────────────────────────────────────────────────

  /**
   * Report a new value, and only if it is new.
   *
   * The attribute moves before the event fires, so a listener reading `event.target.value` gets
   * the value the event is about — the ordering `<mjx-toggle-button>` states and every control in
   * this catalogue keeps.
   */
  protected commitValue(next: string): void {
    const previous = this.value;
    if (next === previous) {
      this.syncText();
      return;
    }
    this.setAttribute('value', next);
    this.syncText();
    this.dispatchEvent(
      new CustomEvent(inputEvents.change, {
        bubbles: true,
        composed: true,
        detail: { value: next, previous },
      }),
    );
  }

  /** Say a value is being tried without committing it. What a live preview listens to. */
  protected previewValue(value: string): void {
    this.dispatchEvent(
      new CustomEvent(inputEvents.preview, { bubbles: true, composed: true, detail: { value } }),
    );
  }

  /** Choose the option the keyboard is on, if there is one and it may be chosen. */
  protected commitActive(): boolean {
    const surface = this.#surface;
    const option = surface?.activeOption;
    if (option === undefined || option.unavailable === true) return false;
    this.commitValue(option.value);
    return true;
  }

  // ── the list surface's two callbacks ───────────────────────────────────────

  chose(option: OptionDescriptor, _index: number): void {
    this.commitValue(option.value);
    this.closeList('commit');
  }

  movedActive(index: number): void {
    const entry = this.#entry;
    const surface = this.#surface;
    if (entry === undefined || surface === undefined) return;
    const active = surface.activeDescendantId;
    if (active === undefined) entry.removeAttribute('aria-activedescendant');
    else entry.setAttribute('aria-activedescendant', active);
    const option = surface.options[index];
    if (option !== undefined) this.previewValue(option.value);
  }

  // ── the keyboard ───────────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    if (refusesActivation(this)) return;
    // A modified key press belongs to the platform: Ctrl+A selects, Alt+Tab leaves, and a list
    // that swallowed either would be a list a person cannot get out of.
    if (event.ctrlKey || event.metaKey || event.altKey) return;

    // A subclass's first look, before the listbox map sees the key at all.
    if (this.interceptKey(event) === 'handled') {
      event.preventDefault();
      event.stopPropagation();
      return;
    }

    const action = listboxKeyAction(event.key, { open: this.open, editable: this.editable });
    if (action === undefined) return;
    if (this.handleAction(action, event) === 'handled') {
      event.preventDefault();
      event.stopPropagation();
    }
  };

  /**
   * A subclass's first look at a key press, **before** the listbox map is consulted.
   *
   * Added by MJXOFF-187, and it exists for a difference the listbox map cannot express rather than
   * as a general escape hatch: a colour grid is two-dimensional, so it has meanings for
   * `ArrowLeft` and `ArrowRight` that a listbox correctly has none for, and those two keys are
   * also a text box's caret keys. Widening `ListboxAction` with two grid movements would have put
   * a grid's vocabulary into a list's key map, where every reader of that map would then have to
   * work out which of the twelve actions a dropdown can actually produce.
   *
   * Returning `'handled'` prevents the default and stops the propagation, exactly as the listbox
   * map's own handling does. The default is `'passed'`, so nothing changes for a subclass that
   * does not want it.
   */
  protected interceptKey(_event: KeyboardEvent): 'handled' | 'passed' {
    return 'passed';
  }

  /**
   * Do what an action says. A subclass overrides it to add its own meanings for `commit` and
   * `revert`, and calls `super` for everything it does not claim.
   *
   * Returns `'handled'` when the default should be prevented. `Tab` deliberately returns
   * `'passed'`: the browser's own sequential navigation continues from the field, which is what
   * makes this a disclosure rather than a trap.
   */
  protected handleAction(action: ListboxAction, event: KeyboardEvent): 'handled' | 'passed' {
    const surface = this.#surface;
    if (surface === undefined) return 'passed';

    switch (action) {
      case 'open':
        this.openList();
        return 'handled';
      case 'close':
        this.closeList('escape');
        return 'handled';
      case 'revert':
        return 'handled';
      case 'commit':
        if (!this.open) return 'passed';
        if (this.commitActive()) this.closeList('commit');
        else this.closeList('escape');
        return 'handled';
      case 'next':
      case 'previous':
      case 'first':
      case 'last':
      case 'pageNext':
      case 'pagePrevious': {
        const count = surface.options.length;
        if (count === 0) return 'handled';
        const from = surface.activeIndex < 0 ? -1 : surface.activeIndex;
        surface.setActive(nextOptionIndex(action, from, count));
        this.movedActive(surface.activeIndex);
        return 'handled';
      }
      case 'typeahead':
        if (!this.open) this.openList();
        this.#type(event.key);
        return 'handled';
      case 'leave':
        this.leaving();
        this.closeList('tab', { restoreFocus: false });
        return 'passed';
    }
  }

  /** A hook a subclass uses to decide what `Tab` out of the field commits. */
  protected leaving(): void {
    // Nothing.
  }

  #type(character: string): void {
    const surface = this.#surface;
    if (surface === undefined) return;
    this.#typeahead += character;
    if (this.#typeaheadTimer !== undefined) clearTimeout(this.#typeaheadTimer);
    this.#typeaheadTimer = setTimeout(() => {
      this.#typeahead = '';
      this.#typeaheadTimer = undefined;
    }, typeaheadResetDelay);
    const found = typeaheadIndex(
      surface.options.map((option) => option.label),
      this.#typeahead,
      surface.activeIndex < 0 ? -1 : surface.activeIndex - 1,
    );
    if (found >= 0) {
      surface.setActive(found);
      this.movedActive(found);
    }
  }

  // ── the pointer ────────────────────────────────────────────────────────────

  #onFieldPointerDown = (event: PointerEvent): void => {
    if (refusesActivation(this)) {
      event.preventDefault();
      return;
    }
    if (this.pointerOpensList(event)) {
      event.preventDefault();
      this.focus();
      if (this.open) this.closeList('escape');
      else this.openList();
    }
  };

  /** Whether a press on the field opens the list. A combo box only opens from its chevron. */
  protected pointerOpensList(_event: PointerEvent): boolean {
    return true;
  }

  /** The disclosure glyph, so a subclass can ask whether a press landed on it. */
  protected get disclosureElement(): HTMLElement | undefined {
    return this.#disclosure;
  }

  // ── dismissal ──────────────────────────────────────────────────────────────

  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    const document_ = this.ownerDocument;
    document_.addEventListener('pointerdown', this.#onDocumentPointerDown, true);
  }

  #stopWatching(): void {
    if (!this.#watching) return;
    this.#watching = false;
    this.ownerDocument.removeEventListener('pointerdown', this.#onDocumentPointerDown, true);
  }

  #onDocumentPointerDown = (event: Event): void => {
    if (event.composedPath().includes(this)) return;
    this.#dismissing = true;
    this.leaving();
    this.closeList('outside', { restoreFocus: false });
    queueMicrotask(() => {
      this.#dismissing = false;
    });
  };

  /**
   * The disclosure half of the focus model — and the half U06's sixth defect is about.
   *
   * A `null` `activeElement` is **a surface rebuilding under itself**, not a person leaving: the
   * list re-renders, the row focus was near is removed, focus falls to the document, and a moment
   * later it is back. Acting on that synchronously would close the list every time a person typed.
   * A microtask later the question answers itself, and the answer is read through every shadow
   * root rather than off the document.
   */
  #onFocusOut = (): void => {
    if (!this.open || this.#dismissing) return;
    queueMicrotask(() => {
      if (!this.open || this.#dismissing) return;
      const active = deepActiveElement(this.ownerDocument);
      if (active === null) return;
      if (active === this || this.contains(active) || this.#root?.contains(active) === true) return;
      this.leaving();
      this.closeList('blur', { restoreFocus: false });
    });
  };

  /** The option a value names, for a subclass. */
  protected optionFor(value: string): OptionDescriptor | undefined {
    return optionByValue(this.options, value);
  }
}

/** The focused element, followed through every shadow root it is hiding in. */
function deepActiveElement(document_: Document): Element | null {
  let element: Element | null = document_.activeElement;
  while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
  if (element === document_.body || element === document_.documentElement) return null;
  return element;
}

/** Registered by the two concrete fields, so a list field's glyph is never missing. */
export function defineListFieldDependencies(): void {
  defineIcon();
}
